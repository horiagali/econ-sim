//! The harness of the policy-scenario suite (spec game/policy-scenarios,
//! "How a scenario is run", "Measurements" and "API sketch").
//!
//! A run is the Romania scenario at a scale, under the law of its folder
//! (`law.json` and `world.json`), spec defaults, seed 42, from 1 December
//! 2021 through 28 February 2025, with a commands file given on its days.
//! Every day of every run keeps the checks of phase 13 and the tests' own
//! reading of them (`assert_day_checks`: I-1 to I-8 and the sector columns)
//! and person conservation (the weighted count of persons never changes; no
//! demography runs in M4). At each monthly close the run records what the
//! spec's indicators read.
//!
//! Runs are cached per scale and commands file, so the baseline and the
//! placebo of a scale are run once per test binary and shared by every
//! scenario, and a scenario run is shared by its own test and AC-POL-10/11.
//! A run that panics (a failed check) or whose commands are rejected is
//! cached as an error and fails every test that needs it, with its message.

// Each test file uses a part of this one.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use econ_core::levers::{Command, Order, Registry};
use econ_core::world::{FiscalMonth, MonthStats, StartState, TradeMonth, World};
use econ_io::commands::{ScenarioLaw, read_commands, read_scenario_law};
use econ_io::read_start_state;
use econ_scenario::write;
use econ_types::{Bani, Date};

use crate::support::{
    assert_day_checks, build_scenario, fresh_dir, inputs, io_table_path, params, person_weights,
    start_state, step,
};
use crate::support_lev::{registry, scenario_law};

/// One, in millionths: bands and measurements are in millionths of their
/// unit (a percent, a percentage point, a percent of GDP).
pub const ONE: i128 = 1_000_000;
/// A ratio of one, in millionths of a percent.
const HUNDRED: i128 = 100 * ONE;

/// The last day of every run (spec, "Run length").
pub fn last_day() -> Date {
    Date::new(2025, 2, 28).expect("a date")
}

/// The close of month 1 (March 2022, the month of T0).
pub fn close_of_month_one() -> Date {
    Date::new(2022, 3, 31).expect("a date")
}

/// Monthly closes in a run: December 2021 to February 2025.
pub const MONTHS_IN_RUN: usize = 39;
/// Months after T0 in a run.
pub const MONTHS_AFTER_T0: u32 = 36;

// --- the spec's types ---------------------------------------------------------

/// How a difference against the baseline is measured (spec, "Measurements").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Mean over the window of the monthly percent differences.
    LevelPct,
    /// Percent difference of the window's sums.
    FlowPct,
    /// Mean over the window of the monthly differences, percentage points.
    Points,
    /// The window's summed difference over the baseline's summed nominal GDP,
    /// percent.
    PctOfGdp,
}

