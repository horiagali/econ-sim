//! Stage I — the income split (spec `society/population-incomes`).
//!
//! Splits every wage into a declared and an undeclared part, gives the
//! self-employed their income and its declared part, gives some people
//! informal income outside a job and some rural households what they grow for
//! themselves, so that households receive the mixed income of the national
//! accounts. Every rule about who gets what is an educated guess; the spec
//! says which. Earlier stages are not changed.
//!
//! `python/reference/popgen_reference.py` (`assign_other_incomes`) is the
//! independent reference; the two agree record for record (AC-POPI-08).

use econ_rng::{KeyedRng, Stream};

use crate::Population;
use crate::arith::{Int, UNIT};
use crate::assign::column_hash;
use crate::attributes::{Activity, PersonAttributes};
use crate::housing::HouseholdHousing;
use crate::jobs::{INDUSTRY_GROUPS, PersonJobs};
use crate::wages::{PersonWages, curve_ok, spread_unsplit};
use crate::wealth::{Curve, draw_ppm, mixed_order};

/// `tick` of the draw context of persons.
const STAGE: u32 = 8;
/// `tick` of the draw context of households (what they grow for themselves).
const STAGE_SUBSISTENCE: u32 = 9;
const BANI_PER_LEU: Int = 100;
/// `employment_status` codes.
const KIND_NONE: u8 = 0;
const KIND_EMPLOYEE: u8 = 1;
const KIND_EMPLOYER: u8 = 2;
const KIND_OWN_ACCOUNT: u8 = 3;
const KIND_FAMILY_WORKER: u8 = 4;
/// The industry group of farming.
const AGRICULTURE: u8 = 0;
/// Ages (months) at which a person can have informal income outside a job: `[from, to)`.
const INFORMAL_MONTHS: (u16, u16) = (15 * 12, 75 * 12);

/// How likely each kind of person is to have informal income outside a job,
/// relative to each other, in millionths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformalPropensity {
    /// Unemployed persons.
    pub unemployed: u32,
    /// Other inactive persons.
    pub inactive_other: u32,
    /// Retired persons.
    pub retired: u32,
    /// Pupils and students.
    pub student: u32,
    /// Employers.
    pub employer: u32,
    /// Own-account workers.
    pub own_account: u32,
    /// Family workers.
    pub family_worker: u32,
    /// Employees, `[industry_group]`.
    pub employee_by_industry_group: Vec<u32>,
}

/// Income margins: amounts in lei per month, shares in millionths. Tables by
/// industry group have [`crate::INDUSTRY_GROUPS`] entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeMargins {
    /// Farm income of households, in cash and in kind.
    pub farm_income: u64,
    /// The part of it that households grow and consume themselves.
    pub farm_in_kind: u64,
    /// Income of employers, own-account and family workers outside farming.
    pub self_employment_income: u64,
    /// Informal income outside a job.
    pub informal_income: u64,
    /// Gross minimum wage: what an envelope job declares at least.
    pub minimum_wage: u64,
    /// The earnings curve, for the spread of every amount.
    pub quantile_curve: Curve,
    /// Share of each industry group's wage bill paid outside contracts.
    pub undeclared_wage_share: Vec<u32>,
    /// Of the undeclared wages, the part earned in jobs with no contract.
    pub fully_informal_share: u32,
    /// What an envelope job declares of the true wage, if above the minimum wage.
    pub envelope_declared_share: u32,
    /// How strongly low wages come first among jobs with no contract.
    pub low_wage_link: u32,
    /// Mean income of an employer, an own-account worker and a family worker, as proportions.
    pub rel_kind: [u32; 3],
    /// How closely a self-employed person's place follows education.
    pub self_employment_education_link: u32,
    /// Declared share of self-employment income outside farming.
    pub self_employment_declared: u32,
    /// Declared share of farm income in cash.
    pub farm_declared: u32,
    /// Share of rural private households that grow food for themselves.
    pub subsistence_household_share: u32,
    /// Added to the key of a household with a farm worker.
    pub subsistence_bonus_farm_worker: u32,
    /// Added to the key of a household with a retired member and no farm worker.
    pub subsistence_bonus_retired: u32,
    /// Share of the curve's spread used for what households grow.
    pub subsistence_dispersion: u32,
    /// Share of persons aged 15 to 74 with informal income outside a job.
    pub informal_recipient_share: u32,
    /// Who they are.
    pub informal_propensity: InformalPropensity,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncomeParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Passes of scaling to the proportions between kinds of self-employed.
    pub scaling_passes: u32,
}

