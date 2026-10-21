//! [`World::new`]: the starting state as column tables, the opening ledger
//! checked against them (I-4 at day 0), the mechanics' starting values
//! ("Starting the clock"), then the edits.

use econ_ledger::{Instrument, Ledger, Sector};
use econ_mech_prices::{daily_share_ppm, unit_cost_ppm};
use econ_popgen::{EduLevel, NO_EMPLOYER};
use econ_rng::KeyedRng;
use econ_types::{Bani, Calendar, Date, Day};

use super::money::{bani, post_opening, ratio};
use super::{
    DayDemand, FirmUnitColumns, HouseholdColumns, HouseholdState, IndustryColumns, IndustryState,
    Law, MacroState, PersonColumns, PersonState, StartEdit, StartState, UnitState, World,
    WorldError, WorldParams,
};
use crate::schedule::{Staggered, stagger_day};

/// The scenario's start date.
const START: (i32, u8, u8) = (2021, 12, 1);
/// The inflation target, millionths a year (2.5%).
const INFLATION_TARGET_PPM: i32 = 25_000;
/// `starting_utilisation`: capacity is starting output over this.
const STARTING_UTILISATION_PPM: i128 = 800_000;
/// `inventory_target_ratio`: a month for storable goods, a quarter for the rest.
const TARGET_GOODS_PPM: u32 = 1_000_000;
const TARGET_SERVICES_PPM: u32 = 250_000;
/// Stage C's activity code of the unemployed.
const ACTIVITY_UNEMPLOYED: u8 = 4;
/// Status in employment of an employee.
pub(super) const EMPLOYEE: u8 = 1;
const ONE: i128 = 1_000_000;

/// The skill tier an occupation belongs to (high 0: ISCO 1 to 3; mid 1: 4 to
/// 8; low 2: 9); `None` for the armed forces (0) and no occupation.
pub(super) fn tier(occupation: u8) -> Option<usize> {
    match occupation {
        1..=3 => Some(0),
        4..=8 => Some(1),
        9 => Some(2),
        _ => None,
    }
}

/// Whether an education qualifies for a tier.
pub(super) fn qualifies(t: usize, edu: EduLevel) -> bool {
    match t {
        0 => edu >= EduLevel::Isced5To6,
        1 => edu >= EduLevel::Isced3,
        _ => true,
    }
}

/// The weighted median of (value, weight) pairs; `None` if empty.
fn median(mut pairs: Vec<(Bani, u64)>) -> Option<Bani> {
    pairs.sort_unstable();
    let total: u64 = pairs.iter().map(|p| p.1).sum();
    let mut seen = 0u64;
    for (v, w) in pairs {
        seen += w;
        if seen.saturating_mul(2) >= total {
            return Some(v);
        }
    }
    None
}

