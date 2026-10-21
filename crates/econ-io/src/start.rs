//! A built scenario folder as the starting state of the `World` (spec
//! `game/world-loop`), with the input-output table's coefficients, the
//! household basket and the placeholder final demand mapped onto the
//! catalogue industries.
//!
//! A catalogue industry has the coefficients of its table industries,
//! weighted by its output in each, and its inputs are split over the
//! supplying catalogue industries by their output shares (spec
//! `economy/production`, "Starting state"). Household purchases, government
//! purchases, investment and exports of a table industry are split over its
//! catalogue industries by the same shares. Integer arithmetic only.

use std::path::Path;

use arrow_array::{
    Array, BooleanArray, Int64Array, RecordBatch, StringArray, UInt8Array, UInt16Array,
    UInt32Array, UInt64Array,
};
use econ_core::world::StartState;
use econ_popgen::EduLevel;
use econ_types::Bani;
use serde_json::Value;

use crate::{SaveError, read_batch};

const ONE: i128 = 1_000_000;

fn read(dir: &Path, name: &str) -> Result<RecordBatch, SaveError> {
    let bytes = std::fs::read(dir.join(name)).map_err(|e| SaveError::Io(format!("{name}: {e}")))?;
    read_batch(bytes)
}

fn column<'a, T: 'static>(b: &'a RecordBatch, file: &str, name: &str) -> Result<&'a T, SaveError> {
    b.column_by_name(name)
        .ok_or_else(|| SaveError::Parse(format!("{file}: no column {name}")))?
        .as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| SaveError::Parse(format!("{file}: column {name} has another type")))
}

fn u8s(b: &RecordBatch, f: &str, n: &str) -> Result<Vec<u8>, SaveError> {
    Ok(column::<UInt8Array>(b, f, n)?.values().to_vec())
}
fn u16s(b: &RecordBatch, f: &str, n: &str) -> Result<Vec<u16>, SaveError> {
    Ok(column::<UInt16Array>(b, f, n)?.values().to_vec())
}
fn u32s(b: &RecordBatch, f: &str, n: &str) -> Result<Vec<u32>, SaveError> {
    Ok(column::<UInt32Array>(b, f, n)?.values().to_vec())
}
fn money(b: &RecordBatch, f: &str, n: &str) -> Result<Vec<Bani>, SaveError> {
    column::<UInt64Array>(b, f, n)?
        .values()
        .iter()
        .map(|&v| {
            i64::try_from(v)
                .map(Bani)
                .map_err(|_| SaveError::Parse(format!("{f}: {n} is too large")))
        })
        .collect()
}

fn parse(why: impl Into<String>) -> SaveError {
    SaveError::Parse(why.into())
}

fn ints(v: &Value, what: &str) -> Result<Vec<i128>, SaveError> {
    v.as_array()
        .ok_or_else(|| parse(format!("io table: {what} is not a list")))?
        .iter()
        .map(|x| {
            x.as_i64()
                .map(i128::from)
                .ok_or_else(|| parse(format!("io table: {what} holds a non-integer")))
        })
        .collect()
}

fn matrix(v: &Value, what: &str) -> Result<Vec<Vec<i128>>, SaveError> {
    v.as_array()
        .ok_or_else(|| parse(format!("io table: {what} is not a table")))?
        .iter()
        .map(|row| ints(row, what))
        .collect()
}

fn edu(code: u8) -> Result<EduLevel, SaveError> {
    Ok(match code {
        0 => EduLevel::Isced0To2,
        1 => EduLevel::Isced3,
        2 => EduLevel::Isced4,
        3 => EduLevel::Isced5To6,
        4 => EduLevel::Isced7To8,
        _ => return Err(parse(format!("persons: education level {code}"))),
    })
}

/// A non-negative i128 as millionths.
fn ppm(v: i128) -> u32 {
    u32::try_from(v.clamp(0, i128::from(u32::MAX))).expect("clamped")
}

/// Lei as bani.
fn bani_of_lei(v: i128) -> Bani {
    Bani(i64::try_from(v.max(0) * 100).expect("an amount that fits i64 bani"))
}

/// `n / d` rounded half away from zero (`d > 0`).
fn rdiv(n: i128, d: i128) -> i128 {
    let (q, r) = (n / d, n % d);
    if 2 * r.abs() >= d { q + n.signum() } else { q }
}

