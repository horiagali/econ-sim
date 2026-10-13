//! Stage H — household wealth (spec `society/population-wealth`).
//!
//! Gives every private household cash, deposits, a dwelling value if it owns
//! its home, and possibly a housing loan and a consumer loan, so that the
//! household sector holds the totals of the financial accounts. Who holds
//! what is an educated guess (no survey covers Romania); the spec says which.
//! Earlier stages are not changed.
//!
//! Amounts are spread over households with stage F's `spread` (one group, no
//! split by sex). `python/reference/popgen_reference.py` (`assign_wealth`) is
//! the independent reference; the two agree household for household
//! (AC-POPH2-08).

use econ_rng::{KeyedRng, Stream};

use crate::arith::{Int, UNIT};
use crate::assign::column_hash;
use crate::housing::HouseholdHousing;
use crate::pensions::PersonPensions;
use crate::wages::{Group, PersonWages, curve_ok, spread};
use crate::{Population, Role};

/// `tick` of the draw context: the stage number.
const STAGE: u32 = 7;
const BANI_PER_LEU: Int = 100;
const MONTHS: u16 = 12;
/// `hh_tenure` of an owner household.
const TENURE_OWNER: u8 = 0;

/// A quantile curve: the value at a rank over the mean, as a line through knots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curve {
    /// Ranks of the knots, in millionths: ascending from 0 to 1,000,000.
    pub rank: Vec<u32>,
    /// Value at each knot over the mean, in millionths; not decreasing.
    pub value: Vec<u32>,
}

/// Who has a loan of one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowerRule {
    /// Share of private households that have the loan, in millionths.
    pub share_ppm: u32,
    /// Youngest age (years) of a borrowing household's head.
    pub age_from: u16,
    /// First age (years) at which a head no longer borrows.
    pub age_to: u16,
    /// How closely borrowing follows income, in millionths.
    pub income_link_ppm: u32,
    /// Added to the key of an urban household, in millionths.
    pub urban_bonus_ppm: u32,
}

/// Wealth margins: totals of the household sector in lei, and the rules for
/// who holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WealthMargins {
    /// Currency held by households.
    pub cash: u64,
    /// Deposits of households.
    pub deposits: u64,
    /// Housing loans outstanding.
    pub housing_loans: u64,
    /// Consumer loans outstanding.
    pub consumer_loans: u64,
    /// Market value of owned dwellings.
    pub dwellings: u64,
    /// Mean dwelling value of each locality class, as proportions, in millionths.
    pub rel_locality: Vec<u32>,
    /// Cash and deposits together.
    pub liquid_curve: Curve,
    /// Dwelling values.
    pub dwelling_curve: Curve,
    /// Loan balances, for both kinds of loan.
    pub loan_curve: Curve,
    /// Housing loans.
    pub mortgage: BorrowerRule,
    /// Consumer loans.
    pub consumer: BorrowerRule,
    /// How closely a household's place on the liquid curve follows its income, in millionths.
    pub liquid_income_link_ppm: u32,
    /// The same for the dwelling curve.
    pub dwelling_income_link_ppm: u32,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WealthParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Passes of scaling to the proportions between locality classes.
    pub scaling_passes: u32,
}

impl WealthParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        WealthParams {
            rng_seed,
            scaling_passes: 20,
        }
    }
}

/// Wealth columns, aligned with the households table; amounts in bani.
/// Records that are not private households have zero in every column.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseholdWealth {
    /// Currency.
    pub hh_cash: Vec<u64>,
    /// Bank deposits.
    pub hh_deposits: Vec<u64>,
    /// Market value of the dwelling; 0 for a household that does not own its home.
    pub hh_dwelling_value: Vec<u64>,
    /// Housing loan outstanding.
    pub hh_mortgage: Vec<u64>,
    /// Consumer loans outstanding.
    pub hh_consumer_debt: Vec<u64>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WealthError {
    /// No locality classes, or a curve with fewer than two knots.
    EmptyTable,
    /// A proportion of zero, a curve that does not ascend from rank 0 to
    /// 1,000,000, a share above 1, an empty age range, a locality class the
    /// margins do not have, or columns of another population.
    ShapeMismatch,
}