/// An indicator (spec, "Measurements", table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ind {
    /// `price_level`: `MonthStats::cpi_ppm`.
    PriceLevel,
    /// `inflation`: `MonthStats::inflation_ppm`, in points.
    Inflation,
    /// `inflation_expectation`: `MonthStats::expected_inflation_ppm`, in points.
    ExpectedInflation,
    /// `unemployment_rate`: unemployed / (employed + unemployed), in points.
    Unemployment,
    /// `wage_nominal`: the mean gross wage of employees at the close (see
    /// [`Monthly::employee_wages`]).
    MeanWage,
    /// `consumption`: `MonthStats::consumption` (VAT included).
    Consumption,
    /// `consumption / price_level` (S3's watched real consumption).
    RealConsumption,
    /// `gdp_real`: `gdp_expenditure / price_level`.
    RealGdp,
    /// `exchange_rate`: `World::foreign().exchange_rate_ppm` at the close.
    ExchangeRate,
    /// `TradeMonth::exports_volume`.
    ExportVolume,
    /// `TradeMonth::current_account`.
    CurrentAccount,
    /// `FiscalMonth::deficit`.
    Deficit,
    /// A tax or benefit: `FiscalMonth::revenue` or `spending` under the code.
    Flow(&'static str),
    /// `cb_credibility`: `World::central_bank().credibility_ppm` at the
    /// close, ×100, in points.
    Credibility,
    /// `employment`: `MonthStats::employed`.
    Employment,
    /// Households in hardship: `MonthStats::hardship_households`.
    Hardship,
    /// The price of a catalogue industry: `World::industries().price_ppm` at
    /// the close.
    Price(&'static str),
    /// The import price index: `World::foreign().import_price_ppm` at the
    /// close. Reading: the World shows no imports by industry, so the index
    /// is the plain mean over industries (watched only, S6; a step of the
    /// rate moves every import price by the same factor, so the weighting
    /// barely matters there).
    ImportPriceIndex,
}

/// One expectation of a scenario's table.
#[derive(Debug, Clone, Copy)]
pub struct Check {
    /// The spec's words, for messages.
    pub what: &'static str,
    /// The indicator.
    pub ind: Ind,
    /// How the difference is measured.
    pub kind: Kind,
    /// Months `a` to `b` after T0, both included (month 1 is March 2022).
    pub months: (u32, u32),
    /// The band `[lo, hi]` in millionths of the kind's unit, inclusive.
    pub band_ppm: (i64, i64),
    /// Checked (fails the test) or watched (printed).
    pub checked: bool,
}

/// A scenario: its id, its commands file under `policy/commands/` and its
/// expectations.
#[derive(Debug)]
pub struct Scenario {
    /// `S1` to `S9`.
    pub id: &'static str,
    /// The criterion, for messages.
    pub ac: &'static str,
    /// The commands file, under `policy/commands/`.
    pub commands: &'static str,
    /// The expectations.
    pub checks: &'static [Check],
}

/// A band edge of percent (or points) given in hundredths: `pct(-1000)` is
/// −10, `pct(-10)` is −0.1.
pub const fn pct(hundredths: i64) -> i64 {
    hundredths * 10_000
}

/// A checked expectation; band edges in hundredths of the unit.
pub const fn checked(
    what: &'static str,
    ind: Ind,
    kind: Kind,
    months: (u32, u32),
    band: (i64, i64),
) -> Check {
    Check {
        what,
        ind,
        kind,
        months,
        band_ppm: (pct(band.0), pct(band.1)),
        checked: true,
    }
}

/// A watched expectation; band edges in hundredths of the unit.
pub const fn watched(
    what: &'static str,
    ind: Ind,
    kind: Kind,
    months: (u32, u32),
    band: (i64, i64),
) -> Check {
    Check {
        what,
        ind,
        kind,
        months,
        band_ppm: (pct(band.0), pct(band.1)),
        checked: false,
    }
}

// --- runs --------------------------------------------------------------------

/// What a run reads at a monthly close.
#[derive(Debug, Clone)]
pub struct Monthly {
    /// The day of the close.
    pub close: Date,
    /// The month's statistics.
    pub stats: MonthStats,
    /// Lei per euro, millionths, at the close.
    pub exchange_rate_ppm: u32,
    /// `cb_credibility`, millionths, at the close.
    pub credibility_ppm: u32,
    /// Each industry's import price, millionths of its start, at the close.
    pub import_price_ppm: Vec<u32>,
    /// Each industry's price, millionths of its start, at the close.
    pub price_ppm: Vec<u32>,
    /// The rest of the world in the month.
    pub trade: Option<TradeMonth>,
    /// The government's budget of the month.
    pub fiscal: Option<FiscalMonth>,
    /// Employees at the close: the weighted sum of their gross monthly wages
    /// (bani) and their weighted count. Reading (AC-POL-07, restated
    /// 2026-10-25): an employee is a person whose `wage` is above zero, as
    /// the glossary's `wage` is "the person's gross monthly wage as an
    /// employee; 0 for everyone else", so the self-employed are left out.
    pub employee_wages: (i128, u64),
}

/// A whole run: the industries' ids and the 39 monthly closes.
#[derive(Debug, Clone)]
pub struct Run {
    /// Catalogue industry ids, in the World's order.
    pub industries: Vec<String>,
    /// Monthly closes, December 2021 to February 2025.
    pub months: Vec<Monthly>,
}

impl Run {
    /// The index of month `k` after T0 (month 1 is March 2022).
    fn index(&self, k: u32) -> usize {
        let one = self
            .months
            .iter()
            .position(|m| m.close == close_of_month_one())
            .expect("the run closes March 2022");
        one + k as usize - 1
    }

    /// The month `k` after T0.
    pub fn month(&self, k: u32) -> &Monthly {
        let i = self.index(k);
        self.months
            .get(i)
            .unwrap_or_else(|| panic!("month {k} after T0 is not in the run"))
    }
}

/// The scale's starting state and law: 1:1000 and 1:100 from the shared
/// support, 1:10 built here once.
fn base(scale: u32) -> (StartState, &'static ScenarioLaw) {
    match scale {
        1000 | 100 => (start_state(scale), scenario_law(scale)),
        10 => {
            static TEN: OnceLock<(StartState, ScenarioLaw)> = OnceLock::new();
            let (start, law) = TEN.get_or_init(|| {
                let scenario = build_scenario(10);
                let dir = fresh_dir("policy_romania_10");
                write(&scenario, inputs(), &dir).expect("the scenario writes");
                let start = read_start_state(&dir, &io_table_path())
                    .expect("the scenario reads as a starting state");
                let law = match read_scenario_law(&dir, registry()) {
                    Ok(law) => law,
                    Err((line, r)) => panic!(
                        "the 1:10 scenario's law reads: line {line}: {:?} {}",
                        r.code, r.message
                    ),
                };
                (start, law)
            });
            (start.clone(), law)
        }
        _ => panic!("no policy runs at 1:{scale}"),
    }
}

/// The registry of the scale's scenario (keys from its tables).
pub fn scenario_registry(scale: u32) -> &'static Registry {
    &base(scale).1.registry
}