impl IncomeParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        IncomeParams {
            rng_seed,
            scaling_passes: 20,
        }
    }
}

/// The columns of the stage; amounts in bani per month.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OtherIncomes {
    /// Per person: the part of the wage that is on the contract (the whole
    /// wage, a part, or 0).
    pub wage_declared: Vec<u64>,
    /// Per person: income as an employer, own-account or family worker.
    pub self_employment_income: Vec<u64>,
    /// Per person: its declared part.
    pub self_employment_declared: Vec<u64>,
    /// Per person: informal income outside a job.
    pub informal_income: Vec<u64>,
    /// Per household: value of what it grows and consumes itself.
    pub hh_subsistence_income: Vec<u64>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncomeError {
    /// No industry groups, or a curve with fewer than two knots.
    EmptyTable,
    /// A table's length does not match its dimensions, a share above 1, a
    /// proportion of zero, farm income in kind above farm income, a curve
    /// that does not ascend from rank 0 to 1,000,000, or columns of another
    /// population.
    ShapeMismatch,
}

/// Split wages and assign the other incomes.
///
/// # Errors
/// See [`IncomeError`].
pub fn assign_other_incomes(
    pop: &Population,
    attrs: &PersonAttributes,
    jobs: &PersonJobs,
    housing: &HouseholdHousing,
    wages: &PersonWages,
    margins: &IncomeMargins,
    params: &IncomeParams,
) -> Result<OtherIncomes, IncomeError> {
    let m = margins;
    check_margins(m)?;
    let (hh, p) = (&pop.households, &pop.persons);
    let (n, n_hh) = (p.household_id.len(), hh.hh_weight.len());
    let (status, group) = (&jobs.employment_status, &jobs.industry_group);
    if attrs.activity.len() != n
        || attrs.edu_level.len() != n
        || status.len() != n
        || group.len() != n
        || wages.wage.len() != n
        || housing.hh_urban.len() != n_hh
        || (0..n).any(|i| status[i] != KIND_NONE && usize::from(group[i]) >= INDUSTRY_GROUPS)
    {
        return Err(IncomeError::ShapeMismatch);
    }
    let pw: Vec<Int> = p
        .household_id
        .iter()
        .map(|&h| Int::from(hh.hh_weight[h as usize]))
        .collect();
    let wage: Vec<Int> = wages.wage.iter().map(|&w| Int::from(w)).collect();
    let mut out = OtherIncomes {
        wage_declared: wages.wage.clone(),
        self_employment_income: vec![0; n],
        self_employment_declared: vec![0; n],
        informal_income: vec![0; n],
        hh_subsistence_income: vec![0; n_hh],
    };
    let (lo, hi) = INFORMAL_MONTHS;
    let of_age = |i: usize| (lo..hi).contains(&p.age[i]);
    // Four keyed draws per working person, in millionths: wage split,
    // self-employment place, informal pick, informal place.
    let rng = KeyedRng::new(params.rng_seed);
    let mut draws: Vec<[Int; 4]> = vec![[0; 4]; n];
    for i in (0..n).filter(|&i| status[i] != KIND_NONE || of_age(i)) {
        let mut d = rng.draw(Stream::PopulationGen, STAGE, i as u64);
        for slot in &mut draws[i] {
            *slot = draw_ppm(d.u64());
        }
    }
    let draws = &draws;
    let draw = |k: usize| move |i: usize| draws[i][k];
    let curve = (
        m.quantile_curve.rank.as_slice(),
        m.quantile_curve.value.as_slice(),
    );
    let passes = params.scaling_passes;
    let bani = |lei: u64| Int::from(lei) * BANI_PER_LEU;
    let amount = |v: Int| u64::try_from(v).expect("amount fits u64");
    let ppm = |v: Int, share: u32| v * Int::from(share) / UNIT;

    // A. The declared part of wages.
    let minimum = bani(m.minimum_wage);
    for g in 0..INDUSTRY_GROUPS {
        let members: Vec<usize> = (0..n)
            .filter(|&i| status[i] == KIND_EMPLOYEE && usize::from(group[i]) == g)
            .collect();
        let bill: Int = members.iter().map(|&i| pw[i] * wage[i]).sum();
        let undeclared = ppm(bill, m.undeclared_wage_share[g]);
        if members.is_empty() || undeclared == 0 {
            continue;
        }
        let order = mixed_order(&members, &pw, &wage, &draw(0), m.low_wage_link, &|_| 0);
        let without_contract = ppm(undeclared, m.fully_informal_share);
        let (mut got, mut k) = (0, 0);
        while k < order.len() && got < without_contract {
            out.wage_declared[order[k]] = 0;
            got += pw[order[k]] * wage[order[k]];
            k += 1;
        }
        let mut rest = order[k..].to_vec();
        rest.sort_by_key(|&i| (draws[i][0], i));
        for i in rest {
            if got >= undeclared {
                break;
            }
            // An envelope needs a wage above the minimum; at or below it the job has no contract.
            let mut declared = minimum.max(ppm(wage[i], m.envelope_declared_share));
            if declared >= wage[i] {
                declared = 0;
            }
            out.wage_declared[i] = amount(declared);
            got += pw[i] * (wage[i] - declared);
        }
    }

    // B and C. Income of the self-employed, outside farming and in it.
    let kind = |k: u8| match k {
        KIND_EMPLOYER => Some(0),
        KIND_OWN_ACCOUNT => Some(1),
        KIND_FAMILY_WORKER => Some(2),
        _ => None,
    };
    let education: Vec<Int> = attrs
        .edu_level
        .iter()
        .map(|&e| Int::from(e as u8))
        .collect();
    let kind_targets: Vec<Int> = m
        .rel_kind
        .iter()
        .chain(&m.rel_kind)
        .map(|&v| Int::from(v))
        .collect();
    let farm_in_cash = bani(m.farm_income) - bani(m.farm_in_kind);
    for (farm, bill, declared_ppm) in [
        (
            false,
            bani(m.self_employment_income),
            m.self_employment_declared,
        ),
        (true, farm_in_cash, m.farm_declared),
    ] {
        let members: Vec<usize> = (0..n)
            .filter(|&i| kind(status[i]).is_some() && (group[i] == AGRICULTURE) == farm)
            .collect();
        if members.is_empty() || bill <= 0 {
            continue;
        }
        let order = mixed_order(
            &members,
            &pw,
            &education,
            &draw(1),
            m.self_employment_education_link,
            &|_| 0,
        );
        let cells = order
            .iter()
            .map(|&i| kind(status[i]).expect("self-employed"))
            .collect();
        let w: Vec<Int> = order.iter().map(|&i| pw[i]).collect();
        let amounts = spread_unsplit(
            &w,
            &[(cells, kind_targets.clone())],
            curve,
            1_000_000,
            passes,
            bill,
        );
        for (&i, a) in order.iter().zip(amounts) {
            out.self_employment_income[i] = amount(a);
            out.self_employment_declared[i] = amount(ppm(a, declared_ppm));
        }
    }

    // D. What rural households grow for themselves.
    let mut farm_worker = vec![false; n_hh];
    let mut retired = vec![false; n_hh];
    for i in 0..n {
        let h = p.household_id[i] as usize;
        farm_worker[h] |= status[i] != KIND_NONE && group[i] == AGRICULTURE;
        retired[h] |= attrs.activity[i] == Activity::Retired;
    }
    let rural: Vec<usize> = (0..n_hh)
        .filter(|&h| !hh.hh_collective[h] && !housing.hh_urban[h])
        .collect();
    if !rural.is_empty() && m.farm_in_kind > 0 {
        let mut pick: Vec<Int> = vec![0; n_hh];
        let mut place: Vec<Int> = vec![0; n_hh];
        for &h in &rural {
            let mut d = rng.draw(Stream::PopulationGen, STAGE_SUBSISTENCE, h as u64);
            pick[h] = draw_ppm(d.u64());
            place[h] = draw_ppm(d.u64());
        }
        let key = |h: usize| {
            pick[h]
                + if farm_worker[h] {
                    Int::from(m.subsistence_bonus_farm_worker)
                } else if retired[h] {
                    Int::from(m.subsistence_bonus_retired)
                } else {
                    0
                }
        };
        let mut ranked = rural.clone();
        ranked.sort_by_cached_key(|&h| (key(h), h));
        let weight: Vec<Int> = hh.hh_weight.iter().map(|&w| Int::from(w)).collect();
        let rural_w: Int = rural.iter().map(|&h| weight[h]).sum();
        let target = ppm(rural_w, m.subsistence_household_share);
        let mut chosen = take_from_top(&ranked, &weight, target);
        chosen.sort_by_key(|&h| (place[h], h));
        if !chosen.is_empty() {
            let w: Vec<Int> = chosen.iter().map(|&h| weight[h]).collect();
            let bill = bani(m.farm_in_kind);
            let amounts = spread_unsplit(&w, &[], curve, m.subsistence_dispersion, passes, bill);
            for (&h, a) in chosen.iter().zip(amounts) {
                out.hh_subsistence_income[h] = amount(a);
            }
        }
    }

    // E. Informal income outside a job.
    let prop = &m.informal_propensity;
    let eligible: Vec<usize> = (0..n).filter(|&i| of_age(i)).collect();
    if !eligible.is_empty() && m.informal_income > 0 {
        let propensity = |i: usize| -> Int {
            let v = match status[i] {
                KIND_EMPLOYEE => prop.employee_by_industry_group[usize::from(group[i])],
                KIND_EMPLOYER => prop.employer,
                KIND_OWN_ACCOUNT => prop.own_account,
                KIND_FAMILY_WORKER => prop.family_worker,
                _ => match attrs.activity[i] {
                    Activity::Unemployed => prop.unemployed,
                    Activity::InactiveOther => prop.inactive_other,
                    Activity::Retired => prop.retired,
                    Activity::Student | Activity::Pupil => prop.student,
                    _ => 0,
                },
            };
            Int::from(v)
        };
        let key = |i: usize| propensity(i) * draws[i][2];
        let mut ranked = eligible.clone();
        ranked.sort_by_cached_key(|&i| (key(i), i));
        let eligible_w: Int = eligible.iter().map(|&i| pw[i]).sum();
        let target = ppm(eligible_w, m.informal_recipient_share);
        let mut chosen: Vec<usize> = take_from_top(&ranked, &pw, target)
            .into_iter()
            .filter(|&i| key(i) > 0)
            .collect();
        chosen.sort_by_key(|&i| (draws[i][3], i));
        if !chosen.is_empty() {
            let w: Vec<Int> = chosen.iter().map(|&i| pw[i]).collect();
            let bill = bani(m.informal_income);
            let amounts = spread_unsplit(&w, &[], curve, 1_000_000, passes, bill);
            for (&i, a) in chosen.iter().zip(amounts) {
                out.informal_income[i] = amount(a);
            }
        }
    }
    Ok(out)
}