fn shape(ok: bool, what: &'static str) -> Result<(), WorldError> {
    if ok {
        Ok(())
    } else {
        Err(WorldError::Shape(what))
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn new(start: StartState, law: Law, params: WorldParams) -> Result<World, WorldError> {
    let s = start;
    let (n_hh, n_p, n_u, n_i) = (
        s.hh_weight.len(),
        s.household_id.len(),
        s.unit_industry.len(),
        s.industry_id.len(),
    );
    for len in [
        s.hh_county.len(),
        s.hh_collective.len(),
        s.hh_region.len(),
        s.hh_cash.len(),
        s.hh_deposits.len(),
        s.hh_subsistence_income.len(),
    ] {
        shape(len == n_hh, "the household columns differ in length")?;
    }
    for len in [
        s.age.len(),
        s.edu_level.len(),
        s.activity.len(),
        s.employment_status.len(),
        s.occupation.len(),
        s.wage.len(),
        s.wage_declared.len(),
        s.pension.len(),
        s.other_income.len(),
        s.self_employment_declared.len(),
        s.employer.len(),
    ] {
        shape(len == n_p, "the person columns differ in length")?;
    }
    for len in [
        s.unit_cohort.len(),
        s.unit_size.len(),
        s.unit_firm_count.len(),
        s.unit_domestic_cohort.len(),
        s.unit_output.len(),
        s.unit_value_added.len(),
        s.unit_wages.len(),
        s.unit_currency.len(),
        s.unit_deposits.len(),
    ] {
        shape(len == n_u, "the firm-unit columns differ in length")?;
    }
    for len in [
        s.industry_group.len(),
        s.goods.len(),
        s.flexibility_ppm.len(),
        s.home_coefficient_ppm.len(),
        s.import_coefficient_ppm.len(),
        s.other_cost_ppm.len(),
        s.basket_ppm.len(),
        s.basket_import_ppm.len(),
        s.gov_domestic.len(),
        s.gov_imported.len(),
        s.investment_domestic.len(),
        s.investment_imported.len(),
        s.exports.len(),
        law.vat_category.len(),
    ] {
        shape(len == n_i, "the industry columns differ in length")?;
    }
    shape(
        s.home_coefficient_ppm.iter().all(|r| r.len() == n_i),
        "the input coefficients are not square",
    )?;
    shape(
        s.household_id.iter().all(|&h| (h as usize) < n_hh),
        "a person of no household",
    )?;
    shape(
        s.unit_industry.iter().all(|&j| usize::from(j) < n_i),
        "a unit of no industry",
    )?;
    shape(
        s.employer
            .iter()
            .all(|&e| e == NO_EMPLOYER || usize::from(e) < n_u),
        "a person employed by no unit",
    )?;
    let groups = 1 + usize::from(s.industry_group.iter().copied().max().unwrap_or(0));

    // The opening ledger, and the tables against it (I-4 at day 0).
    let mut ledger = Ledger::new();
    post_opening(&mut ledger, &s.balances)?;
    ledger.allow_negative(Sector::Government, Instrument::GovDepositsAtCb);
    let weighted = |col: &[Bani]| -> i128 {
        s.hh_weight
            .iter()
            .zip(col)
            .map(|(&w, v)| i128::from(w) * i128::from(v.get()))
            .sum()
    };
    let plain = |col: &[Bani]| -> i128 { col.iter().map(|v| i128::from(v.get())).sum() };
    let in_ledger = |sector, instrument| i128::from(ledger.balance(sector, instrument).get());
    for (records, sector, instrument, what) in [
        (
            weighted(&s.hh_cash),
            Sector::Households,
            Instrument::Cash,
            "households' cash",
        ),
        (
            weighted(&s.hh_deposits),
            Sector::Households,
            Instrument::Deposits,
            "households' deposits",
        ),
        (
            plain(&s.unit_currency),
            Sector::Firms,
            Instrument::Cash,
            "firm units' currency",
        ),
        (
            plain(&s.unit_deposits),
            Sector::Firms,
            Instrument::Deposits,
            "firm units' deposits",
        ),
    ] {
        if records != in_ledger(sector, instrument) {
            return Err(WorldError::TablesDisagreeWithLedger { what });
        }
    }
    ledger.begin_tick(0);

    let seed = KeyedRng::new(params.rng_seed);
    let (y, m, d) = START;
    let calendar = Calendar::new(Date::new(y, m, d).expect("the start date exists"));
    let minimum = law.minimum_wage;

    // Persons: employees keep their unit and their wage, not below the minimum.
    let mut employer = vec![NO_EMPLOYER; n_p];
    let mut wage = vec![Bani::ZERO; n_p];
    let mut wage_declared = vec![Bani::ZERO; n_p];
    let mut seeking = vec![false; n_p];
    for i in 0..n_p {
        if s.employment_status[i] == EMPLOYEE && s.employer[i] != NO_EMPLOYER {
            employer[i] = s.employer[i];
            let w = s.wage[i].max(minimum);
            wage[i] = w;
            wage_declared[i] = (s.wage_declared[i] + (w - s.wage[i])).min(w);
        }
        seeking[i] = s.activity[i] == ACTIVITY_UNEMPLOYED;
    }
    let region: Vec<u8> = s
        .household_id
        .iter()
        .map(|&h| s.hh_region[h as usize])
        .collect();
    let pw: Vec<u64> = s
        .household_id
        .iter()
        .map(|&h| u64::from(s.hh_weight[h as usize]))
        .collect();

    // Households: equivalised adults, the month's income, the fifths.
    let mut eq = vec![0u32; n_hh];
    let mut adults = vec![0u32; n_hh];
    let mut income = vec![Bani::ZERO; n_hh];
    for i in 0..n_p {
        let h = s.household_id[i] as usize;
        if s.age[i] >= 14 * 12 {
            eq[h] += if adults[h] == 0 { 1_000 } else { 500 };
            adults[h] += 1;
        } else {
            eq[h] += 300;
        }
        income[h] += wage[i] + s.pension[i] + s.other_income[i];
    }
    let per_adult: Vec<i128> = (0..n_hh)
        .map(|h| i128::from(income[h].get()) * 1_000 / i128::from(eq[h].max(1)))
        .collect();
    let (fifth, bounds) = super::close::fifths(&per_adult, &s.hh_weight);

    let households = HouseholdColumns {
        hh_weight: s.hh_weight.clone(),
        hh_cash: s.hh_cash.clone(),
        hh_deposits: s.hh_deposits.clone(),
        hh_income_received: vec![Bani::ZERO; n_hh],
        hh_stagger_day: (0..n_hh)
            .map(|h| stagger_day(seed, Staggered::Household, h as u64))
            .collect(),
        hh_hardship: vec![false; n_hh],
        hh_fifth: fifth,
        last_decision: vec![None; n_hh],
        income_to_date: vec![Bani::ZERO; n_hh],
        spending_to_date: vec![Bani::ZERO; n_hh],
    };
    let hh = HouseholdState {
        eq_adults_milli: eq,
        subsistence: s.hh_subsistence_income.clone(),
        income_this_month: vec![Bani::ZERO; n_hh],
        income_last_month: income,
    };

    // Firm units.
    let mut members = vec![Vec::new(); n_u];
    let mut staff = vec![0u64; n_u];
    let mut start_mix = vec![[0u64; 10]; n_u];
    let mut unit_wages = vec![0i128; n_u];
    for i in 0..n_p {
        if employer[i] != NO_EMPLOYER {
            let u = usize::from(employer[i]);
            members[u].push(u32::try_from(i).expect("a person id"));
            staff[u] += pw[i];
            if let Some(o) = start_mix[u].get_mut(usize::from(s.occupation[i])) {
                *o += pw[i];
            }
            unit_wages[u] += i128::from(pw[i]) * i128::from(wage[i].get());
        }
    }
    let monthly: Vec<i64> = s
        .unit_output
        .iter()
        .map(|o| i64::try_from(ratio(i128::from(o.get()), 1, 12)).expect("a month's output"))
        .collect();
    let target_of = |j: usize| {
        if s.goods[j] {
            TARGET_GOODS_PPM
        } else {
            TARGET_SERVICES_PPM
        }
    };
    let inventory_target_ppm: Vec<u32> = s
        .unit_industry
        .iter()
        .map(|&j| target_of(usize::from(j)))
        .collect();
    let mut inventory: Vec<i64> = (0..n_u)
        .map(|u| {
            i64::try_from(ratio(
                i128::from(monthly[u]),
                i128::from(inventory_target_ppm[u]),
                ONE,
            ))
            .expect("an inventory")
        })
        .collect();
    let mut capacity: Vec<i64> = monthly
        .iter()
        .map(|&y| {
            i64::try_from(ratio(i128::from(y), ONE, STARTING_UTILISATION_PPM)).expect("a capacity")
        })
        .collect();
    // The plan of November, made on each unit's day and running until its day in December.
    let stagger: Vec<u8> = (0..n_u)
        .map(|u| stagger_day(seed, Staggered::FirmUnit, u as u64))
        .collect();
    let mut plan_output = monthly.clone();
    let plan_days = vec![30u8; n_u];
    let plan_done: Vec<u8> = stagger
        .iter()
        .map(|&d| 31u8.saturating_sub(d).min(30))
        .collect();

    // Industries: unit costs at starting prices, and the markups that make
    // every target price 1.
    let mut ind_wages = vec![0i128; n_i];
    let mut ind_output = vec![0i128; n_i];
    for u in 0..n_u {
        let j = usize::from(s.unit_industry[u]);
        ind_wages[j] += unit_wages[u];
        ind_output[j] += i128::from(monthly[u]);
    }
    let inputs: Vec<Vec<(usize, u32)>> = s
        .home_coefficient_ppm
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter(|&(_, &a)| a > 0)
                .map(|(i, &a)| (i, a))
                .collect()
        })
        .collect();
    let wage_cost_ppm: Vec<u32> = (0..n_i)
        .map(|j| {
            if ind_output[j] > 0 {
                u32::try_from(ratio(ind_wages[j], ONE, ind_output[j]).min(i128::from(u32::MAX)))
                    .expect("small")
            } else {
                0
            }
        })
        .collect();
    let price = 1_000_000u32;
    let markup0_ppm: Vec<i32> = (0..n_i)
        .map(|j| {
            let home: Vec<(u32, u32)> = inputs[j].iter().map(|&(_, a)| (a, price)).collect();
            let uc = unit_cost_ppm(
                &home,
                s.import_coefficient_ppm[j],
                wage_cost_ppm[j],
                s.other_cost_ppm[j],
            );
            if uc == 0 {
                0
            } else {
                i32::try_from(ratio(ONE, ONE, i128::from(uc)) - ONE).unwrap_or(i32::MAX)
            }
        })
        .collect();
    let day_part = |v: &[Bani]| -> Vec<i64> {
        v.iter()
            .map(|b| i64::try_from(i128::from(b.get()) / 365).expect("a day's demand"))
            .collect()
    };
    let day_money =
        |v: &[Bani]| -> Vec<Bani> { v.iter().map(|b| bani(i128::from(b.get()) / 365)).collect() };
    let mut basket = s.basket_ppm.clone();

    // Reference wages: the median wage of the employees of each group and
    // occupation, then of the group, then of everyone.
    let unit_group: Vec<usize> = s
        .unit_industry
        .iter()
        .map(|&j| usize::from(s.industry_group[usize::from(j)]))
        .collect();
    let mut cells: Vec<Vec<Vec<(Bani, u64)>>> = vec![vec![Vec::new(); 10]; groups];
    for i in 0..n_p {
        if employer[i] != NO_EMPLOYER {
            let g = unit_group[usize::from(employer[i])];
            if let Some(c) = cells[g].get_mut(usize::from(s.occupation[i])) {
                c.push((wage[i], pw[i]));
            }
        }
    }
    let national = median(cells.iter().flatten().flatten().copied().collect()).unwrap_or(minimum);
    let mut reference_wage = vec![[minimum; 10]; groups];
    for g in 0..groups {
        let group_median = median(cells[g].iter().flatten().copied().collect()).unwrap_or(national);
        for o in 0..10 {
            reference_wage[g][o] = median(cells[g][o].clone())
                .unwrap_or(group_median)
                .max(minimum);
        }
    }

    // Seekers by tier, and the tightness of normal turnover by group.
    let mut tier_seekers = [0u64; 3];
    for i in 0..n_p {
        if seeking[i] {
            for (t, n) in tier_seekers.iter_mut().enumerate() {
                if qualifies(t, s.edu_level[i]) {
                    *n += pw[i];
                }
            }
        }
    }
    let mut group_tiers = vec![[0u64; 3]; groups];
    for i in 0..n_p {
        if employer[i] != NO_EMPLOYER
            && let Some(t) = tier(s.occupation[i])
        {
            group_tiers[unit_group[usize::from(employer[i])]][t] += pw[i];
        }
    }
    let quit = i128::from(params.labour.quit_rate_ppm);
    let normal_tightness: Vec<u32> = group_tiers
        .iter()
        .map(|mix| {
            let employees: i128 = mix.iter().map(|&v| i128::from(v)).sum();
            let vacancies = ratio(employees, quit, ONE);
            super::close::tightness_ppm(vacancies, mix, &tier_seekers)
        })
        .collect();

    let mut uu = UnitState {
        plan_output: Vec::new(),
        plan_days,
        plan_done,
        inputs_served_ppm: vec![1_000_000; n_u],
        input_stock: Vec::new(),
        members,
        start_mix,
        hire_carry: vec![[0; 3]; n_u],
        vacancies_month: vec![[0; 3]; n_u],
        mixed_weight: (0..n_u)
            .map(|u| {
                if s.unit_domestic_cohort[u] {
                    u64::try_from((s.unit_value_added[u] - s.unit_wages[u]).get().max(0))
                        .expect("non-negative")
                } else {
                    0
                }
            })
            .collect(),
        invest_weight: (0..n_u)
            .map(|u| {
                if s.unit_cohort[u] {
                    u64::try_from((s.unit_value_added[u] - s.unit_wages[u]).get().max(0))
                        .expect("non-negative")
                } else {
                    0
                }
            })
            .collect(),
        public: s.unit_cohort.iter().map(|&c| !c).collect(),
        wage_reserve: vec![0; n_u],
        owes: vec![Bani::ZERO; n_u],
        lent: vec![Bani::ZERO; n_u],
        staff0: staff.clone(),
        output0: monthly.clone(),
        labour_factor_ppm: 1_000_000,
    };
    let mut wage_scale: Option<u32> = None;
    let mut transfer = Vec::new();
    #[cfg_attr(not(feature = "pay_day"), allow(unused_mut))]
    let mut layoffs: Vec<(usize, u32)> = Vec::new();
    #[cfg_attr(not(feature = "trade"), allow(unused_mut, unused_variables))]
    let mut world_factor: u32 = 1_000_000;
    // Edits applied once the world exists, in order.
    #[cfg_attr(not(feature = "banking"), allow(unused_mut))]
    let mut later: Vec<StartEdit> = Vec::new();

    // The edits, in order.
    for e in &s.edits {
        match e {
            StartEdit::Transfer(v) => {
                shape(
                    v.len() == n_hh,
                    "a transfer of another number of households",
                )?;
                transfer.clone_from(v);
            }
            StartEdit::ShutIndustry(j) => {
                shape(*j < n_i, "an industry that does not exist")?;
                for u in 0..n_u {
                    if usize::from(s.unit_industry[u]) == *j {
                        inventory[u] = 0;
                        capacity[u] = 0;
                        plan_output[u] = 0;
                    }
                }
            }
            StartEdit::ScaleInventory {
                industry,
                factor_ppm,
            } => {
                shape(*industry < n_i, "an industry that does not exist")?;
                for u in 0..n_u {
                    if usize::from(s.unit_industry[u]) == *industry {
                        inventory[u] = i64::try_from(ratio(
                            i128::from(inventory[u]),
                            i128::from(*factor_ppm),
                            ONE,
                        ))
                        .expect("an inventory");
                    }
                }
            }
            StartEdit::ScaleLabourRequirement { factor_ppm } => {
                uu.labour_factor_ppm = *factor_ppm;
            }
            StartEdit::BasketShares(v) => {
                shape(v.len() == n_i, "basket shares of another number of goods")?;
                shape(
                    v.iter().map(|&x| u64::from(x)).sum::<u64>() == 1_000_000,
                    "basket shares that do not sum to one",
                )?;
                basket.clone_from(v);
            }
            StartEdit::ScaleWages { factor_ppm } => wage_scale = Some(*factor_ppm),
            #[cfg(feature = "pay_day")]
            StartEdit::Layoff {
                industry,
                share_ppm,
            } => {
                shape(*industry < n_i, "an industry that does not exist")?;
                layoffs.push((*industry, *share_ppm));
            }
            #[cfg(feature = "banking")]
            StartEdit::PolicyRate { .. } => later.push(e.clone()),
            #[cfg(feature = "banking")]
            StartEdit::ExtraLoan { unit, .. } => {
                shape(*unit < n_u, "a firm unit that does not exist")?;
                later.push(e.clone());
            }
            #[cfg(feature = "monetary")]
            StartEdit::CentralBankFinancing { .. } => later.push(e.clone()),
            #[cfg(feature = "trade")]
            StartEdit::WorldPrices { factor_ppm } => world_factor = *factor_ppm,
            #[cfg(feature = "revenue")]
            StartEdit::WithoutRevenue(_) => {
                unimplemented!("test writer's addition (ADR-0014): the implementation applies it")
            }
        }
    }
    if let Some(f) = wage_scale {
        let scale = |b: Bani| b.mul_ratio(i64::from(f), 1_000_000);
        for i in 0..n_p {
            wage[i] = scale(wage[i]);
            wage_declared[i] = scale(wage_declared[i]);
        }
        for row in &mut reference_wage {
            for w in row.iter_mut() {
                *w = scale(*w);
            }
        }
    }
    uu.plan_output = plan_output;

    let desired: Vec<u64> = (0..n_u)
        .map(|u| super::goods::desired(&uu, u, uu.plan_output[u]))
        .collect();
    // What each unit sold since its plan of November, at the starting pace:
    // so that its first plan sees a normal month.
    let sales_since_plan: Vec<i64> = (0..n_u)
        .map(|u| {
            i64::try_from(ratio(
                i128::from(monthly[u]),
                i128::from(uu.plan_done[u]),
                30,
            ))
            .expect("sales")
        })
        .collect();
    let units = FirmUnitColumns {
        industry: s.unit_industry.clone(),
        unit_stagger_day: stagger,
        staff,
        desired_employment: desired,
        expected_demand: monthly,
        capacity,
        inventory,
        inventory_target_ppm,
        sales_since_plan,
        output_today: vec![0; n_u],
        sales_today: vec![0; n_u],
        revenue_today: vec![Bani::ZERO; n_u],
        currency: s.unit_currency.clone(),
        deposits: s.unit_deposits.clone(),
        last_plan: vec![None; n_u],
        last_staffing: vec![None; n_u],
    };
    let industries = IndustryColumns {
        id: s.industry_id.clone(),
        group: s.industry_group.clone(),
        basket_share_ppm: basket,
        price_ppm: vec![price; n_i],
        consumer_price_ppm: vec![price; n_i],
        demand_today: vec![0; n_i],
        shortage_today: vec![0; n_i],
        sales_value_today: vec![Bani::ZERO; n_i],
    };
    let essential: Vec<Vec<usize>> = inputs
        .iter()
        .map(|row| {
            row.iter()
                .filter(|&&(_, a)| a >= super::goods::ESSENTIAL_INPUT_PPM)
                .map(|&(i, _)| i)
                .collect()
        })
        .collect();
    // Each unit starts with its target stock of its essential inputs.
    let input_stock: Vec<Vec<i64>> = (0..n_u)
        .map(|u| {
            let j = usize::from(s.unit_industry[u]);
            let day = i128::from(uu.plan_output[u]) / 30;
            essential[j]
                .iter()
                .map(|&i| {
                    let a = inputs[j]
                        .iter()
                        .find(|&&(k, _)| k == i)
                        .map_or(0, |&(_, a)| a);
                    i64::try_from(
                        day * i128::from(a) / ONE * i128::from(super::goods::INPUT_STOCK_DAYS),
                    )
                    .expect("a stock")
                })
                .collect()
        })
        .collect();
    uu.input_stock = input_stock;
    let essential_ppm = (0..n_i)
        .map(|j| {
            inputs[j]
                .iter()
                .filter(|&&(_, a)| a >= super::goods::ESSENTIAL_INPUT_PPM)
                .map(|&(_, a)| a)
                .collect()
        })
        .collect();
    let essential_slot = (0..n_i)
        .map(|j| {
            inputs[j]
                .iter()
                .map(|(i, _)| essential[j].binary_search(i).ok())
                .collect()
        })
        .collect();
    let ii = IndustryState {
        essential,
        essential_ppm,
        essential_slot,
        inputs,
        import_ppm: s.import_coefficient_ppm.clone(),
        other_ppm: s.other_cost_ppm.clone(),
        markup0_ppm,
        flex_daily_ppm: s
            .flexibility_ppm
            .iter()
            .map(|&f| daily_share_ppm(f))
            .collect(),
        wage_cost_ppm,
        basket_import_ppm: s.basket_import_ppm.clone(),
        gov_q: day_part(&s.gov_domestic),
        inv_q: day_part(&s.investment_domestic),
        exp_q: day_part(&s.exports),
        gov_imp: day_money(&s.gov_imported),
        inv_imp: day_money(&s.investment_imported),
        output_month: vec![0; n_i],
        wages_month: vec![Bani::ZERO; n_i],
        shortage_month: vec![0; n_i],
    };
    let persons = PersonColumns {
        household_id: s.household_id.clone(),
        edu_level: s.edu_level.clone(),
        employer,
        occupation: s.occupation.clone(),
        wage,
        wage_declared,
        months_unemployed: vec![0; n_p],
        hired_on: vec![None; n_p],
        last_pay: vec![Bani::ZERO; n_p],
    };
    let pp = PersonState {
        status: s.employment_status.clone(),
        seeking,
        region,
        pension: s.pension.clone(),
        age: s.age.clone(),
        other_income: s.other_income.clone(),
    };
    let mac = MacroState {
        expected_inflation_ppm: INFLATION_TARGET_PPM,
        target_ppm: INFLATION_TARGET_PPM,
        cpi: Vec::new(),
        fifth_bounds: bounds,
        tier_seekers,
        tier_vacancies: [0; 3],
        tightness: vec![Vec::new(); groups],
        normal_tightness,
        reference_wage,
        first_consumption: Bani::ZERO,
        last_consumption: Bani::ZERO,
        owed: Vec::new(),
        transfer,
        start_pay: true,
        clearing: Vec::new(),
        month: super::MonthAcc::default(),
    };
    let fs = super::fiscal::FiscalState::new(
        &law,
        seed,
        &s.household_id,
        &s.age,
        &s.edu_level,
        &s.pension,
        &s.self_employment_declared,
        &s.hh_collective,
        &(0..n_u)
            .map(|u| s.unit_cohort[u] && s.unit_size[u] == 0)
            .collect::<Vec<_>>(),
        &s.unit_firm_count,
    );
    let mut world = World {
        #[cfg(feature = "banking")]
        bk: super::banking::Banks::default(),
        #[cfg(feature = "trade")]
        fx: super::foreign::ForeignState::default(),
        #[cfg(feature = "monetary")]
        cb: super::central_bank::CentralBank::default(),
        #[cfg(feature = "investment")]
        inv: super::investment::Investment::default(),
        #[cfg(feature = "revenue")]
        rev: super::revenue::Revenue::default(),
        #[cfg(feature = "fiscal")]
        tr: super::treasury::TreasuryState::default(),
        slips: super::PaySlips::default(),
        fs,
        sample_scale: s.sample_scale,
        law,
        params,
        seed,
        calendar,
        today: Day(0),
        ledger,
        households,
        persons,
        units,
        industries,
        hh,
        pp,
        uu,
        ii,
        mac,
        demand: DayDemand {
            units: vec![Vec::new(); n_i],
            households: Vec::new(),
        },
        months: Vec::new(),
    };
    #[cfg(feature = "fiscal")]
    world.start_treasury(&s.gov_debt_profile, &s.gov_transfers_year);
    #[cfg(feature = "banking")]
    world.start_banks(
        &s.unit_loans,
        &s.hh_mortgage,
        &s.hh_consumer_debt,
        &s.unit_output,
    );
    #[cfg(feature = "trade")]
    world.start_trade(world_factor);
    #[cfg(feature = "revenue")]
    world.start_revenue(&s);
    #[cfg(feature = "monetary")]
    world.start_central_bank();
    #[cfg(feature = "investment")]
    world.start_investment(
        &s.unit_fixed_assets,
        &s.unit_controller,
        &s.unit_ownership_ppm,
        &s.unit_output,
        &s.unit_value_added,
        &s.unit_wages,
        s.gov_investment_year,
    );
    for e in later {
        match e {
            #[cfg(feature = "banking")]
            StartEdit::PolicyRate { from, rate_ppm } => world.bank_policy_change(from, rate_ppm),
            #[cfg(feature = "banking")]
            StartEdit::ExtraLoan { unit, amount } => world.bank_extra_loan(unit, amount),
            #[cfg(feature = "monetary")]
            StartEdit::CentralBankFinancing { months } => world.treasury_cb_financing(months),
            _ => {}
        }
    }
    // Layoffs of a starting edit, before the first day (AC-ST-08).
    for (industry, share_ppm) in layoffs {
        world.lay_off_share(industry, share_ppm);
    }
    Ok(world)
}