/// The commands files' folder.
pub fn commands_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/acceptance/policy/commands")
}

/// Weighted persons.
fn persons(w: &World) -> u64 {
    person_weights(w).iter().map(|&x| u64::from(x)).sum()
}

/// Employees' weighted wage sum and weighted count (`Monthly::employee_wages`).
fn employee_wages(w: &World) -> (i128, u64) {
    let weights = person_weights(w);
    let mut sum = 0i128;
    let mut count = 0u64;
    for (&wage, &wt) in w.persons().wage.iter().zip(&weights) {
        if wage.get() > 0 {
            sum += i128::from(wage.get()) * i128::from(wt);
            count += u64::from(wt);
        }
    }
    (sum, count)
}

/// One run, uncached. `Err` when a command is rejected.
fn run_uncached(scale: u32, commands: Option<&str>) -> Result<Run, String> {
    let (start, law) = base(scale);
    let log = match commands {
        None => Vec::new(),
        Some(file) => {
            read_commands(&commands_dir().join(file), &law.registry).map_err(|(line, r)| {
                format!(
                    "{file} line {line} rejected: {} ({:?}): {}",
                    r.code.as_str(),
                    r.lever.map(|l| l.as_str()),
                    r.message
                )
            })?
        }
    };
    let mut w = World::with_law(start, law.registry.clone(), law.law.clone(), params())
        .map_err(|e| format!("the world builds: {e:?}"))?;
    let industries = w.industries().id.clone();
    let persons0 = persons(&w);
    let mut months: Vec<Monthly> = Vec::new();
    let mut i = 0;
    while w.date() <= last_day() {
        let today = w.date();
        while i < log.len() && log[i].given == today {
            let order = match &log[i].order {
                Order::Set(c) => Order::Set(Command {
                    effective: log[i].effective,
                    ..c.clone()
                }),
                cancel @ Order::Cancel { .. } => cancel.clone(),
            };
            let shown = format!("{order:?}");
            w.give(order).map_err(|r| {
                format!(
                    "{} on {today:?}: {shown} rejected {}: {}",
                    commands.unwrap_or("?"),
                    r.code.as_str(),
                    r.message
                )
            })?;
            i += 1;
        }
        let r = step(&mut w);
        assert_day_checks(&w, &r);
        assert_eq!(
            persons(&w),
            persons0,
            "{:?}: person conservation (no demography in M4)",
            r.date
        );
        if w.months().len() > months.len() {
            let stats = w.months().last().expect("a closed month").clone();
            months.push(Monthly {
                close: r.date,
                stats,
                exchange_rate_ppm: w.foreign().exchange_rate_ppm,
                credibility_ppm: w.central_bank().credibility_ppm,
                import_price_ppm: w.foreign().import_price_ppm.clone(),
                price_ppm: w.industries().price_ppm.clone(),
                trade: None,
                fiscal: None,
                employee_wages: employee_wages(&w),
            });
        }
    }
    if i != log.len() {
        return Err(format!(
            "{}: {} line(s) dated after the run's last day",
            commands.unwrap_or("?"),
            log.len() - i
        ));
    }
    for m in &mut months {
        let month = m.stats.month;
        m.trade = w.trade_months().iter().find(|t| t.month == month).cloned();
        m.fiscal = w.fiscal_months().iter().find(|f| f.month == month).cloned();
    }
    if months.len() != MONTHS_IN_RUN {
        return Err(format!(
            "{} monthly closes from 1 December 2021 to 28 February 2025, not {MONTHS_IN_RUN}",
            months.len()
        ));
    }
    Ok(Run { industries, months })
}

