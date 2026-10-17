//! The consistency report (spec `game/scenario-builder`): the checks of the
//! interim Python builder (`python/pipeline/build_scenario.py`), with its
//! figures and tolerances, plus four against the opening ledger. Amounts are
//! whole lei, rounded half away from zero; ratios are not rounded unless the
//! reference rounds them.

// Report arithmetic is on i128 sums of bani (far from overflow) and on
// floats compared with tolerances, as the reference computes them.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::float_arithmetic
)]

use econ_ledger::{Instrument, Sector};
use serde_json::{Map, Value, json};

use crate::build::{FIRM_SHEETS, FIRMS, IO, SHEETS, weighted};
use crate::opening::StatInstrument;
use crate::{Check, Inputs, Report, Scenario};

/// Weighted persons of the census.
const CENSUS_PERSONS: i128 = 19_053_815;
/// Mean disposable income of households by income fifth, Romania 2020, euro a
/// year (Eurostat `icw_res_02`). A check, not an input.
const INCOME_BY_FIFTH_2020: [f64; 5] = [2483.3, 5438.3, 7764.3, 11155.5, 18997.8];
/// `employment_status` of an employee, and of a person not employed.
const KIND_EMPLOYEE: u8 = 1;
const KIND_NONE: u8 = 0;
/// `employer` of a person without one.
const NO_EMPLOYER: u16 = u16::MAX;
/// The sector keys of the financial accounts, in `Sector::ALL` order.
const SECTOR_KEYS: [&str; 8] = [
    "households",
    "firms",
    "banks",
    "pension_funds",
    "other_financial",
    "government",
    "central_bank",
    "rest_of_world",
];

/// Why `income_distribution` fails until roadmap M3.
pub(crate) const INCOME_GAP: &str = "Benefits other than pensions (child allowance, unemployment benefit, minimum income) and taxes are not applied before the simulation runs, so the poorest fifth has under half the share the survey gives it and 6% of households have no income at all. Re-check after the tax and transfer mechanics exist (roadmap M3); if it still fails, informal income is too concentrated (14% of adults share it).";

/// Bani as whole lei, half away from zero.
fn lei(bani: i128) -> i64 {
    let whole = (bani.abs() + 50) / 100;
    i64::try_from(if bani < 0 { -whole } else { whole }).expect("lei fit i64")
}

/// A float rounded to four decimals, as Python's `round(x, 4)`.
fn round4(x: f64) -> f64 {
    format!("{x:.4}").parse().expect("a formatted float")
}

fn int(v: &Value) -> i128 {
    i128::from(v.as_i64().expect("whole lei in a checked input"))
}

/// Share of the amount held by each fifth of the weight; `pairs` are (sort
/// key, weight, amount). The reference's `quintile_shares`.
fn quintile_shares(mut pairs: Vec<(i128, i128, i128)>) -> [f64; 5] {
    pairs.sort_unstable();
    let total_w: i128 = pairs.iter().map(|p| p.1).sum();
    let total: i128 = pairs.iter().map(|p| p.1 * p.2).sum();
    let mut shares = [0i128; 5];
    let mut seen = 0i128;
    for &(_, wt, amount) in &pairs {
        let mut left = wt;
        // A record may straddle two fifths.
        while left > 0 {
            let q = (seen * 5 / total_w).min(4);
            let room = if q < 4 {
                (q + 1) * total_w / 5 - seen
            } else {
                left
            };
            let part = left.min(room.max(1));
            shares[usize::try_from(q).expect("a fifth")] += part * amount;
            seen += part;
            left -= part;
        }
    }
    shares.map(|s| {
        if total == 0 {
            0.0
        } else {
            round4(s as f64 / total as f64)
        }
    })
}

