//! Acceptance tests for AC-STAT-01 to AC-STAT-08 (spec:
//! game/statistics-registry, draft of 2026-11-01), written by the test writer
//! from the spec and its API sketch before the code (ADR-0014); behind the
//! feature `stats`, against the test writer's API skeleton `econ_core::stats`
//! (`indicators`, `IndicatorMeta`, `StatsHistory`, `Key`, `Series`,
//! `World::stats`, `derive_point`, `WorldParams::stats`).
//!
//! The registry and its generator do not exist yet, so the tests name no
//! variant of `IndicatorId`: they look indicators up by their dotted name in
//! `indicators()` (`id("gdp.nominal")`, which panics when the registry has no
//! such entry).
//!
//! "1:1000" is the Romania scenario at that scale, seed 42, under the law of
//! its folder (`law.json`: the 2021 law), every parameter at its spec default.
//! Month 0 is December 2021.
//!
//! Readings shared by the tests:
//! - The national value is the key `Key { breakdown: None, index: 0 }`.
//! - A breakdown's members are the keys `keys(id)` returns with that
//!   breakdown; the member `index` follows the order of the source record
//!   (regions, counties, classes, … as the month records list them).
//! - "Its feature is compiled" means `feature` is `None`, or names a feature of
//!   econ-core's `Cargo.toml` that this build turned on.
//! - The four indicators the spec marks **(new)** (`industry.wage_cost_per_unit`,
//!   `prices.cpi_contribution`, `informal.wages_declared_share` and
//!   `informal.mixed_income_declared_share`, `budget.revenue_lost`) are an open
//!   question of the spec: the tests require nothing of their values and do not
//!   require them in the registry; when present they follow the generic checks
//!   (AC-STAT-01's consistency, AC-STAT-02's twelve points, AC-STAT-03's
//!   aggregates).

use std::collections::BTreeSet;
use std::sync::OnceLock;

use econ_core::stats::{
    Aggregate, Breakdown, DeriveOp, IndicatorId, IndicatorMeta, Key, Period, Series, Status, Unit,
    View, derive_point, indicators,
};
use econ_core::world::{DayReport, World, WorldParams};
use econ_types::Month;

use super::support::{assert_day_checks, params, repo_root, start_state, step};
use super::support_lev::scenario_law;

// --- the registry -------------------------------------------------------------

/// The registry's entry named `name`; panics when there is none.
fn meta(name: &str) -> &'static IndicatorMeta {
    indicators()
        .iter()
        .find(|m| m.name == name)
        .unwrap_or_else(|| panic!("the registry has no indicator {name}"))
}

/// The id of the indicator named `name`.
fn id(name: &str) -> IndicatorId {
    meta(name).id
}

/// The national value's key.
const NATIONAL: Key = Key {
    breakdown: None,
    index: 0,
};

/// The overview's (headline) indicators of the spec's table, view 1.
const HEADLINE: [&str; 18] = [
    "gdp.nominal",
    "gdp.real",
    "gdp.per_capita",
    "gdp.growth_yoy",
    "prices.cpi",
    "prices.inflation_yoy",
    "labour.unemployment_rate",
    "labour.employed",
    "wages.mean_nominal",
    "wages.mean_real",
    "budget.balance",
    "debt.gdp_ratio",
    "money.policy_rate",
    "fx.eur_lei",
    "trade.balance",
    "approval.national",
    "people.gini",
    "people.population",
];

/// The rest of the spec's alpha list (views 3 to 8, those of the overview
/// left out), the **(new)** ones left out.
const ALPHA_REST: [&str; 43] = [
    "people.poverty_rate",
    "people.poverty_rate_anchored",
    "people.class_share",
    "people.hardship_rate",
    "people.s80_s20",
    "people.sol",
    "people.wellbeing",
    "people.life_expectancy",
    "people.births",
    "people.deaths",
    "people.emigrants",
    "people.returns",
    "people.immigrants",
    "people.diaspora",
    "people.remittances",
    "education.enrolled",
    "education.early_leavers_rate",
    "education.tertiary_attainment",
    "approval.by_group",
    "industry.output_real",
    "industry.price",
    "industry.employment",
    "industry.investment",
    "budget.revenue",
    "budget.spending",
    "budget.interest",
    "budget.deficit",
    "debt.stock",
    "budget.line_paid",
    "budget.eu_net_balance",
    "prices.inflation_mom",
    "prices.good",
    "prices.expected_inflation",
    "money.deposits_households",
    "money.credit_households",
    "money.credit_firms",
    "money.cash",
    "money.cb_foreign_assets",
    "trade.exports_volume",
    "trade.current_account",
    "trade.capital_account",
    "trade.current_account_proper",
    "fx.intervention",
];

/// The spec's day indicators (AC-STAT-05).
const DAY: [&str; 3] = ["fx.eur_lei", "money.policy_rate", "prices.good"];

/// econ-core's features, read from its `Cargo.toml` (`[features]`).
fn crate_features() -> &'static BTreeSet<String> {
    static F: OnceLock<BTreeSet<String>> = OnceLock::new();
    F.get_or_init(|| {
        let path = repo_root().join("crates/econ-core/Cargo.toml");
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut in_features = false;
        let mut out = BTreeSet::new();
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('[') {
                in_features = t == "[features]";
                continue;
            }
            if !in_features || t.starts_with('#') || t.is_empty() {
                continue;
            }
            if let Some((name, _)) = t.split_once('=') {
                let name = name.trim();
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                {
                    out.insert(name.to_string());
                }
            }
        }
        assert!(out.contains("stats"), "the features of econ-core: {out:?}");
        out
    })
}