/// Assign cash, deposits, dwelling value and loans to every household.
/// Without wages or pensions every income counts as zero.
///
/// # Errors
/// See [`WealthError`].
pub fn assign_wealth(
    pop: &Population,
    housing: &HouseholdHousing,
    wages: Option<&PersonWages>,
    pensions: Option<&PersonPensions>,
    margins: &WealthMargins,
    params: &WealthParams,
) -> Result<HouseholdWealth, WealthError> {
    check_margins(margins)?;
    let m = margins;
    let (hh, p) = (&pop.households, &pop.persons);
    let (n_hh, n) = (hh.hh_weight.len(), p.household_id.len());
    if housing.hh_tenure.len() != n_hh
        || housing.hh_locality_size.len() != n_hh
        || housing.hh_urban.len() != n_hh
        || wages.is_some_and(|w| w.wage.len() != n)
        || pensions.is_some_and(|g| g.pension.len() != n)
        || housing
            .hh_locality_size
            .iter()
            .any(|&k| usize::from(k) >= m.rel_locality.len())
    {
        return Err(WealthError::ShapeMismatch);
    }
    let weight: Vec<Int> = hh.hh_weight.iter().map(|&w| Int::from(w)).collect();
    let mut income: Vec<Int> = vec![0; n_hh];
    let mut head_years = vec![0u16; n_hh];
    for i in 0..n {
        let h = p.household_id[i] as usize;
        income[h] += wages.map_or(0, |w| Int::from(w.wage[i]))
            + pensions.map_or(0, |g| Int::from(g.pension[i]));
        if p.role[i] == Role::Head {
            head_years[h] = p.age[i] / MONTHS;
        }
    }
    let private: Vec<usize> = (0..n_hh).filter(|&h| !hh.hh_collective[h]).collect();
    let mut out = HouseholdWealth {
        hh_cash: vec![0; n_hh],
        hh_deposits: vec![0; n_hh],
        hh_dwelling_value: vec![0; n_hh],
        hh_mortgage: vec![0; n_hh],
        hh_consumer_debt: vec![0; n_hh],
    };
    if private.is_empty() {
        return Ok(out);
    }
    // Four keyed draws per household, in millionths: dwelling, liquid, mortgage, consumer.
    let rng = KeyedRng::new(params.rng_seed);
    let mut draws: Vec<[Int; 4]> = vec![[0; 4]; n_hh];
    for &h in &private {
        let mut d = rng.draw(Stream::PopulationGen, STAGE, h as u64);
        for slot in &mut draws[h] {
            *slot = (Int::from(d.u64() >> 44) * UNIT) >> 20;
        }
    }
    let draws = &draws;
    let draw = |k: usize| move |h: usize| draws[h][k];
    let ctx = Ctx {
        weight: &weight,
        income: &income,
        passes: params.scaling_passes,
    };
    let bani = |lei: u64| Int::from(lei) * BANI_PER_LEU;
    let amount = |v: Int| u64::try_from(v).expect("amount fits u64");

    // 1. Dwellings of owner households.
    let owners: Vec<usize> = private
        .iter()
        .copied()
        .filter(|&h| housing.hh_tenure[h] == TENURE_OWNER)
        .collect();
    let order = ctx.mixed_order(&owners, &draw(0), m.dwelling_income_link_ppm, &|_| 0);
    let locality_targets: Vec<Int> = m
        .rel_locality
        .iter()
        .chain(&m.rel_locality)
        .map(|&v| Int::from(v))
        .collect();
    let dims = [(
        order
            .iter()
            .map(|&h| usize::from(housing.hh_locality_size[h]))
            .collect(),
        locality_targets,
    )];
    let values = ctx.place(&order, &dims, &m.dwelling_curve, bani(m.dwellings));
    for (&h, v) in order.iter().zip(values) {
        out.hh_dwelling_value[h] = amount(v);
    }

    // 2. Money of private households: cash and deposits in the national proportion.
    let order = ctx.mixed_order(&private, &draw(1), m.liquid_income_link_ppm, &|_| 0);
    let (cash, liquid_total) = (bani(m.cash), bani(m.cash) + bani(m.deposits));
    for (&h, v) in order
        .iter()
        .zip(ctx.place(&order, &[], &m.liquid_curve, liquid_total))
    {
        let c = if liquid_total == 0 {
            0
        } else {
            v * cash / liquid_total
        };
        out.hh_cash[h] = amount(c);
        out.hh_deposits[h] = amount(v - c);
    }

    // 3 and 4. Loans: who borrows, then how much.
    let total_w: Int = private.iter().map(|&h| weight[h]).sum();
    let borrowers = |pool: &[usize], rule: &BorrowerRule, k: usize| -> Vec<usize> {
        let eligible: Vec<usize> = pool
            .iter()
            .copied()
            .filter(|&h| (rule.age_from..rule.age_to).contains(&head_years[h]))
            .collect();
        let bonus = |h: usize| {
            if housing.hh_urban[h] {
                Int::from(rule.urban_bonus_ppm) * UNIT
            } else {
                0
            }
        };
        let ranked = ctx.mixed_order(&eligible, &draw(k), rule.income_link_ppm, &bonus);
        let target = Int::from(rule.share_ppm) * total_w / UNIT;
        let (mut got, mut chosen) = (0, Vec::new());
        for &h in ranked.iter().rev() {
            if got >= target {
                break;
            }
            chosen.push(h);
            got += weight[h];
        }
        chosen
    };
    let mut chosen = borrowers(&owners, &m.mortgage, 2);
    chosen.sort_by_key(|&h| (out.hh_dwelling_value[h], h));
    let values = ctx.place(&chosen, &[], &m.loan_curve, bani(m.housing_loans));
    for (&h, v) in chosen.iter().zip(values) {
        out.hh_mortgage[h] = amount(v);
    }
    let mut chosen = borrowers(&private, &m.consumer, 3);
    chosen.reverse(); // lowest key first
    let values = ctx.place(&chosen, &[], &m.loan_curve, bani(m.consumer_loans));
    for (&h, v) in chosen.iter().zip(values) {
        out.hh_consumer_debt[h] = amount(v);
    }
    Ok(out)
}