/// The report of a built scenario.
#[allow(clippy::too_many_lines)]
pub(crate) fn report(inputs: &Inputs, sc: &Scenario) -> Report {
    let pop = &sc.population;
    let st = &sc.stages;
    let hh_w = &pop.households.hh_weight;
    let pw: Vec<i128> = pop
        .persons
        .household_id
        .iter()
        .map(|&h| i128::from(hh_w[h as usize]))
        .collect();
    let (n, n_hh) = (pw.len(), hh_w.len());
    let by_person = |column: &[u64]| -> i128 {
        pw.iter()
            .zip(column)
            .map(|(&w, &v)| w * i128::from(v))
            .sum()
    };
    let docs = &inputs.docs;
    let mut checks = Vec::new();
    let mut check = |name: &'static str, ok: bool, facts: Value| {
        checks.push(Check {
            name,
            ok,
            known_gap: None,
            facts,
        });
    };

    // 1. People and households are the census's.
    let persons: i128 = pw.iter().sum();
    check(
        "persons",
        persons == CENSUS_PERSONS,
        json!({"weighted_persons": persons as i64, "synthetic_persons": n, "synthetic_households": n_hh}),
    );

    // 2. Monthly incomes of households against the national accounts.
    let wages = by_person(&st.wages.wage);
    let pensions = by_person(&st.pensions.pension);
    let inc = &st.incomes;
    let self_emp = by_person(&inc.self_employment_income);
    let informal = by_person(&inc.informal_income);
    let in_kind = weighted(hh_w, &inc.hh_subsistence_income);
    let earnings = &inputs.stages.wages.margins;
    let want_wages: i128 = earnings
        .wage_bill
        .iter()
        .map(|&v| i128::from(v))
        .sum::<i128>()
        * 100;
    let near = |got: i128, want: i128, base: i128| (got - want).abs() as f64 <= 1e-4 * base as f64;
    check(
        "wage_bill",
        near(wages, want_wages, want_wages),
        json!({"got": lei(wages), "want": lei(want_wages)}),
    );
    let want_pensions = i128::from(inputs.stages.pensions.margins.pension_bill) * 100;
    check(
        "pension_bill",
        near(pensions, want_pensions, pensions),
        json!({"got": lei(pensions), "want": lei(want_pensions)}),
    );
    let mixed = self_emp + informal + in_kind;
    let want_mixed = int(&docs["other_incomes2021_ro.json"]["mixed_income"]) * 100;
    check(
        "mixed_income",
        near(mixed, want_mixed, mixed),
        json!({"got": lei(mixed), "want": lei(want_mixed)}),
    );
    let declared_wages = by_person(&inc.wage_declared);
    let income_month = wages + pensions + mixed;

    // 3. The wage bill against the input-output table's compensation, by industry group.
    let io = &docs[IO];
    let group_names: Vec<&str> = io["industry_groups"]
        .as_array()
        .expect("checked input")
        .iter()
        .map(|g| g.as_str().expect("a group name"))
        .collect();
    let mut comp = vec![0i128; group_names.len()];
    for (g, c) in io["industry_group_of"]
        .as_array()
        .expect("checked input")
        .iter()
        .zip(io["compensation"].as_array().expect("checked input"))
    {
        comp[usize::try_from(g.as_u64().expect("a group")).expect("small")] += int(c);
    }
    let (status, group) = (&st.jobs.employment_status, &st.jobs.industry_group);
    let mut ratios = Map::new();
    let mut all_in_band = true;
    for (g, name) in group_names.iter().enumerate() {
        let paid: i128 = (0..n)
            .filter(|&i| status[i] == KIND_EMPLOYEE && usize::from(group[i]) == g)
            .map(|i| pw[i] * i128::from(st.wages.wage[i]))
            .sum();
        let r = (paid * 12) as f64 / 100.0 / comp[g] as f64;
        all_in_band &= (0.85..=1.25).contains(&r);
        ratios.insert((*name).to_string(), json!(r));
    }
    check(
        "wages_against_input_output_table",
        all_in_band,
        json!({"year_of_population_wages_over_table_compensation": ratios,
               "note": "1.0 would mean a month of 2021-Q4 times twelve equals the year 2021 including employers' contributions"}),
    );

    // 4. Household wealth against the sector balance sheets.
    let sheets = &docs[SHEETS];
    let w = &st.wealth;
    let (cash, deposits) = (weighted(hh_w, &w.hh_cash), weighted(hh_w, &w.hh_deposits));
    let loans = weighted(hh_w, &w.hh_mortgage) + weighted(hh_w, &w.hh_consumer_debt);
    let want_cash = int(&sheets["assets"]["currency"]["households"]) * 100;
    let want_deposits = int(&sheets["assets"]["deposits"]["households"]) * 100;
    let want_loans = int(&sheets["liabilities"]["loans"]["households"]) * 100;
    check(
        "household_balance_sheet",
        near(cash, want_cash, want_cash)
            && near(deposits, want_deposits, want_deposits)
            && near(loans, want_loans, loans),
        json!({"cash": lei(cash), "deposits": lei(deposits), "loans": lei(loans),
               "dwellings": lei(weighted(hh_w, &w.hh_dwelling_value))}),
    );

    // 5. Deposits: what households and firms hold against what banks owe.
    let held = int(&sheets["assets"]["deposits"]["households"])
        + int(&sheets["assets"]["deposits"]["firms"]);
    let owed = int(&sheets["liabilities"]["deposits"]["banks"]);
    let firm_deposits: i128 = docs[FIRM_SHEETS]["units"]
        .as_array()
        .expect("checked input")
        .iter()
        .map(|u| int(&u["deposits"]))
        .sum();
    check(
        "deposits",
        firm_deposits == int(&sheets["assets"]["deposits"]["firms"]) && held <= owed,
        json!({"households_and_firms": held as i64, "owed_by_banks": owed as i64}),
    );

    // 6. Every employed person has an employer of their industry group.
    let firms = &docs[FIRMS];
    let units = &sc.firm_units;
    let employer = &st.employers.employer;
    let placed = (0..n).all(|i| {
        let e = employer[i];
        (e == NO_EMPLOYER) == (status[i] == KIND_NONE)
            && (e == NO_EMPLOYER || units.industry_group.get(usize::from(e)) == Some(&group[i]))
    });
    let mut staffed: Vec<u16> = employer
        .iter()
        .copied()
        .filter(|&e| e != NO_EMPLOYER)
        .collect();
    staffed.sort_unstable();
    staffed.dedup();
    let employees: i128 = (0..n)
        .filter(|&i| status[i] == KIND_EMPLOYEE)
        .map(|i| pw[i])
        .sum();
    check(
        "employers",
        placed,
        json!({"units_with_workers": staffed.len(), "units": units.unit_id.len(),
               "employees": employees as i64,
               "persons_employed_in_firm_table": firms["totals"]["persons_employed"]}),
    );

    // 7. Firm units against the input-output table.
    let (out_firms, out_table) = (
        int(&firms["totals"]["output"]),
        int(&io["totals"]["output"]),
    );
    check(
        "firm_units_against_input_output_table",
        near(out_firms, out_table, out_table),
        json!({"output": out_firms as i64, "table": out_table as i64}),
    );

    // 8. A year of household income against consumption in the table.
    let consumption = int(&io["totals"]["household_consumption"]);
    let ratio = (income_month * 12) as f64 / 100.0 / consumption as f64;
    check(
        "income_against_consumption",
        (0.8..=1.4).contains(&ratio),
        json!({"income_a_year": lei(income_month * 12), "consumption_in_table": consumption as i64,
               "ratio": ratio,
               "note": "gross money and in-kind income of a month of 2021-Q4, times twelve, over household consumption of 2021"}),
    );

    // 9. Household income by fifth of households, against the survey's shape.
    let mut income = vec![0i128; n_hh];
    for (i, &h) in pop.persons.household_id.iter().enumerate() {
        income[h as usize] += i128::from(st.wages.wage[i])
            + i128::from(st.pensions.pension[i])
            + i128::from(inc.self_employment_income[i])
            + i128::from(inc.informal_income[i]);
    }
    for (v, &s) in income.iter_mut().zip(&inc.hh_subsistence_income) {
        *v += i128::from(s);
    }
    let private: Vec<usize> = (0..n_hh)
        .filter(|&h| !pop.households.hh_collective[h])
        .collect();
    let shares = quintile_shares(
        private
            .iter()
            .map(|&h| (income[h], i128::from(hh_w[h]), income[h]))
            .collect(),
    );
    let survey_total: f64 = INCOME_BY_FIFTH_2020.iter().sum();
    let survey = INCOME_BY_FIFTH_2020.map(|v| v / survey_total);
    let private_w: i128 = private.iter().map(|&h| i128::from(hh_w[h])).sum();
    let without: i128 = private
        .iter()
        .filter(|&&h| income[h] == 0)
        .map(|&h| i128::from(hh_w[h]))
        .sum();
    check(
        "income_distribution",
        shares
            .iter()
            .zip(&survey)
            .all(|(a, b)| (a - b).abs() <= 0.08),
        json!({"share_of_income_by_fifth": shares, "survey_2020_disposable": survey.map(round4),
               "households_without_income": round4(without as f64 / private_w as f64),
               "note": "gross income here, disposable income in the survey: taxes and benefits are not applied yet"}),
    );

    // 10. The declared economy.
    let declared_share = declared_wages as f64 / wages as f64;
    check(
        "declared_wages",
        (0.8..=0.97).contains(&declared_share),
        json!({"declared_share": declared_share}),
    );

    // 11 to 14. The opening ledger and the tables against it (I-4).
    let ledger = &sc.ledger;
    let mut rows = Map::new();
    for (i, row) in Instrument::ALL.into_iter().zip(&sc.opening.balances) {
        let sum: i128 = row.iter().map(|b| i128::from(b.get())).sum();
        if sum != 0 {
            rows.insert(format!("{i:?}"), json!(sum as i64));
        }
    }
    check(
        "ledger_invariants",
        rows.is_empty() && ledger.check_invariants().is_ok(),
        json!({"rows_not_zero": rows}),
    );
    let in_ledger = |s: Sector, i: Instrument| i128::from(ledger.balance(s, i).get());
    let (l_cash, l_deposits) = (
        in_ledger(Sector::Households, Instrument::Cash),
        in_ledger(Sector::Households, Instrument::Deposits),
    );
    check(
        "households_against_ledger",
        cash == l_cash && deposits == l_deposits,
        json!({"cash": lei(l_cash), "deposits": lei(l_deposits)}),
    );
    let sum = |column: &[u64]| column.iter().map(|&v| i128::from(v)).sum::<i128>();
    let (f_cash, f_deposits) = (
        in_ledger(Sector::Firms, Instrument::Cash),
        in_ledger(Sector::Firms, Instrument::Deposits),
    );
    check(
        "firms_against_ledger",
        sum(&units.currency) == f_cash && sum(&units.deposits) == f_deposits,
        json!({"cash": lei(f_cash), "deposits": lei(f_deposits)}),
    );
    let mut left_out = Map::new();
    for (k, row) in StatInstrument::LEFT_OUT.iter().zip(&sc.opening.left_out) {
        let by_sector: Map<String, Value> = SECTOR_KEYS
            .iter()
            .zip(row)
            .map(|(s, b)| ((*s).to_string(), json!(lei(i128::from(b.get())))))
            .collect();
        left_out.insert(k.key().to_string(), Value::Object(by_sector));
    }
    check("left_out", true, Value::Object(left_out));

    for c in &mut checks {
        if c.name == "income_distribution" {
            c.known_gap = Some(INCOME_GAP);
        }
    }
    Report { checks }
}

#[cfg(test)]
// The shares are parsed from four-decimal text, so they compare exactly.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn lei_rounds_half_away_from_zero() {
        assert_eq!(lei(150), 2);
        assert_eq!(lei(149), 1);
        assert_eq!(lei(-150), -2);
        assert_eq!(lei(-149), -1);
    }

    #[test]
    fn fifths_split_a_record_that_straddles_them() {
        // Five records of weight 1: each fifth holds one.
        let s = quintile_shares((1..=5).map(|v| (v, 1, v)).collect());
        assert_eq!(s, [0.0667, 0.1333, 0.2, 0.2667, 0.3333]);
        // One record of weight 5 is spread over every fifth.
        assert_eq!(quintile_shares(vec![(3, 5, 3)]), [0.2; 5]);
    }
}