/// Whether econ-core's feature `name` is compiled in this build. Panics on a
/// name this list does not know (add it with its `cfg!`).
fn feature_on(name: &str) -> bool {
    match name {
        "default" => true,
        "world" => cfg!(feature = "world"),
        "pay_day" => cfg!(feature = "pay_day"),
        "banking" => cfg!(feature = "banking"),
        "fiscal" => cfg!(feature = "fiscal"),
        "monetary" => cfg!(feature = "monetary"),
        "trade" => cfg!(feature = "trade"),
        "investment" => cfg!(feature = "investment"),
        "revenue" => cfg!(feature = "revenue"),
        "demography" => cfg!(feature = "demography"),
        "lifecycle" => cfg!(feature = "lifecycle"),
        "levers" => cfg!(feature = "levers"),
        "policy" => cfg!(feature = "policy"),
        "lever_policy_rate" => cfg!(feature = "lever_policy_rate"),
        "lever_monetary_financing" => cfg!(feature = "lever_monetary_financing"),
        "lever_fx_devaluation" => cfg!(feature = "lever_fx_devaluation"),
        "lever_fuel_price" => cfg!(feature = "lever_fuel_price"),
        "migration" => cfg!(feature = "migration"),
        "education" => cfg!(feature = "education"),
        "approval" => cfg!(feature = "approval"),
        "poverty" => cfg!(feature = "poverty"),
        "wellbeing" => cfg!(feature = "wellbeing"),
        "home_quality" => cfg!(feature = "home_quality"),
        "stats" => cfg!(feature = "stats"),
        other => panic!("feature {other}: not in the test's list of econ-core's features"),
    }
}

/// Whether an indicator's feature is compiled.
fn compiled(m: &IndicatorMeta) -> bool {
    m.feature.is_none_or(feature_on)
}

/// Whether an indicator is active and its feature compiled.
fn computed(m: &IndicatorMeta) -> bool {
    m.status == Status::Active && compiled(m)
}

/// Whether an indicator is a year-on-year growth rate (no point before
/// month 12).
fn is_growth_yoy(m: &IndicatorMeta) -> bool {
    matches!(m.derive, Some(econ_core::stats::Derive::GrowthYoy(_)))
}

// --- runs ---------------------------------------------------------------------

/// A world on the Romania scenario at `scale` under the law of its folder.
fn world_2021(scale: u32, p: WorldParams) -> World {
    let law = scenario_law(scale);
    World::with_law(start_state(scale), law.registry.clone(), law.law.clone(), p)
        .expect("the world builds under the scenario's law")
}

/// One day, checked.
fn step_checked(w: &mut World) -> DayReport {
    let r = step(w);
    assert_day_checks(w, &r);
    r
}

/// What a run of closes saw in the state views, day by day.
#[derive(Debug, Default)]
struct Run {
    /// Days stepped.
    days: usize,
    /// For each close, the index of the day it closed on (0-based among the
    /// days stepped).
    close_day: Vec<usize>,
    /// The exchange rate after each day, lei per euro, millionths.
    #[cfg(feature = "trade")]
    fx_day: Vec<i64>,
    /// The policy rate after each day, millionths.
    #[cfg(feature = "monetary")]
    policy_rate_day: Vec<i64>,
}

/// Step from the start until `closes` months are closed, checking every
/// day and reading the state views after each.
fn run_closes(w: &mut World, closes: usize) -> Run {
    let mut run = Run::default();
    while w.months().len() < closes {
        let before = w.months().len();
        step_checked(w);
        #[cfg(feature = "trade")]
        run.fx_day.push(i64::from(w.foreign().exchange_rate_ppm));
        #[cfg(feature = "monetary")]
        run.policy_rate_day
            .push(i64::from(w.central_bank().policy_rate_ppm));
        if w.months().len() > before {
            assert_eq!(w.months().len(), before + 1, "one close a day");
            run.close_day.push(run.days);
        }
        run.days += 1;
    }
    run
}

/// The 12-close run of AC-STAT-02, 03 and 05, at 1:1000 with the default
/// parameters.
fn year_1000() -> (World, Run) {
    let mut w = world_2021(1000, params());
    let run = run_closes(&mut w, 12);
    (w, run)
}

/// The monthly point of a series at month `t`, if it has one.
fn point(s: &Series<'_>, t: usize) -> Option<i64> {
    let first = usize::try_from(s.first_month.0).expect("a month index");
    t.checked_sub(first).and_then(|i| s.months.get(i).copied())
}

/// The national monthly points of a computed indicator, from month 0.
fn national(w: &World, name: &str) -> Vec<i64> {
    let s = w
        .stats()
        .series(id(name), NATIONAL)
        .unwrap_or_else(|| panic!("{name}: no national series"));
    assert_eq!(s.first_month, Month(0), "{name}: starts at month 0");
    s.months.to_vec()
}

/// Checks the national points of `name` against a record's values, month by
/// month. The indicator is read from the record, not derived.
fn assert_record(w: &World, name: &str, record: &[i64]) {
    let m = meta(name);
    assert!(
        m.derive.is_none(),
        "{name} is read from its record, not derived: {:?}",
        m.derive
    );
    assert_eq!(m.status, Status::Active, "{name} is active");
    assert!(
        compiled(m),
        "{name}: its feature {:?} is compiled",
        m.feature
    );
    let got = national(w, name);
    assert_eq!(got.len(), record.len(), "{name}: one point per close");
    for (t, (g, r)) in got.iter().zip(record).enumerate() {
        assert_eq!(g, r, "{name}, month {t}: the indicator against its record");
    }
}

/// Checks one breakdown member of `name` against a record's values.
#[cfg(all(feature = "home_quality", feature = "education", feature = "approval"))]
fn assert_member(w: &World, name: &str, breakdown: Breakdown, index: usize, record: &[i64]) {
    let key = Key {
        breakdown: Some(breakdown),
        index: u32::try_from(index).expect("an index"),
    };
    let s = w
        .stats()
        .series(id(name), key)
        .unwrap_or_else(|| panic!("{name} {key:?}: no series"));
    for (t, r) in record.iter().enumerate() {
        assert_eq!(
            point(&s, t),
            Some(*r),
            "{name} {key:?}, month {t}: the indicator against its record"
        );
    }
}