/// Read a scenario folder and the input-output table as a `StartState`.
///
/// # Errors
/// A file is missing or malformed.
#[allow(clippy::too_many_lines)]
pub fn read_start_state(scenario_dir: &Path, io_table: &Path) -> Result<StartState, SaveError> {
    let scenario: Value = serde_json::from_slice(
        &std::fs::read(scenario_dir.join("scenario.json"))
            .map_err(|e| SaveError::Io(format!("scenario.json: {e}")))?,
    )
    .map_err(|e| parse(format!("scenario.json: {e}")))?;
    let sample_scale = scenario["sample_scale"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| parse("scenario.json: no sample_scale"))?;
    let rng_seed = scenario["rng_seed"]
        .as_u64()
        .ok_or_else(|| parse("scenario.json: no rng_seed"))?;

    let (hf, pf, ff, bf) = (
        "tables/households.arrow",
        "tables/persons.arrow",
        "tables/firm_units.arrow",
        "balances.arrow",
    );
    let hb = read(scenario_dir, hf)?;
    let pb = read(scenario_dir, pf)?;
    let fb = read(scenario_dir, ff)?;
    let bb = read(scenario_dir, bf)?;

    // The opening balances, in matrix order.
    let (instrument, sector) = (u8s(&bb, bf, "instrument")?, u8s(&bb, bf, "sector")?);
    let balance = column::<Int64Array>(&bb, bf, "balance")?.values().to_vec();
    let n_instruments = 1 + usize::from(instrument.iter().copied().max().unwrap_or(0));
    let mut balances = vec![[Bani::ZERO; 8]; n_instruments];
    for k in 0..balance.len() {
        let (i, s) = (usize::from(instrument[k]), usize::from(sector[k]));
        *balances[i]
            .get_mut(s)
            .ok_or_else(|| parse(format!("{bf}: sector {s}")))? = Bani(balance[k]);
    }

    // Firm units, and the catalogue industries they name.
    let ids = column::<StringArray>(&fb, ff, "unit_id")?;
    let unit_industry = u8s(&fb, ff, "industry")?;
    let unit_group = u8s(&fb, ff, "industry_group")?;
    let kind = u8s(&fb, ff, "kind")?;
    let controller = u8s(&fb, ff, "controller")?;
    let n_i = 1 + usize::from(unit_industry.iter().copied().max().unwrap_or(0));
    let mut industry_id = vec![String::new(); n_i];
    let mut industry_group = vec![0u8; n_i];
    for u in 0..ids.len() {
        let j = usize::from(unit_industry[u]);
        let id = ids.value(u);
        industry_id[j] = id.split('.').next().unwrap_or(id).to_string();
        industry_group[j] = unit_group[u];
    }
    if industry_id.iter().any(String::is_empty) {
        return Err(parse(format!("{ff}: an industry without units")));
    }

    // The input-output table, mapped onto the catalogue.
    let io: Value = serde_json::from_slice(
        &std::fs::read(io_table)
            .map_err(|e| SaveError::Io(format!("{}: {e}", io_table.display())))?,
    )
    .map_err(|e| parse(format!("io table: {e}")))?;
    let codes: Vec<String> = io["industries"]
        .as_array()
        .ok_or_else(|| parse("io table: no industries"))?
        .iter()
        .map(|i| i["code"].as_str().unwrap_or_default().to_string())
        .collect();
    let n_t = codes.len();
    let output = ints(&io["output"], "output")?;
    let domestic = matrix(&io["domestic"], "domestic")?;
    let imports_eu = matrix(&io["imports_eu"], "imports_eu")?;
    let imports_non_eu = matrix(&io["imports_non_eu"], "imports_non_eu")?;
    let other_taxes = ints(&io["other_taxes_less_subsidies"], "other taxes")?;
    let product_taxes = ints(&io["product_taxes"], "product taxes")?;
    let exports: Vec<i128> = ints(&io["exports_eu"], "exports")?
        .iter()
        .zip(ints(&io["exports_non_eu"], "exports")?)
        .map(|(a, b)| a + b)
        .collect();
    let final_use = |use_: &str, part: &str| ints(&io["final"][use_][part], use_);
    let (hh_dom, gov_dom, inv_dom) = (
        final_use("households", "domestic")?,
        final_use("government", "domestic")?,
        final_use("investment", "domestic")?,
    );
    let sum2 =
        |a: Vec<i128>, b: Vec<i128>| -> Vec<i128> { a.iter().zip(b).map(|(x, y)| x + y).collect() };
    let hh_imp = sum2(
        final_use("households", "imports_eu")?,
        final_use("households", "imports_non_eu")?,
    );
    let gov_imp = sum2(
        final_use("government", "imports_eu")?,
        final_use("government", "imports_non_eu")?,
    );
    let inv_imp = sum2(
        final_use("investment", "imports_eu")?,
        final_use("investment", "imports_non_eu")?,
    );
    // share[t][c]: catalogue industry c's share of table industry t's output, millionths.
    let mut share = vec![vec![0i128; n_i]; n_t];
    for entry in io["catalogue"]
        .as_array()
        .ok_or_else(|| parse("io table: no catalogue"))?
    {
        let code = entry["industry"].as_str().unwrap_or_default();
        let t = codes
            .iter()
            .position(|c| c == code)
            .ok_or_else(|| parse(format!("io table: catalogue code {code}")))?;
        for part in entry["parts"]
            .as_array()
            .ok_or_else(|| parse("io table: catalogue parts"))?
        {
            let id = part["id"].as_str().unwrap_or_default();
            let c = industry_id.iter().position(|x| x == id).ok_or_else(|| {
                parse(format!(
                    "io table: catalogue industry {id} has no firm unit"
                ))
            })?;
            share[t][c] = i128::from(
                part["share"]
                    .as_i64()
                    .ok_or_else(|| parse("io table: a share"))?,
            );
        }
    }
    for (t, row) in share.iter().enumerate() {
        if output[t] > 0 && row.iter().sum::<i128>() != ONE {
            return Err(parse(format!(
                "io table: the catalogue shares of {} do not sum to one",
                codes[t]
            )));
        }
    }
    // A catalogue industry's output (times a million).
    let out_c: Vec<i128> = (0..n_i)
        .map(|c| (0..n_t).map(|t| share[t][c] * output[t]).sum())
        .collect();
    let mut home_coefficient_ppm = vec![vec![0u32; n_i]; n_i];
    let mut import_coefficient_ppm = vec![0u32; n_i];
    let mut other_cost_ppm = vec![0u32; n_i];
    for c in 0..n_i {
        if out_c[c] <= 0 {
            continue;
        }
        let mut home = vec![0i128; n_i];
        let (mut imported, mut other) = (0i128, 0i128);
        for t in 0..n_t {
            let s = share[t][c];
            if s == 0 {
                continue;
            }
            for t2 in 0..n_t {
                let v = domestic[t2][t];
                if v == 0 {
                    continue;
                }
                for c2 in 0..n_i {
                    if share[t2][c2] > 0 {
                        home[c2] += s * v * share[t2][c2];
                    }
                }
                imported += s * (imports_eu[t2][t] + imports_non_eu[t2][t]);
            }
            other += s * (other_taxes[t] + product_taxes[t]);
        }
        for c2 in 0..n_i {
            home_coefficient_ppm[c][c2] = ppm(rdiv(home[c2], out_c[c]));
        }
        import_coefficient_ppm[c] = ppm(rdiv(imported * ONE, out_c[c]));
        other_cost_ppm[c] = ppm(rdiv(other * ONE, out_c[c]));
    }
    let by_catalogue = |v: &[i128]| -> Vec<i128> {
        (0..n_i)
            .map(|c| (0..n_t).map(|t| share[t][c] * v[t]).sum::<i128>() / ONE)
            .collect()
    };
    let (b_dom, b_imp) = (by_catalogue(&hh_dom), by_catalogue(&hh_imp));
    let b_total: Vec<i128> = b_dom.iter().zip(&b_imp).map(|(a, b)| a + b).collect();
    let weights: Vec<u64> = b_total
        .iter()
        .map(|&v| u64::try_from(v.max(0)).expect("non-negative"))
        .collect();
    let basket_ppm: Vec<u32> = econ_types::split_largest_remainder(Bani(1_000_000), &weights)
        .into_iter()
        .map(|b| u32::try_from(b.get()).expect("a share"))
        .collect();
    let basket_import_ppm: Vec<u32> = (0..n_i)
        .map(|c| {
            if b_total[c] > 0 {
                ppm(rdiv(b_imp[c] * ONE, b_total[c]))
            } else {
                0
            }
        })
        .collect();
    let money_of =
        |v: &[i128]| -> Vec<Bani> { by_catalogue(v).into_iter().map(bani_of_lei).collect() };
    // Each catalogue industry's main table industry: storable goods, price flexibility.
    let mut goods = vec![false; n_i];
    let mut flexibility_ppm = vec![100_000u32; n_i];
    for c in 0..n_i {
        let Some(t) = (0..n_t).max_by_key(|&t| (share[t][c] * output[t], std::cmp::Reverse(t)))
        else {
            continue;
        };
        let code = codes[t].as_str();
        let section = code.chars().next().unwrap_or(' ');
        goods[c] = matches!(section, 'A' | 'B' | 'C');
        flexibility_ppm[c] = match section {
            'A' | 'B' | 'D' => 500_000,
            'C' if code == "C19" => 500_000,
            'C' => 250_000,
            _ => 100_000,
        };
    }

    // Households and persons.
    let hh_weight = u32s(&hb, hf, "hh_weight")?;
    let hh_county = u8s(&hb, hf, "hh_county")?;
    let hh_region = u8s(&hb, hf, "hh_region")?;
    let hh_collective = column::<BooleanArray>(&hb, hf, "hh_collective")?;
    let hh_collective: Vec<bool> = (0..hh_collective.len())
        .map(|k| hh_collective.value(k))
        .collect();
    let hh_cash = money(&hb, hf, "hh_cash")?;
    let hh_deposits = money(&hb, hf, "hh_deposits")?;
    let hh_subsistence_income = money(&hb, hf, "hh_subsistence_income")?;
    let self_employment = money(&pb, pf, "self_employment_income")?;
    let informal = money(&pb, pf, "informal_income")?;
    let other_income = self_employment
        .iter()
        .zip(&informal)
        .map(|(&a, &b)| a + b)
        .collect();
    let edu_level = u8s(&pb, pf, "edu_level")?
        .into_iter()
        .map(edu)
        .collect::<Result<_, _>>()?;
    let mut s = StartState::default();
    s.sample_scale = sample_scale;
    s.rng_seed = rng_seed;
    s.hh_weight = hh_weight;
    s.hh_county = hh_county;
    s.hh_region = hh_region;
    s.hh_collective = hh_collective;
    s.self_employment_declared = money(&pb, pf, "self_employment_declared")?;
    s.unit_size = u8s(&fb, ff, "size")?;
    s.unit_firm_count = u32s(&fb, ff, "firm_count")?;
    s.hh_cash = hh_cash;
    s.hh_deposits = hh_deposits;
    s.hh_subsistence_income = hh_subsistence_income;
    s.household_id = u32s(&pb, pf, "household_id")?;
    s.age = u16s(&pb, pf, "age")?;
    s.edu_level = edu_level;
    s.activity = u8s(&pb, pf, "activity")?;
    s.employment_status = u8s(&pb, pf, "employment_status")?;
    s.occupation = u8s(&pb, pf, "occupation")?;
    s.wage = money(&pb, pf, "wage")?;
    s.wage_declared = money(&pb, pf, "wage_declared")?;
    s.pension = money(&pb, pf, "pension")?;
    s.other_income = other_income;
    s.employer = u16s(&pb, pf, "employer")?;
    s.unit_industry = unit_industry;
    s.unit_cohort = kind.iter().map(|&k| k == 0).collect();
    s.unit_domestic_cohort = kind
        .iter()
        .zip(&controller)
        .map(|(&k, &c)| k == 0 && c == 0)
        .collect();
    s.unit_output = money(&fb, ff, "output")?;
    s.unit_value_added = money(&fb, ff, "value_added")?;
    s.unit_wages = money(&fb, ff, "wages")?;
    s.unit_currency = money(&fb, ff, "currency")?;
    s.unit_deposits = money(&fb, ff, "deposits")?;
    s.balances = balances;
    s.industry_id = industry_id;
    s.industry_group = industry_group;
    s.goods = goods;
    s.flexibility_ppm = flexibility_ppm;
    s.home_coefficient_ppm = home_coefficient_ppm;
    s.import_coefficient_ppm = import_coefficient_ppm;
    s.other_cost_ppm = other_cost_ppm;
    s.basket_ppm = basket_ppm;
    s.basket_import_ppm = basket_import_ppm;
    s.gov_domestic = money_of(&gov_dom);
    s.gov_imported = money_of(&gov_imp);
    // Capital formation: fixed investment and the change in inventories.
    let inventories = final_use("inventories", "domestic")?;
    let formation: Vec<i128> = inv_dom
        .iter()
        .zip(&inventories)
        .map(|(a, b)| (a + b).max(0))
        .collect();
    s.investment_domestic = money_of(&formation);
    s.investment_imported = money_of(&inv_imp);
    s.exports = money_of(&exports);
    s.unit_loans = money(&fb, ff, "loans")?;
    s.unit_fixed_assets = money(&fb, ff, "fixed_assets")?;
    s.unit_controller = controller.clone();
    let (own_s, own_d, own_f) = (
        u32s(&fb, ff, "own_state")?,
        u32s(&fb, ff, "own_domestic_private")?,
        u32s(&fb, ff, "own_foreign")?,
    );
    s.unit_ownership_ppm = (0..own_s.len())
        .map(|u| [own_s[u], own_d[u], own_f[u]])
        .collect();
    s.hh_mortgage = money(&hb, hf, "hh_mortgage")?;
    s.hh_consumer_debt = money(&hb, hf, "hh_consumer_debt")?;
    s.gov_debt_profile = debt_profile(&scenario)?;
    s.gov_transfers_year = budget_transfers(scenario_dir)?;
    Ok(s)
}