type Slot = Arc<OnceLock<Result<Arc<Run>, String>>>;
/// The runs by scale and commands file (`None`: the baseline).
type Cache = Mutex<BTreeMap<(u32, Option<&'static str>), Slot>>;

/// The cache slot of a run.
fn slot(scale: u32, commands: Option<&'static str>) -> Slot {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let mut map = CACHE
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    map.entry((scale, commands)).or_default().clone()
}

/// A run, cached: computed on its own thread so that a panic (a failed
/// check of the day) is kept as the run's error.
pub fn run(scale: u32, commands: Option<&'static str>) -> Result<Arc<Run>, String> {
    let s = slot(scale, commands);
    s.get_or_init(|| {
        let name = format!("policy 1:{scale} {}", commands.unwrap_or("baseline"));
        std::thread::Builder::new()
            .name(name.clone())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || run_uncached(scale, commands))
            .expect("a run thread")
            .join()
            .unwrap_or_else(|p| Err(format!("{name}: {}", panic_text(p.as_ref()))))
            .map(Arc::new)
    })
    .clone()
}

fn panic_text(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_else(|| "a panic".to_string())
}

/// The placebo's commands file (standard VAT 19.00% to 19.01% from T0).
pub const PLACEBO: &str = "placebo.jsonl";

/// The baseline and the placebo of a scale.
pub struct Runs {
    /// No command.
    pub baseline: Arc<Run>,
    /// The one-basis-point VAT move.
    pub placebo: Arc<Run>,
}

/// The baseline and placebo of a scale, cached.
pub fn runs(scale: u32) -> Result<Runs, String> {
    Ok(Runs {
        baseline: run(scale, None).map_err(|e| format!("the baseline at 1:{scale}: {e}"))?,
        placebo: run(scale, Some(PLACEBO)).map_err(|e| format!("the placebo at 1:{scale}: {e}"))?,
    })
}