fn check_margins(m: &WealthMargins) -> Result<(), WealthError> {
    if m.rel_locality.is_empty() {
        return Err(WealthError::EmptyTable);
    }
    if m.rel_locality.contains(&0) {
        return Err(WealthError::ShapeMismatch);
    }
    for curve in [&m.liquid_curve, &m.dwelling_curve, &m.loan_curve] {
        if curve.rank.len() < 2 {
            return Err(WealthError::EmptyTable);
        }
        if !curve_ok(&curve.rank, &curve.value) {
            return Err(WealthError::ShapeMismatch);
        }
    }
    let unit = |v: u32| Int::from(v) <= UNIT;
    let rule_ok =
        |b: &BorrowerRule| unit(b.share_ppm) && unit(b.income_link_ppm) && b.age_from < b.age_to;
    if rule_ok(&m.mortgage)
        && rule_ok(&m.consumer)
        && unit(m.liquid_income_link_ppm)
        && unit(m.dwelling_income_link_ppm)
    {
        Ok(())
    } else {
        Err(WealthError::ShapeMismatch)
    }
}

/// What the placing of households needs.
struct Ctx<'a> {
    weight: &'a [Int],
    income: &'a [Int],
    passes: u32,
}

impl Ctx<'_> {
    /// `members` from the lowest to the highest key, where the key mixes a
    /// member's rank by income (the midpoint of its own weight, in millionths)
    /// with its keyed draw: `link_ppm` of the first, the rest of the second,
    /// plus a bonus.
    fn mixed_order(
        &self,
        members: &[usize],
        draw: &dyn Fn(usize) -> Int,
        link_ppm: u32,
        bonus: &dyn Fn(usize) -> Int,
    ) -> Vec<usize> {
        if members.is_empty() {
            return Vec::new();
        }
        let link = Int::from(link_ppm);
        let total: Int = members.iter().map(|&h| self.weight[h]).sum();
        let mut by_income = members.to_vec();
        by_income.sort_by_key(|&h| (self.income[h], h));
        let mut keyed = Vec::with_capacity(members.len());
        let mut before = 0;
        for h in by_income {
            let rank = (2 * before + self.weight[h]) * UNIT / (2 * total);
            keyed.push((link * rank + (UNIT - link) * draw(h) + bonus(h), h));
            before += self.weight[h];
        }
        keyed.sort_unstable();
        keyed.into_iter().map(|(_, h)| h).collect()
    }

    /// Spread `bill` over `members` (in this order) along `curve`: one group
    /// without a split by sex, the curve in full, no floor.
    fn place(
        &self,
        members: &[usize],
        dims: &[(Vec<usize>, Vec<Int>)],
        curve: &Curve,
        bill: Int,
    ) -> Vec<Int> {
        if members.is_empty() || bill <= 0 {
            return vec![0; members.len()];
        }
        let w: Vec<Int> = members.iter().map(|&h| self.weight[h]).collect();
        let sex = vec![0; members.len()];
        let group = Group {
            w: &w,
            sex: &sex,
            rel_sex: [UNIT, UNIT],
            curve_rank: &curve.rank,
            curve_value: &curve.value,
            dispersion_ppm: 1_000_000,
            scaling_passes: self.passes,
            bill,
            floor: 0,
        };
        spread(&group, dims)
    }
}

impl HouseholdWealth {
    /// Wealth hash (FNV-1a 64 over the household count, then each
    /// household's five amounts; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let n = self.hh_cash.len();
        column_hash(
            n,
            (0..n).flat_map(|h| {
                [
                    self.hh_cash[h],
                    self.hh_deposits[h],
                    self.hh_dwelling_value[h],
                    self.hh_mortgage[h],
                    self.hh_consumer_debt[h],
                ]
                .into_iter()
                .flat_map(u64::to_le_bytes)
            }),
        )
    }
}