/// The buckets of `gov_debt_by_maturity` in `scenario.json`: up to 1 year,
/// 1–3, 3–5, 5–10 and over 10 years (taken as 10 to 20).
const DEBT_BUCKETS: [(u8, u8); 5] = [(0, 1), (1, 3), (3, 5), (5, 10), (10, 20)];

/// The debt profile of `scenario.json`; none when it has no
/// `gov_debt_by_maturity` (a scenario written before it existed).
fn debt_profile(scenario: &Value) -> Result<Vec<(u8, u8, Bani)>, SaveError> {
    let Some(v) = scenario.get("gov_debt_by_maturity") else {
        return Ok(Vec::new());
    };
    let amounts = v
        .as_array()
        .filter(|a| a.len() == DEBT_BUCKETS.len())
        .ok_or_else(|| parse("scenario.json: gov_debt_by_maturity has five buckets"))?;
    DEBT_BUCKETS
        .iter()
        .zip(amounts)
        .map(|(&(from, to), a)| {
            a.as_i64()
                .map(|b| (from, to, Bani(b)))
                .ok_or_else(|| parse("scenario.json: gov_debt_by_maturity in whole bani"))
        })
        .collect()
}

/// The yearly budget lines the `World` does not pay through its own phases
/// (spec `economy/fiscal-policy`, "In", step 1), from `budget.json`: the
/// EU budget (D76) to the rest of the world; the other current transfers and
/// the other taxes to households; capital transfers and subsidies to firms.
/// None when the scenario has no budget.
fn budget_transfers(
    scenario_dir: &Path,
) -> Result<Vec<(econ_ledger::FlowCode, econ_ledger::Sector, Bani)>, SaveError> {
    use crate::budget::{Budget, EconType};
    use econ_ledger::{FlowCode, Sector};
    let path = scenario_dir.join("budget.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(&path).map_err(|e| SaveError::Io(format!("budget.json: {e}")))?;
    let b = Budget::from_json(&bytes).map_err(|e| parse(format!("budget.json: {e:?}")))?;
    let published = |code: &str| {
        b.published
            .0
            .iter()
            .find(|(c, _)| c == code)
            .map_or(Bani::ZERO, |&(_, v)| v)
    };
    let eu = published("D76");
    let current = b.total(EconType::CurrentTransfers);
    Ok(vec![
        (FlowCode::TRANSFER_EU_BUDGET, Sector::RestOfWorld, eu),
        (
            FlowCode::TRANSFER_OTHER_CURRENT,
            Sector::Households,
            current - eu + b.total(EconType::OtherTaxes),
        ),
        (
            FlowCode::TRANSFER_CAPITAL,
            Sector::Firms,
            b.total(EconType::CapitalTransfers),
        ),
        (
            FlowCode::SUBSIDY_PRODUCTION,
            Sector::Firms,
            b.total(EconType::Subsidies),
        ),
    ])
}