fn i(x: impl Into<i128>) -> i64 {
    i64::try_from(x.into()).expect("fits an i64")
}

// --- AC-STAT-01 -----------------------------------------------------------------

/// Every string value of a JSON document, recursively.
fn json_strings(v: &serde_json::Value, out: &mut BTreeSet<String>) {
    match v {
        serde_json::Value::String(s) => {
            out.insert(s.clone());
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| json_strings(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| json_strings(x, out)),
        _ => {}
    }
}

/// **AC-STAT-01** `[unit]` The registry parses; every entry has a known unit,
/// period, breakdown, aggregate and view; ids are unique; every `glossary`
/// names a glossary `code_name` (or the entry derives from one);
/// `just codegen-check` passes (the generated files are in step).
///
/// Readings: "parses" is `indicators()` returning the entries of
/// `crates/econ-core/indicators.toml`, which names each (`"<name>"` appears
/// in the file); "known" units, periods, breakdowns, aggregates and views are
/// those of the generated enums, so the test checks what the types cannot:
/// no breakdown or view listed twice; `aggregate = sum` only for counts and
/// money (`bani`, `real_bani`, `persons`, `count`: "the parts add up
/// exactly"); `feature` a feature of econ-core's `Cargo.toml`. Ids and names
/// are unique, names dotted (`[a-z][a-z0-9_]*` parts joined by `.`). An entry
/// without `derive` names a glossary `code_name` (`econ_types::glossary`); a
/// derived one names one or none, and its inputs are other entries of the
/// registry. `just codegen-check` itself runs in `just check`; here the
/// generated UI file `ui/src/generated/indicators.json` parses and holds
/// every name, and the Rust side is compiled from the generator's output. The
/// registry also holds the spec's alpha list (the four **(new)** indicators
/// left to the open question): the overview's 18 with `headline` and the
/// view `overview`, and the three day indicators with `period = day`; every
/// headline entry is in the overview.
#[test]
fn ac_stat_01() {
    let all = indicators();
    assert!(!all.is_empty(), "the registry has entries");
    let toml_path = repo_root().join("crates/econ-core/indicators.toml");
    let toml = std::fs::read_to_string(&toml_path)
        .unwrap_or_else(|e| panic!("{}: {e}", toml_path.display()));
    let json_path = repo_root().join("ui/src/generated/indicators.json");
    let json_text = std::fs::read_to_string(&json_path)
        .unwrap_or_else(|e| panic!("{}: {e}", json_path.display()));
    let json: serde_json::Value =
        serde_json::from_str(&json_text).expect("the generated indicators.json parses");
    let mut json_names = BTreeSet::new();
    json_strings(&json, &mut json_names);

    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for m in all {
        let n = m.name;
        assert!(ids.insert(m.id), "{n}: id {:?} listed twice", m.id);
        assert!(names.insert(n), "{n}: name listed twice");
        let dotted = n.split('.').count() >= 2
            && n.split('.').all(|p| {
                p.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                    && p.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            });
        assert!(dotted, "{n}: a dotted name");
        assert!(
            toml.contains(&format!("\"{n}\"")),
            "{n}: an entry of indicators.toml"
        );
        assert!(
            json_names.contains(n),
            "{n}: in the generated indicators.json"
        );
        let in_glossary = econ_types::glossary::lookup(m.glossary).is_some();
        if m.derive.is_none() {
            assert!(
                in_glossary,
                "{n}: glossary {:?} is not a glossary code_name",
                m.glossary
            );
        } else {
            assert!(
                m.glossary.is_empty() || in_glossary,
                "{n} (derived): glossary {:?} is neither empty nor a code_name",
                m.glossary
            );
        }
        let distinct_b: BTreeSet<Breakdown> = m.breakdowns.iter().copied().collect();
        assert_eq!(
            distinct_b.len(),
            m.breakdowns.len(),
            "{n}: a breakdown listed twice: {:?}",
            m.breakdowns
        );
        let distinct_v: BTreeSet<View> = m.views.iter().copied().collect();
        assert_eq!(
            distinct_v.len(),
            m.views.len(),
            "{n}: a view listed twice: {:?}",
            m.views
        );
        if m.aggregate == Aggregate::Sum {
            assert!(
                matches!(
                    m.unit,
                    Unit::Bani | Unit::RealBani | Unit::Persons | Unit::Count
                ),
                "{n}: aggregate sum over {:?}, not a count or money",
                m.unit
            );
        }
        if let Some(f) = m.feature {
            assert!(
                crate_features().contains(f),
                "{n}: feature {f} is not a feature of econ-core"
            );
        }
        if m.headline {
            assert!(
                m.views.contains(&View::Overview),
                "{n}: a headline indicator in the overview"
            );
        }
    }
    // Derived entries read other entries.
    for m in all {
        let inputs: Vec<IndicatorId> = match m.derive {
            None => Vec::new(),
            Some(econ_core::stats::Derive::Ratio(a, b)) => vec![a, b],
            Some(
                econ_core::stats::Derive::Real(a)
                | econ_core::stats::Derive::GrowthYoy(a)
                | econ_core::stats::Derive::GrowthMom(a)
                | econ_core::stats::Derive::PerCapita(a),
            ) => vec![a],
        };
        for a in inputs {
            assert!(
                ids.contains(&a),
                "{}: input {a:?} is in the registry",
                m.name
            );
            assert_ne!(a, m.id, "{}: derived from itself", m.name);
        }
    }
    // The spec's alpha list.
    for n in HEADLINE {
        let m = meta(n);
        assert!(m.headline, "{n}: headline");
        assert!(m.views.contains(&View::Overview), "{n}: in the overview");
    }
    for n in ALPHA_REST {
        assert_eq!(meta(n).name, n, "{n}: in the registry");
    }
    for n in DAY {
        assert_eq!(meta(n).period, Period::Day, "{n}: a day indicator");
    }
}

// --- AC-STAT-02 -----------------------------------------------------------------

/// **AC-STAT-02** `[sim]` At 1:1000 under the 2021 law with the default
/// features, after 12 closes, every active indicator whose feature is
/// compiled has exactly 12 monthly national points, and each indicator read
/// from a month record equals that record's value for every month (e.g.
/// `gdp.nominal` = `months()[t].gdp_production`, `budget.deficit` =
/// `fiscal_months()[t].deficit`, `trade.current_account` =
/// `trade_months()[t].current_account`); an indicator whose feature is not
/// compiled has no series.
///
/// Readings: the run is the default parameters (`WorldParams::stats` on)
/// stepped from the start to the 12th close (November 2022's), checking
/// every day. "12 monthly national points" is the national series's
/// `months` of length 12 from month 0; a year-on-year growth rate (`derive =
/// growth_yoy`) has none before month 12, so after 12 closes it has no
/// point (`None` or an empty series). "No series" for a feature not compiled
/// is `series(id, national)` `None` and no key. The indicators read from a
/// record, at each close `t`, and checked here (each also not derived):
/// - `months()[t]`: `gdp.nominal` = `gdp_production`, `prices.cpi` =
///   `cpi_ppm`, `prices.inflation_yoy` = `inflation_ppm` (the record's
///   twelve-month inflation), `prices.expected_inflation` =
///   `expected_inflation_ppm`, `labour.employed` = `employed`;
/// - `trade_months()[t]`: `trade.current_account` = `current_account`,
///   `trade.capital_account` = `capital_account`,
///   `trade.current_account_proper` = `current_account − capital_account`,
///   `trade.exports_volume` = `exports_volume`, `fx.intervention` =
///   `intervention`;
/// - `fiscal_months()[t]`: `budget.deficit` = `deficit`, `budget.interest`
///   = `interest`, `budget.revenue` = the sum of `revenue`, `budget.spending`
///   = the sum of `spending` (interest excluded, as the record);
/// - `budget_months()`: `budget.line_paid` (national) = the sum over the
///   month's lines of `wages + inputs + investment` (what the lines paid in
///   the month, overrun included in `wages`);
/// - the state views at the close: `fx.eur_lei` =
///   `foreign().exchange_rate_ppm` and `money.policy_rate` =
///   `central_bank().policy_rate_ppm` after the close's day.
#[test]
fn ac_stat_02() {
    let (w, run) = year_1000();
    assert_eq!(w.months().len(), 12);
    let stats = w.stats();
    for m in indicators() {
        if !compiled(m) {
            assert!(
                stats.series(m.id, NATIONAL).is_none(),
                "{}: feature {:?} not compiled, yet a series",
                m.name,
                m.feature
            );
            assert!(
                stats.keys(m.id).is_empty(),
                "{}: feature {:?} not compiled, yet keys {:?}",
                m.name,
                m.feature,
                stats.keys(m.id)
            );
            continue;
        }
        if m.status != Status::Active {
            continue;
        }
        let s = stats.series(m.id, NATIONAL);
        if is_growth_yoy(m) {
            assert_eq!(
                s.map_or(0, |s| s.months.len()),
                0,
                "{}: a year-on-year rate has no point before month 12",
                m.name
            );
            continue;
        }
        let s = s.unwrap_or_else(|| panic!("{}: no national series", m.name));
        assert_eq!(s.first_month, Month(0), "{}: from month 0", m.name);
        assert_eq!(s.months.len(), 12, "{}: 12 monthly points", m.name);
        assert_eq!(
            stats.latest(m.id, NATIONAL),
            s.months.last().copied(),
            "{}: latest is the last point",
            m.name
        );
    }

    let months = w.months();
    let col = |f: &dyn Fn(&econ_core::world::MonthStats) -> i64| -> Vec<i64> {
        months.iter().map(f).collect()
    };
    assert_record(&w, "gdp.nominal", &col(&|m| m.gdp_production.get()));
    assert_record(&w, "prices.cpi", &col(&|m| i64::from(m.cpi_ppm)));
    assert_record(
        &w,
        "prices.inflation_yoy",
        &col(&|m| i64::from(m.inflation_ppm)),
    );
    assert_record(
        &w,
        "prices.expected_inflation",
        &col(&|m| i64::from(m.expected_inflation_ppm)),
    );
    assert_record(&w, "labour.employed", &col(&|m| i(m.employed)));

    #[cfg(feature = "trade")]
    {
        let tm = w.trade_months();
        assert_eq!(tm.len(), 12, "a trade month per close");
        let col = |f: &dyn Fn(&econ_core::world::TradeMonth) -> i64| -> Vec<i64> {
            tm.iter().map(f).collect()
        };
        assert_record(
            &w,
            "trade.current_account",
            &col(&|m| m.current_account.get()),
        );
        assert_record(
            &w,
            "trade.capital_account",
            &col(&|m| m.capital_account.get()),
        );
        assert_record(
            &w,
            "trade.current_account_proper",
            &col(&|m| m.current_account.get() - m.capital_account.get()),
        );
        assert_record(&w, "trade.exports_volume", &col(&|m| m.exports_volume));
        assert_record(&w, "fx.intervention", &col(&|m| m.intervention.get()));
        let fx: Vec<i64> = run.close_day.iter().map(|&d| run.fx_day[d]).collect();
        assert_record(&w, "fx.eur_lei", &fx);
    }
    #[cfg(feature = "monetary")]
    {
        let rate: Vec<i64> = run
            .close_day
            .iter()
            .map(|&d| run.policy_rate_day[d])
            .collect();
        assert_record(&w, "money.policy_rate", &rate);
    }
    #[cfg(feature = "fiscal")]
    {
        let fm = w.fiscal_months();
        assert_eq!(fm.len(), 12, "a fiscal month per close");
        let sum = |v: &[(econ_ledger::FlowCode, econ_types::Bani)]| -> i64 {
            i(v.iter().map(|(_, b)| i128::from(b.get())).sum::<i128>())
        };
        let col = |f: &dyn Fn(&econ_core::world::FiscalMonth) -> i64| -> Vec<i64> {
            fm.iter().map(f).collect()
        };
        assert_record(&w, "budget.deficit", &col(&|m| m.deficit.get()));
        assert_record(&w, "budget.interest", &col(&|m| m.interest.get()));
        assert_record(&w, "budget.revenue", &col(&|m| sum(&m.revenue)));
        assert_record(&w, "budget.spending", &col(&|m| sum(&m.spending)));
    }
    {
        let lines = w.budget_months();
        assert!(!lines.is_empty(), "the budget lines report");
        let paid: Vec<i64> = months
            .iter()
            .map(|ms| {
                i(lines
                    .iter()
                    .filter(|l| l.month == ms.month)
                    .map(|l| {
                        i128::from(l.wages.get())
                            + i128::from(l.inputs.get())
                            + i128::from(l.investment.get())
                    })
                    .sum::<i128>())
            })
            .collect();
        assert_record(&w, "budget.line_paid", &paid);
    }
}

// --- AC-STAT-03 -----------------------------------------------------------------

/// **AC-STAT-03** `[sim]` Breakdowns: at every close of AC-STAT-02's run, for
/// indicators with `aggregate = sum` the breakdown's values add up to the
/// national value exactly; with `aggregate = mean`, the national value is the
/// population-weighted mean of the breakdown's values to within one unit of
/// the last place per breakdown member.
///
/// Readings: every active indicator whose feature is compiled, every
/// breakdown it has keys for (`keys(id)`), every close `t` of the 12; a
/// member without a point at `t` is an error. For `sum`, Σ members =
/// national exactly. For `mean`, "the breakdown's population" is the
/// registry's `people.population` at the same key: where it has every member
/// of the breakdown, |national − Σ pop·v / Σ pop| ≤ n units (n the members),
/// checked in integers as |national·Σ pop − Σ pop·v| ≤ n·Σ pop; where it does
/// not (a breakdown `people.population` lacks, such as industry), the
/// population weights are not observable through the registry, and the test
/// checks what any weighted mean satisfies: min v − n ≤ national ≤ max v +
/// n.
#[test]
fn ac_stat_03() {
    let (w, _) = year_1000();
    let stats = w.stats();
    let pop = meta("people.population");
    let mut checked = 0usize;
    for m in indicators() {
        if !computed(m) || m.aggregate == Aggregate::None || is_growth_yoy(m) {
            continue;
        }
        let keys = stats.keys(m.id);
        let breakdowns: BTreeSet<Breakdown> = keys.iter().filter_map(|k| k.breakdown).collect();
        let nat = stats
            .series(m.id, NATIONAL)
            .unwrap_or_else(|| panic!("{}: no national series", m.name));
        for b in breakdowns {
            let members: Vec<Key> = keys
                .iter()
                .copied()
                .filter(|k| k.breakdown == Some(b))
                .collect();
            let n = i128::try_from(members.len()).expect("a count");
            let series: Vec<Series<'_>> = members
                .iter()
                .map(|&k| {
                    stats
                        .series(m.id, k)
                        .unwrap_or_else(|| panic!("{} {k:?}: a key without a series", m.name))
                })
                .collect();
            let weights: Option<Vec<Series<'_>>> = if computed(pop) {
                members.iter().map(|&k| stats.series(pop.id, k)).collect()
            } else {
                None
            };
            for t in 0..12 {
                let Some(national) = point(&nat, t) else {
                    continue;
                };
                let national = i128::from(national);
                let v: Vec<i128> =
                    series
                        .iter()
                        .zip(&members)
                        .map(|(s, k)| {
                            i128::from(point(s, t).unwrap_or_else(|| {
                                panic!("{} {k:?}: no point at month {t}", m.name)
                            }))
                        })
                        .collect();
                match m.aggregate {
                    Aggregate::Sum => {
                        let total: i128 = v.iter().sum();
                        assert_eq!(
                            total, national,
                            "{} by {b:?}, month {t}: the parts add up to the national value",
                            m.name
                        );
                    }
                    Aggregate::Mean => {
                        let pw: Option<Vec<i128>> = weights.as_ref().map(|ws| {
                            ws.iter()
                                .map(|s| i128::from(point(s, t).unwrap_or(0)))
                                .collect()
                        });
                        match pw {
                            Some(pw) if pw.iter().sum::<i128>() > 0 => {
                                let total_w: i128 = pw.iter().sum();
                                let wv: i128 = pw.iter().zip(&v).map(|(p, x)| p * x).sum();
                                assert!(
                                    (national * total_w - wv).abs() <= n * total_w,
                                    "{} by {b:?}, month {t}: national {national} against the population-weighted mean {} of {n} members",
                                    m.name,
                                    wv / total_w
                                );
                            }
                            _ => {
                                let lo = v.iter().min().copied().unwrap_or(national);
                                let hi = v.iter().max().copied().unwrap_or(national);
                                assert!(
                                    lo - n <= national && national <= hi + n,
                                    "{} by {b:?}, month {t}: national {national} outside its members' {lo}..{hi}",
                                    m.name
                                );
                            }
                        }
                    }
                    Aggregate::None => {}
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "some indicator has a breakdown to check");
}

// --- AC-STAT-04 -----------------------------------------------------------------

/// **AC-STAT-04** `[unit]` Derived indicators follow their formulas on given
/// inputs: `ratio`, `real`, `growth_yoy` (absent before month 12),
/// `growth_mom`, `per_capita`, in integers rounded half away from zero.
///
/// Readings: through `derive_point(op, a, b, t)` (the test writer's
/// addition): `ratio` = a / b in millionths; `real` = a × 1,000,000 / CPI
/// (the CPI in millionths); `growth_yoy` = a_t / a_{t−12} − 1 in millionths,
/// none before month 12; `growth_mom` = a_t / a_{t−1} − 1, none at month 0;
/// `per_capita` = a / persons (the unit of a per person). "Rounded half away
/// from zero" applies to the value itself: a growth rate is the rounding of
/// (a_t − a_before) × 1,000,000 / a_before, so −0.5 millionths is −1. A zero
/// denominator has no value.
#[test]
fn ac_stat_04() {
    let d = |op, a: &[i64], b: &[i64], t| derive_point(op, a, b, t);
    // ratio, millionths.
    assert_eq!(d(DeriveOp::Ratio, &[1], &[3], 0), Some(333_333));
    assert_eq!(d(DeriveOp::Ratio, &[2], &[3], 0), Some(666_667));
    assert_eq!(d(DeriveOp::Ratio, &[1], &[2_000_000], 0), Some(1), "0.5 up");
    assert_eq!(
        d(DeriveOp::Ratio, &[-1], &[2_000_000], 0),
        Some(-1),
        "−0.5 down"
    );
    assert_eq!(d(DeriveOp::Ratio, &[-7], &[4], 0), Some(-1_750_000));
    assert_eq!(
        d(DeriveOp::Ratio, &[5, 9], &[10, 4], 1),
        Some(2_250_000),
        "month 1's inputs"
    );
    assert_eq!(d(DeriveOp::Ratio, &[5], &[0], 0), None, "zero denominator");
    // real: value × 1,000,000 / CPI.
    assert_eq!(
        d(DeriveOp::Real, &[1_000_000_000], &[1_100_000], 0),
        Some(909_090_909)
    );
    assert_eq!(d(DeriveOp::Real, &[3], &[2_000_000], 0), Some(2), "1.5 up");
    assert_eq!(
        d(DeriveOp::Real, &[-3], &[2_000_000], 0),
        Some(-2),
        "−1.5 down"
    );
    assert_eq!(
        d(DeriveOp::Real, &[100, 500], &[1_000_000, 1_250_000], 1),
        Some(400)
    );
    // growth_yoy: none before month 12.
    let mut a: Vec<i64> = (0..13).map(|k| 200 + k).collect();
    a[12] = 230;
    for t in 0..12 {
        assert_eq!(d(DeriveOp::GrowthYoy, &a, &[], t), None, "month {t}");
    }
    assert_eq!(
        d(DeriveOp::GrowthYoy, &a, &[], 12),
        Some(150_000),
        "230 / 200 − 1"
    );
    let mut a = vec![3; 13];
    a[12] = 4;
    assert_eq!(d(DeriveOp::GrowthYoy, &a, &[], 12), Some(333_333));
    let mut a = vec![2_000_000; 13];
    a[12] = 2_000_001;
    assert_eq!(d(DeriveOp::GrowthYoy, &a, &[], 12), Some(1), "+0.5 up");
    a[12] = 1_999_999;
    assert_eq!(d(DeriveOp::GrowthYoy, &a, &[], 12), Some(-1), "−0.5 down");
    a[12] = 1_999_997;
    assert_eq!(d(DeriveOp::GrowthYoy, &a, &[], 12), Some(-2), "−1.5 down");
    let mut a = vec![0; 13];
    a[12] = 5;
    assert_eq!(d(DeriveOp::GrowthYoy, &a, &[], 12), None, "from zero");
    // growth_mom: none at month 0.
    assert_eq!(d(DeriveOp::GrowthMom, &[400, 300], &[], 0), None);
    assert_eq!(d(DeriveOp::GrowthMom, &[400, 300], &[], 1), Some(-250_000));
    assert_eq!(d(DeriveOp::GrowthMom, &[3, 4, 5], &[], 2), Some(250_000));
    assert_eq!(
        d(DeriveOp::GrowthMom, &[2_000_000, 2_000_003], &[], 1),
        Some(2),
        "1.5 up"
    );
    // per_capita: a / persons.
    assert_eq!(
        d(DeriveOp::PerCapita, &[10_000_000], &[3], 0),
        Some(3_333_333)
    );
    assert_eq!(d(DeriveOp::PerCapita, &[5], &[2], 0), Some(3), "2.5 up");
    assert_eq!(
        d(DeriveOp::PerCapita, &[-5], &[2], 0),
        Some(-3),
        "−2.5 down"
    );
    assert_eq!(d(DeriveOp::PerCapita, &[5], &[0], 0), None, "nobody");
}

// --- AC-STAT-05 -----------------------------------------------------------------

/// **AC-STAT-05** `[sim]` Day indicators (`fx.eur_lei`, `money.policy_rate`,
/// `prices.good`) have one point per day stepped, the last day's point of
/// each month equal to the month's point; month indicators have none.
///
/// Readings: AC-STAT-02's run (12 closes from the start, at 1:1000). For
/// each active, compiled indicator with `period = day` and each of its keys:
/// `days` holds one point per day stepped (the run's count), and at each
/// close `t` the point of the close's day equals the month's point `t`. The
/// three named indicators are such (AC-STAT-01 checks their period), and
/// `fx.eur_lei` and `money.policy_rate` each day equal the state view after
/// that day (`foreign().exchange_rate_ppm`, `central_bank().policy_rate_ppm`).
/// "Month indicators have none": every key of every `period = month`
/// indicator has `days` `None`.
#[test]
fn ac_stat_05() {
    let (w, run) = year_1000();
    let stats = w.stats();
    assert_eq!(run.close_day.len(), 12);
    let mut day_series = 0usize;
    for m in indicators() {
        if !computed(m) {
            continue;
        }
        for &k in stats.keys(m.id) {
            let s = stats
                .series(m.id, k)
                .unwrap_or_else(|| panic!("{} {k:?}: a key without a series", m.name));
            match m.period {
                Period::Month => assert!(
                    s.days.is_none(),
                    "{} {k:?}: a month indicator with daily points",
                    m.name
                ),
                Period::Day => {
                    let days = s
                        .days
                        .unwrap_or_else(|| panic!("{} {k:?}: no daily points", m.name));
                    assert_eq!(
                        days.len(),
                        run.days,
                        "{} {k:?}: one point per day stepped",
                        m.name
                    );
                    for (t, &d) in run.close_day.iter().enumerate() {
                        assert_eq!(
                            Some(days[d]),
                            point(&s, t),
                            "{} {k:?}: the close of month {t} (day {d})",
                            m.name
                        );
                    }
                    day_series += 1;
                }
            }
        }
    }
    for n in DAY {
        let m = meta(n);
        if computed(m) {
            assert!(
                !stats.keys(m.id).is_empty(),
                "{n}: a computed day indicator with no key"
            );
        }
    }
    assert!(day_series > 0, "some day indicator is computed");
    let day_points = |name: &str| -> Vec<i64> {
        stats
            .series(id(name), NATIONAL)
            .and_then(|s| s.days)
            .unwrap_or_else(|| panic!("{name}: no national daily points"))
            .to_vec()
    };
    #[cfg(feature = "trade")]
    assert_eq!(day_points("fx.eur_lei"), run.fx_day, "fx.eur_lei: each day");
    #[cfg(feature = "monetary")]
    assert_eq!(
        day_points("money.policy_rate"),
        run.policy_rate_day,
        "money.policy_rate: each day"
    );
    let _ = day_points;
}

// --- AC-STAT-06 -----------------------------------------------------------------

/// **AC-STAT-06** `[sim]` Isolation: at 1:1000, the state hash after 24
/// months is the same with `stats` compiled and not.
///
/// Readings: both builds cannot run in one test, so, as AC-POV-07 and
/// AC-WB-04 do, "not compiled" is the history turned off at run time:
/// `WorldParams::stats` `false` (the test writer's addition), against `true`.
/// The test compares the state hash of every day through the 24th close,
/// which includes the one after 24 months, and checks that the history was
/// filled in one run (24 points of `gdp.nominal`) and not in the other.
#[test]
fn ac_stat_06() {
    let on = params();
    assert!(on.stats, "the history is on by default");
    let mut off = params();
    off.stats = false;
    let mut a = world_2021(1000, on);
    let mut b = world_2021(1000, off);
    while a.months().len() < 24 {
        let ra = step_checked(&mut a);
        let rb = step_checked(&mut b);
        assert_eq!(
            ra.state_hash, rb.state_hash,
            "{:?}: the state hashes differ",
            ra.date
        );
    }
    assert_eq!(b.months().len(), 24);
    assert_eq!(a.state_hash(), b.state_hash(), "after 24 months");
    let gdp = id("gdp.nominal");
    assert_eq!(
        a.stats().series(gdp, NATIONAL).map(|s| s.months.len()),
        Some(24),
        "the history was filled"
    );
    assert!(
        b.stats().series(gdp, NATIONAL).is_none(),
        "the history was off"
    );
}

// --- AC-STAT-07 -----------------------------------------------------------------

/// **AC-STAT-07** `[sim]` With the M5 features compiled (`home_quality`,
/// `education`, `approval`), the people indicators exist and equal their
/// records (`people.poverty_rate` =
/// `poverty_stats()[t].national.poverty_rate_ppm`, `people.sol` =
/// `poverty_stats()[t].sol.national_milli`, `people.wellbeing` =
/// `wellbeing_stats()[t].national.index_ppm`, `people.emigrants` =
/// `migration_months()[t].emigrants`, `approval.national` = the approval
/// stats' national mean).
///
/// Readings: 12 closes from the start at 1:1000, default parameters. "The
/// people indicators exist": every active registry entry in the view
/// `people` whose feature is compiled has 12 national points (a
/// year-on-year rate none). The records, at each close `t`, checked here:
/// - `poverty_stats()[t]`: `people.poverty_rate` =
///   `national.poverty_rate_ppm`; `people.poverty_rate_anchored` =
///   `class_share_ppm[0]` (the share of the class at risk, as the spec of
///   poverty defines it); `people.hardship_rate` = `hardship_rate_ppm`;
///   `people.class_share` class `k` = `class_share_ppm[k]`; `people.sol` =
///   `sol.national_milli`, region `r` = `sol.by_region[r]`, county `c` =
///   `sol.by_county[c]`, class `k` = `sol.by_class[k]`, fifth `q` =
///   `sol.by_fifth[q]`, age band `a` = `sol.by_age[a]`;
/// - `wellbeing_stats()[t]`: `people.wellbeing` = `national.index_ppm`,
///   region `r` = `by_region[r].index_ppm`; `people.life_expectancy` =
///   `life_expectancy_milli`;
/// - `migration_months()[t]`: `people.emigrants` = `emigrants`,
///   `people.returns` = `returns`, `people.immigrants` = `immigrants`;
/// - `approval_stats()[t]`: `approval.national` = `national`'s `mean_milli`
///   (every close has adults).
#[test]
#[cfg(all(feature = "home_quality", feature = "education", feature = "approval"))]
fn ac_stat_07() {
    let mut w = world_2021(1000, params());
    let _ = run_closes(&mut w, 12);
    let stats = w.stats();
    for m in indicators() {
        if !computed(m) || !m.views.contains(&View::People) {
            continue;
        }
        let s = stats.series(m.id, NATIONAL);
        if is_growth_yoy(m) {
            continue;
        }
        let s = s.unwrap_or_else(|| panic!("{}: a people indicator with no series", m.name));
        assert_eq!(s.months.len(), 12, "{}: 12 monthly points", m.name);
    }

    let ps = w.poverty_stats();
    assert_eq!(ps.len(), 12, "poverty indicators at every close");
    let col = |f: &dyn Fn(&econ_core::poverty::PovertyStats) -> i64| -> Vec<i64> {
        ps.iter().map(f).collect()
    };
    assert_record(
        &w,
        "people.poverty_rate",
        &col(&|s| i64::from(s.national.poverty_rate_ppm)),
    );
    assert_record(
        &w,
        "people.poverty_rate_anchored",
        &col(&|s| i64::from(s.class_share_ppm[0])),
    );
    assert_record(
        &w,
        "people.hardship_rate",
        &col(&|s| i64::from(s.hardship_rate_ppm)),
    );
    for k in 0..4 {
        assert_member(
            &w,
            "people.class_share",
            Breakdown::Class,
            k,
            &col(&|s| i64::from(s.class_share_ppm[k])),
        );
    }
    assert_record(&w, "people.sol", &col(&|s| i64::from(s.sol.national_milli)));
    for r in 0..ps[0].sol.by_region.len() {
        assert_member(
            &w,
            "people.sol",
            Breakdown::Region,
            r,
            &col(&|s| i64::from(s.sol.by_region[r])),
        );
    }
    for c in 0..ps[0].sol.by_county.len() {
        assert_member(
            &w,
            "people.sol",
            Breakdown::County,
            c,
            &col(&|s| i64::from(s.sol.by_county[c])),
        );
    }
    for k in 0..4 {
        assert_member(
            &w,
            "people.sol",
            Breakdown::Class,
            k,
            &col(&|s| i64::from(s.sol.by_class[k])),
        );
    }
    for q in 0..5 {
        assert_member(
            &w,
            "people.sol",
            Breakdown::Fifth,
            q,
            &col(&|s| i64::from(s.sol.by_fifth[q])),
        );
    }
    for a in 0..3 {
        assert_member(
            &w,
            "people.sol",
            Breakdown::AgeBand,
            a,
            &col(&|s| i64::from(s.sol.by_age[a])),
        );
    }

    let ws = w.wellbeing_stats();
    assert_eq!(ws.len(), 12, "the wellbeing index at every close");
    let col = |f: &dyn Fn(&econ_core::wellbeing::WellbeingStats) -> i64| -> Vec<i64> {
        ws.iter().map(f).collect()
    };
    assert_record(
        &w,
        "people.wellbeing",
        &col(&|s| i64::from(s.national.index_ppm)),
    );
    for r in 0..ws[0].by_region.len() {
        assert_member(
            &w,
            "people.wellbeing",
            Breakdown::Region,
            r,
            &col(&|s| i64::from(s.by_region[r].index_ppm)),
        );
    }
    assert_record(
        &w,
        "people.life_expectancy",
        &col(&|s| i64::from(s.life_expectancy_milli)),
    );

    let mm = w.migration_months();
    assert_eq!(mm.len(), 12, "a migration month per close");
    let col = |f: &dyn Fn(&econ_core::world::MigrationMonth) -> i64| -> Vec<i64> {
        mm.iter().map(f).collect()
    };
    assert_record(&w, "people.emigrants", &col(&|m| i(m.emigrants)));
    assert_record(&w, "people.returns", &col(&|m| i(m.returns)));
    assert_record(&w, "people.immigrants", &col(&|m| i(m.immigrants)));

    let ap = w.approval_stats();
    assert_eq!(ap.len(), 12, "approval at every close");
    let national: Vec<i64> = ap
        .iter()
        .enumerate()
        .map(|(t, s)| {
            i64::from(
                s.national
                    .as_ref()
                    .unwrap_or_else(|| panic!("month {t}: adults approve"))
                    .mean_milli,
            )
        })
        .collect();
    assert_record(&w, "approval.national", &national);
}

// --- AC-STAT-08 -----------------------------------------------------------------

/// A simulated year at 1:100 under the 2021 law, the history on or off; the
/// time of the 365 days (the build excluded), milliseconds. Checks that the
/// history was filled when on (12 points of `gdp.nominal`) and not when off,
/// so that the comparison times the registry's work.
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn year_millis(stats_on: bool) -> u128 {
    let mut p = params();
    p.stats = stats_on;
    let mut w = world_2021(100, p);
    let started = std::time::Instant::now();
    for _ in 0..365 {
        step(&mut w);
    }
    let took = started.elapsed().as_millis();
    assert_eq!(w.months().len(), 12, "every close ran");
    let filled = w
        .stats()
        .series(id("gdp.nominal"), NATIONAL)
        .map(|s| s.months.len());
    assert_eq!(
        filled,
        stats_on.then_some(12),
        "the history was filled with the registry on, and only then"
    );
    took
}

/// **AC-STAT-08** `[bench]` The registry adds at most 10% to a simulated year
/// at 1:100 in a release build.
///
/// Readings: the year is 365 days from the start of the Romania scenario at
/// 1:100 under the 2021 law (the build of the world excluded), with
/// `WorldParams::stats` on against off (the same build, as AC-STAT-06). Each
/// is timed twice, alternating (off, on, off, on), and the faster of each
/// pair is compared, so that one slow moment of the machine does not decide:
/// on ≤ 110% of off. A time budget means nothing in a debug build: the test
/// runs in release builds only (`cargo test --release -p econ-core
/// --features stats --test acceptance stats::`).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build time budget: cargo test --release -p econ-core --features stats --test acceptance stats::"
)]
fn ac_stat_08() {
    let mut off = u128::MAX;
    let mut on = u128::MAX;
    for _ in 0..2 {
        off = off.min(year_millis(false));
        on = on.min(year_millis(true));
    }
    assert!(
        on * 10 <= off * 11,
        "a year at 1:100 took {on} ms with the registry, {off} ms without (more than 10% more)"
    );
}