fn check_margins(m: &IncomeMargins) -> Result<(), IncomeError> {
    let n_g = m.undeclared_wage_share.len();
    if n_g == 0 || m.quantile_curve.rank.len() < 2 {
        return Err(IncomeError::EmptyTable);
    }
    let prop = &m.informal_propensity;
    let share_ok = |v: u32| Int::from(v) <= UNIT;
    let ok = n_g == INDUSTRY_GROUPS
        && prop.employee_by_industry_group.len() == n_g
        && m.undeclared_wage_share.iter().all(|&v| share_ok(v))
        && [
            m.fully_informal_share,
            m.envelope_declared_share,
            m.low_wage_link,
            m.informal_recipient_share,
            m.subsistence_household_share,
            m.self_employment_declared,
            m.farm_declared,
            m.self_employment_education_link,
            m.subsistence_dispersion,
        ]
        .into_iter()
        .all(share_ok)
        && m.rel_kind.iter().all(|&v| v > 0)
        && m.farm_in_kind <= m.farm_income
        && curve_ok(&m.quantile_curve.rank, &m.quantile_curve.value);
    if ok {
        Ok(())
    } else {
        Err(IncomeError::ShapeMismatch)
    }
}

/// The members with the highest keys (the end of `ranked`), until their weight reaches `target`.
fn take_from_top(ranked: &[usize], weight: &[Int], target: Int) -> Vec<usize> {
    let (mut chosen, mut got) = (Vec::new(), 0);
    for &i in ranked.iter().rev() {
        if got >= target {
            break;
        }
        chosen.push(i);
        got += weight[i];
    }
    chosen
}

impl OtherIncomes {
    /// Income hash (FNV-1a 64 over the person count and each person's four
    /// amounts, then the household count and each household's amount; see
    /// the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        const PRIME: u64 = 0x0000_0100_0000_01B3;
        let n = self.informal_income.len();
        let persons = column_hash(
            n,
            (0..n).flat_map(|i| {
                [
                    self.wage_declared[i],
                    self.self_employment_income[i],
                    self.self_employment_declared[i],
                    self.informal_income[i],
                ]
                .into_iter()
                .flat_map(u64::to_le_bytes)
            }),
        );
        let n_hh = u32::try_from(self.hh_subsistence_income.len()).expect("table length fits u32");
        n_hh.to_le_bytes()
            .into_iter()
            .chain(
                self.hh_subsistence_income
                    .iter()
                    .flat_map(|v| v.to_le_bytes()),
            )
            .fold(persons, |hash, b| (hash ^ u64::from(b)).wrapping_mul(PRIME))
    }
}