/// Compute every run the scales and scenarios need, in parallel, before the
/// first is read.
pub fn prepare(scales: &[u32], scenarios: &[&'static Scenario]) {
    let mut wanted: Vec<(u32, Option<&'static str>)> = Vec::new();
    for &scale in scales {
        wanted.push((scale, None));
        wanted.push((scale, Some(PLACEBO)));
        for s in scenarios {
            wanted.push((scale, Some(s.commands)));
        }
    }
    std::thread::scope(|sc| {
        for &(scale, commands) in &wanted {
            sc.spawn(move || {
                let _ = run(scale, commands);
            });
        }
    });
}

// --- measurements --------------------------------------------------------------

/// An indicator's value at a close, in a unit fit for its kind: for the
/// ratio kinds any positive scale; for points, millionths of a percentage
/// point; for % of GDP, bani.
fn value(ind: Ind, run: &Run, m: &Monthly) -> Result<i128, String> {
    let s = &m.stats;
    let cpi = i128::from(s.cpi_ppm);
    let real = |x: Bani| -> Result<i128, String> {
        if cpi <= 0 {
            return Err(format!("the CPI is {cpi} in {:?}", m.close));
        }
        Ok(i128::from(x.get()) * ONE / cpi)
    };
    let trade = || {
        m.trade
            .as_ref()
            .ok_or_else(|| format!("no trade month for {:?}", m.close))
    };
    let fiscal = || {
        m.fiscal
            .as_ref()
            .ok_or_else(|| format!("no fiscal month for {:?}", m.close))
    };
    Ok(match ind {
        Ind::PriceLevel => cpi,
        Ind::Inflation => i128::from(s.inflation_ppm) * 100,
        Ind::ExpectedInflation => i128::from(s.expected_inflation_ppm) * 100,
        Ind::Unemployment => {
            let labour = i128::from(s.employed) + i128::from(s.unemployed);
            if labour == 0 {
                return Err(format!("no labour force in {:?}", m.close));
            }
            i128::from(s.unemployed) * HUNDRED / labour
        }
        Ind::MeanWage => {
            let (sum, count) = m.employee_wages;
            if count == 0 {
                return Err(format!("no employee in {:?}", m.close));
            }
            sum * 1_000 / i128::from(count)
        }
        Ind::Consumption => i128::from(s.consumption.get()),
        Ind::RealConsumption => real(s.consumption)?,
        Ind::RealGdp => real(s.gdp_expenditure)?,
        Ind::ExchangeRate => i128::from(m.exchange_rate_ppm),
        Ind::ExportVolume => i128::from(trade()?.exports_volume),
        Ind::CurrentAccount => i128::from(trade()?.current_account.get()),
        Ind::Deficit => i128::from(fiscal()?.deficit.get()),
        Ind::Flow(code) => {
            let f = fiscal()?;
            f.revenue
                .iter()
                .chain(&f.spending)
                .filter(|(c, _)| c.name() == code)
                .map(|(_, v)| i128::from(v.get()))
                .sum()
        }
        Ind::Credibility => i128::from(m.credibility_ppm) * 100,
        Ind::Employment => i128::from(s.employed),
        Ind::Hardship => i128::from(s.hardship_households),
        Ind::Price(id) => {
            let j = run
                .industries
                .iter()
                .position(|x| x == id)
                .ok_or_else(|| format!("no industry {id}"))?;
            i128::from(m.price_ppm[j])
        }
        Ind::ImportPriceIndex => {
            let n = m.import_price_ppm.len();
            if n == 0 {
                return Err("no import prices".to_string());
            }
            m.import_price_ppm
                .iter()
                .map(|&x| i128::from(x))
                .sum::<i128>()
                / i128::try_from(n).expect("fits")
        }
    })
}

/// Nominal GDP (`gdp_expenditure`) of a close, bani.
fn gdp(m: &Monthly) -> i128 {
    i128::from(m.stats.gdp_expenditure.get())
}

/// `a / b`, refusing a base at or below zero.
fn over(a: i128, b: i128, what: &str) -> Result<i128, String> {
    if b <= 0 {
        return Err(format!("{what}: the baseline is {b}, not positive"));
    }
    Ok(a / b)
}

/// The measured difference of `scen` against `base` over a check's window,
/// in millionths of the kind's unit (spec, "Differences against the
/// baseline").
pub fn measure(c: &Check, scen: &Run, base: &Run) -> Result<i64, String> {
    let (a, b) = c.months;
    let n = i128::from(b - a + 1);
    let mut xs = Vec::new();
    let mut xb = Vec::new();
    let mut gb = Vec::new();
    for k in a..=b {
        xs.push(value(c.ind, scen, scen.month(k))?);
        xb.push(value(c.ind, base, base.month(k))?);
        gb.push(gdp(base.month(k)));
    }
    let what = c.what;
    let d = match c.kind {
        Kind::LevelPct => {
            let mut sum = 0i128;
            for (s, b) in xs.iter().zip(&xb) {
                sum += over(HUNDRED * (s - b), *b, what)?;
            }
            sum / n
        }
        Kind::FlowPct => {
            let (ss, sb): (i128, i128) = (xs.iter().sum(), xb.iter().sum());
            over(HUNDRED * (ss - sb), sb, what)?
        }
        Kind::Points => xs.iter().zip(&xb).map(|(s, b)| s - b).sum::<i128>() / n,
        Kind::PctOfGdp => {
            let diff: i128 = xs.iter().zip(&xb).map(|(s, b)| s - b).sum();
            over(HUNDRED * diff, gb.iter().sum(), what)?
        }
    };
    i64::try_from(d).map_err(|_| format!("{what}: a difference that does not fit"))
}

/// The noise floor ν of a check: the largest absolute monthly difference
/// between the placebo and the baseline over the check's window, in the
/// check's unit. Monthly differences: for the % kinds `100 (x^p_k / x^b_k
/// − 1)`; for points `x^p_k − x^b_k`; for % of GDP `100 (x^p_k − x^b_k) /
/// GDP^b_k` (reading: the month's share of the month's GDP).
pub fn noise_floor(c: &Check, runs: &Runs) -> Result<i64, String> {
    let (p, b0) = (&*runs.placebo, &*runs.baseline);
    let mut nu = 0i128;
    for k in c.months.0..=c.months.1 {
        let (xp, xb) = (value(c.ind, p, p.month(k))?, value(c.ind, b0, b0.month(k))?);
        let d = match c.kind {
            Kind::LevelPct | Kind::FlowPct => over(HUNDRED * (xp - xb), xb, c.what)?,
            Kind::Points => xp - xb,
            Kind::PctOfGdp => over(HUNDRED * (xp - xb), gdp(b0.month(k)), c.what)?,
        };
        nu = nu.max(d.abs());
    }
    i64::try_from(nu).map_err(|_| format!("{}: a noise floor that does not fit", c.what))
}

/// The result of one check.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The scenario.
    pub scenario: &'static str,
    /// The check.
    pub check: Check,
    /// The scale.
    pub scale: u32,
    /// The measured difference, millionths of the unit.
    pub measured: Result<i64, String>,
    /// The noise floor, millionths of the unit.
    pub nu: Result<i64, String>,
    /// Whether it holds: expected sign, in the band, and at least 2ν unless
    /// the band contains zero.
    pub holds: bool,
    /// Why not, when it does not.
    pub why: String,
}

impl Outcome {
    /// A line for the test's output.
    pub fn line(&self) -> String {
        let unit = match self.check.kind {
            Kind::LevelPct | Kind::FlowPct => "%",
            Kind::Points => " points",
            Kind::PctOfGdp => "% of GDP",
        };
        let show = |r: &Result<i64, String>| match r {
            Ok(v) => fmt_ppm(*v),
            Err(e) => format!("error: {e}"),
        };
        format!(
            "{} 1:{} [{}] {} (months {}-{}): measured {}{unit}, band [{}, {}], 2nu {} -> {}{}",
            self.scenario,
            self.scale,
            if self.check.checked {
                "checked"
            } else {
                "watched"
            },
            self.check.what,
            self.check.months.0,
            self.check.months.1,
            show(&self.measured),
            fmt_ppm(self.check.band_ppm.0),
            fmt_ppm(self.check.band_ppm.1),
            self.nu
                .as_ref()
                .map_or_else(|e| format!("error: {e}"), |v| fmt_ppm(2 * v)),
            if self.holds { "holds" } else { "FAILS" },
            if self.holds {
                String::new()
            } else {
                format!(": {}", self.why)
            }
        )
    }
}

/// Millionths as a decimal with three places.
pub fn fmt_ppm(v: i64) -> String {
    let sign = if v < 0 { "-" } else { "" };
    let a = v.unsigned_abs();
    format!("{sign}{}.{:03}", a / 1_000_000, (a % 1_000_000) / 1_000)
}

/// Whether a band contains zero (its check skips the 2ν rule and has no
/// required sign).
pub fn band_has_zero(c: &Check) -> bool {
    c.band_ppm.0 <= 0 && 0 <= c.band_ppm.1
}

/// Evaluate one scenario's checks at a scale.
pub fn evaluate(s: &Scenario, scale: u32, scen: &Run, runs: &Runs) -> Vec<Outcome> {
    s.checks
        .iter()
        .map(|c| {
            let measured = measure(c, scen, &runs.baseline);
            let nu = noise_floor(c, runs);
            let (lo, hi) = c.band_ppm;
            let (holds, why) = match (&measured, &nu) {
                (Err(e), _) | (_, Err(e)) => (false, e.clone()),
                (Ok(d), Ok(nu)) => {
                    let d = *d;
                    if band_has_zero(c) {
                        if lo <= d && d <= hi {
                            (true, String::new())
                        } else {
                            (false, "outside the band".to_string())
                        }
                    } else if (d > 0) != (lo > 0) || d == 0 {
                        (false, "the wrong sign".to_string())
                    } else if d < lo || d > hi {
                        (false, "outside the band".to_string())
                    } else if i128::from(d).abs() < 2 * i128::from(*nu) {
                        (false, "below twice the noise floor".to_string())
                    } else {
                        (true, String::new())
                    }
                }
            };
            Outcome {
                scenario: s.id,
                check: *c,
                scale,
                measured,
                nu,
                holds,
                why,
            }
        })
        .collect()
}

/// Run a scenario at a scale (and its baseline and placebo, cached), print
/// every outcome, and return the failed checked ones as lines (empty when
/// the scenario passes). A rejected command or a failed run is a failure.
pub fn scenario_failures(s: &'static Scenario, scale: u32) -> Vec<String> {
    let runs = match runs(scale) {
        Ok(r) => r,
        Err(e) => return vec![format!("{} 1:{scale}: {e}", s.id)],
    };
    let scen = match run(scale, Some(s.commands)) {
        Ok(r) => r,
        Err(e) => return vec![format!("{} ({}) 1:{scale}: {e}", s.id, s.ac)],
    };
    let mut failed = Vec::new();
    for o in evaluate(s, scale, &scen, &runs) {
        println!("{}", o.line());
        if o.check.checked && !o.holds {
            failed.push(o.line());
        }
    }
    failed
}

/// A scenario criterion at one scale: every checked expectation holds.
pub fn assert_scenario(s: &'static Scenario, scale: u32) {
    prepare(&[scale], &[s]);
    let failed = scenario_failures(s, scale);
    assert!(
        failed.is_empty(),
        "{} ({}) at 1:{scale}: {} checked expectation(s) fail:\n{}",
        s.id,
        s.ac,
        failed.len(),
        failed.join("\n")
    );
}

/// The placebo's measured difference for every checked expectation of the
/// scenarios, each below the band's bound nearest zero in absolute value
/// (AC-POL-10). Returns the failures.
pub fn placebo_failures(scenarios: &[&'static Scenario], scale: u32) -> Vec<String> {
    let runs = match runs(scale) {
        Ok(r) => r,
        Err(e) => return vec![e],
    };
    let mut failed = Vec::new();
    for s in scenarios {
        for c in s.checks.iter().filter(|c| c.checked) {
            let bound = c.band_ppm.0.unsigned_abs().min(c.band_ppm.1.unsigned_abs());
            match measure(c, &runs.placebo, &runs.baseline) {
                Ok(d) => {
                    let line = format!(
                        "placebo 1:{scale} against {} {}: {} (bound nearest zero {})",
                        s.id,
                        c.what,
                        fmt_ppm(d),
                        fmt_ppm(i64::try_from(bound).expect("fits"))
                    );
                    println!("{line}");
                    if d.unsigned_abs() >= bound {
                        failed.push(line);
                    }
                }
                Err(e) => failed.push(format!(
                    "placebo 1:{scale} against {} {}: {e}",
                    s.id, c.what
                )),
            }
        }
    }
    failed
}

/// Whether the placebo splits the paths (spec, "Placebo and noise floor"):
/// some month after T0 whose statistics, trade, budget, exchange rate,
/// credibility or prices differ from the baseline's. `None` when it does;
/// else the failure.
pub fn placebo_split_failure(scale: u32) -> Option<String> {
    let runs = match runs(scale) {
        Ok(r) => r,
        Err(e) => return Some(e),
    };
    let differs = (1..=MONTHS_AFTER_T0).any(|k| {
        let (b, p) = (runs.baseline.month(k), runs.placebo.month(k));
        b.stats != p.stats
            || b.trade != p.trade
            || b.fiscal != p.fiscal
            || b.exchange_rate_ppm != p.exchange_rate_ppm
            || b.credibility_ppm != p.credibility_ppm
            || b.price_ppm != p.price_ppm
    });
    if differs {
        None
    } else {
        Some(format!(
            "placebo 1:{scale}: standard VAT at 19.01% from T0 leaves every month's statistics, \
             trade, budget, exchange rate, credibility and prices equal to the baseline's, so \
             every noise floor is zero: the placebo does not split the paths"
        ))
    }
}
