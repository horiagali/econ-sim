//! Taxes on income and the core social transfers on pay day, and the
//! quarterly taxes of the firm units (specs `economy/taxation` and
//! `economy/social-transfers`, "First version", roadmap M3). They apply when
//! the law has them ([`Law::law_2021`]); the M2 placeholder law has VAT only.
//!
//! The rules are the pure functions of `econ-mech-tax` and
//! `econ-mech-transfers`; this module finds who they apply to and posts what
//! they give through the ledger.

use econ_ledger::{FlowCode, Sector};
use econ_mech_tax::income::{
    DeductionRule, IndustryOverride, LabourTaxes, ProfitTaxes, Schedule, WageTax, tax_pension,
    tax_quarter, tax_wage,
};
use econ_mech_transfers::{
    ChildRule, MinimumIncomeRule, PensionRule, Separation, UnemploymentClaim, UnemploymentRule,
    child_allowance, claim_month, minimum_income, pension, pension_points, start_claim,
};
use econ_popgen::{EduLevel, NO_EMPLOYER};
use econ_rng::Stream;
use econ_types::Bani;

use super::money::{Postings, bani};
use super::{CATALOGUE_VAT, Law, World};

/// The taxes on income of the law in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeTaxLaw {
    /// Contributions, the schedule and the personal deduction.
    pub labour: LabourTaxes,
    /// Industries with their own rates: (catalogue index, override).
    pub overrides: Vec<(usize, IndustryOverride)>,
    /// `pension_tax_free`, a month.
    pub pension_tax_free: Bani,
    /// Profit and micro-enterprise taxes.
    pub profit: ProfitTaxes,
}

/// The social transfers of the law in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferLaw {
    /// State pension.
    pub pension: PensionRule,
    /// Unemployment benefit.
    pub unemployment: UnemploymentRule,
    /// Child allowance.
    pub child: ChildRule,
    /// Guaranteed minimum income.
    pub minimum_income: MinimumIncomeRule,
}

fn catalogue(id: &str) -> usize {
    CATALOGUE_VAT
        .iter()
        .position(|&(c, _)| c == id)
        .expect("a catalogue industry")
}

impl Law {
    /// The law of 1 December 2021 with the taxes on income and the social
    /// transfers of the specs' first versions ([law in force]): CAS 25%
    /// (3.75 points to Pillar II), CASS 10%, CAM 2.25%, income tax 10% with
    /// the personal deduction, construction and software overrides, pensions
    /// taxed above 2,000 lei, profit tax 16%, micro tax 1% or 3% under 1
    /// million euro; pension point 1,442 lei and minimum pension 800, the
    /// unemployment benefit, child allowance 427 and 214 lei, the minimum
    /// income with 60% take-up.
    ///
    /// [law in force]: ../../../docs/01-research/notes/law-in-force-romania-2021.md
    #[must_use]
    pub fn law_2021() -> Law {
        let lei = Bani::from_lei;
        let mut law = Law::placeholder_2021();
        let construction = IndustryOverride {
            income_rate_bp: Some(0),
            cas_bp: 2_125,
            cass_bp: 0,
            ceiling: lei(30_000),
        };
        let software = IndustryOverride {
            income_rate_bp: Some(0),
            cas_bp: 2_500,
            cass_bp: 1_000,
            ceiling: lei(1_000_000_000),
        };
        law.income = Some(IncomeTaxLaw {
            labour: LabourTaxes {
                cas_bp: 2_500,
                pillar2_bp: 375,
                cass_bp: 1_000,
                cam_bp: 225,
                income: Schedule::flat(1_000),
                deduction: DeductionRule {
                    base: [lei(510), lei(670), lei(830), lei(990), lei(1_310)],
                    full_until: lei(1_950),
                    zero_at: lei(3_600),
                },
            },
            overrides: vec![
                (catalogue("construction_buildings"), construction),
                (catalogue("construction_civil"), construction),
                (catalogue("it_software"), software),
            ],
            pension_tax_free: lei(2_000),
            profit: ProfitTaxes {
                profit_bp: 1_600,
                // 1 million euro at the input-output table's 4.9215 lei.
                micro_threshold: lei(4_921_500),
                micro_with_staff_bp: 100,
                micro_no_staff_bp: 300,
            },
        });
        law.transfers = Some(TransferLaw {
            pension: PensionRule {
                point_value: lei(1_442),
                minimum: lei(800),
            },
            unemployment: UnemploymentRule {
                isr: lei(500),
                isr_share_ppm: 750_000,
                wage_share_by_years: vec![(3, 30_000), (5, 50_000), (10, 70_000), (20, 100_000)],
                months_by_years: vec![(1, 6), (5, 9), (10, 12)],
            },
            child: ChildRule {
                under2: lei(427),
                to18: lei(214),
            },
            minimum_income: MinimumIncomeRule {
                floor_by_size: vec![lei(142), lei(255), lei(357), lei(442), lei(527)],
                per_extra: lei(36),
                take_up_ppm: 600_000,
            },
        });
        law
    }
}

/// What the taxes and transfers keep per record.
#[derive(Debug, Clone, Default)]
pub(super) struct FiscalState {
    /// Per person: pension points (millionths), years of contributions, a
    /// running unemployment claim, the declared self-employment income at the
    /// start (a month), and whether the person claims the household's
    /// dependants.
    points: Vec<u64>,
    contribution_years: Vec<u8>,
    claims: Vec<Option<UnemploymentClaim>>,
    self_declared: Vec<Bani>,
    claims_dependants: Vec<bool>,
    /// Per household: members, children under 18, collective, takes up the
    /// minimum income when it qualifies.
    size: Vec<u16>,
    children: Vec<u8>,
    collective: Vec<bool>,
    take_up: Vec<bool>,
    /// Per unit: the quarter's value of output, input purchases, labour cost
    /// and sales; sales of the last four quarters; losses carried forward;
    /// the unit is a micro cohort unit, and the firms it stands for.
    q_value: Vec<i128>,
    q_inputs: Vec<i128>,
    q_labour: Vec<i128>,
    q_sales: Vec<i128>,
    sales_4q: Vec<[i128; 4]>,
    losses: Vec<Bani>,
    micro: Vec<bool>,
    firm_count: Vec<u32>,
}

/// Years of contributions (social transfers, simplification): years since
/// 18 less the years of schooling above upper secondary, not below zero.
fn contribution_years(age_months: u16, edu: EduLevel) -> u8 {
    let schooling = match edu {
        EduLevel::Isced4 => 2,
        EduLevel::Isced5To6 => 4,
        EduLevel::Isced7To8 => 6,
        _ => 0,
    };
    let years = (age_months / 12)
        .saturating_sub(18)
        .saturating_sub(schooling);
    u8::try_from(years.min(60)).expect("small")
}

impl FiscalState {
    /// The fiscal state at load.
    #[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
    pub(super) fn new(
        law: &Law,
        seed: econ_rng::KeyedRng,
        household_id: &[u32],
        age: &[u16],
        edu: &[EduLevel],
        pension_start: &[Bani],
        self_declared: &[Bani],
        collective: &[bool],
        unit_micro: &[bool],
        firm_count: &[u32],
    ) -> FiscalState {
        let (n_p, n_hh, n_u) = (household_id.len(), collective.len(), unit_micro.len());
        let mut size = vec![0u16; n_hh];
        let mut children = vec![0u8; n_hh];
        let mut claims_dependants = vec![false; n_p];
        let mut claimed = vec![false; n_hh];
        for p in 0..n_p {
            let h = household_id[p] as usize;
            size[h] = size[h].saturating_add(1);
            if age[p] < 18 * 12 {
                children[h] = children[h].saturating_add(1);
            } else if !claimed[h] {
                claimed[h] = true;
                claims_dependants[p] = true;
            }
        }
        let points = match &law.transfers {
            Some(t) => pension_start
                .iter()
                .map(|&p| {
                    if p > Bani::ZERO {
                        pension_points(p, t.pension.point_value)
                    } else {
                        0
                    }
                })
                .collect(),
            None => vec![0; n_p],
        };
        let take_up_ppm = law
            .transfers
            .as_ref()
            .map_or(0, |t| u64::from(t.minimum_income.take_up_ppm));
        FiscalState {
            points,
            contribution_years: (0..n_p)
                .map(|p| contribution_years(age[p], edu[p]))
                .collect(),
            claims: vec![None; n_p],
            self_declared: self_declared.to_vec(),
            claims_dependants,
            size,
            children,
            collective: collective.to_vec(),
            take_up: (0..n_hh)
                .map(|h| {
                    seed.fast_u64(Stream::Consumption, 0, h as u64, 11) % 1_000_000 < take_up_ppm
                })
                .collect(),
            q_value: vec![0; n_u],
            q_inputs: vec![0; n_u],
            q_labour: vec![0; n_u],
            q_sales: vec![0; n_u],
            sales_4q: vec![[0; 4]; n_u],
            losses: vec![Bani::ZERO; n_u],
            micro: unit_micro.to_vec(),
            firm_count: firm_count.to_vec(),
        }
    }

    pub(super) fn add_value(&mut self, u: usize, v: i128) {
        self.q_value[u] += v;
    }
    pub(super) fn add_inputs(&mut self, u: usize, v: i128) {
        self.q_inputs[u] += v;
    }
    pub(super) fn add_labour(&mut self, u: usize, v: i128) {
        self.q_labour[u] += v;
    }
    pub(super) fn add_sales(&mut self, u: usize, v: i128) {
        self.q_sales[u] += v;
    }

    /// Hash what the taxes and transfers carry from one day to the next.
    pub(super) fn hash(&self, write: &mut impl FnMut(u64)) {
        for c in &self.claims {
            write(c.map_or(u64::MAX, |c| {
                u64::from(c.months_left) << 48 ^ c.amount.get().unsigned_abs()
            }));
        }
        for (&a, &b) in self.q_value.iter().zip(&self.q_inputs) {
            write(a.unsigned_abs() as u64 ^ (b.unsigned_abs() as u64).rotate_left(17));
        }
        for &l in &self.losses {
            write(l.get().unsigned_abs());
        }
    }
}

/// What the withholding on the month's wages comes to, weighted.
#[derive(Debug, Default)]
pub(super) struct Withheld {
    pub cas_public: i128,
    pub cas_pillar2: i128,
    pub cass: i128,
    pub income_tax: i128,
    pub cam: i128,
}

impl World {
    /// Taxes on one wage payment: the general rules, the unit's industry's
    /// override, the household's dependants for the person who claims them.
    /// No taxes under a law without them: the net is the gross.
    pub(super) fn wage_tax(&self, p: usize, unit: u16, gross: Bani, declared: Bani) -> WageTax {
        let Some(law) = &self.law.income else {
            return WageTax {
                cas_public: Bani::ZERO,
                cas_pillar2: Bani::ZERO,
                cass: Bani::ZERO,
                income_tax: Bani::ZERO,
                cam: Bani::ZERO,
                net: gross,
            };
        };
        let j = usize::from(self.units.industry[usize::from(unit)]);
        let over = law.overrides.iter().find(|&&(k, _)| k == j).map(|(_, o)| o);
        let h = self.persons.household_id[p] as usize;
        let dependants = if self.fs.claims_dependants[p] {
            self.fs.children[h]
        } else {
            0
        };
        tax_wage(gross, declared.min(gross), dependants, &law.labour, over)
    }

    /// Post the month's withholding: contributions and income tax to the
    /// government, the Pillar II share to the pension funds, CAM on top.
    pub(super) fn post_withheld(post: &mut Postings, w: &Withheld) {
        post.pays_gov(Sector::Firms, bani(w.cas_public), FlowCode::CONTRIB_PENSION);
        post.deposits(
            Sector::Firms,
            Sector::PensionFunds,
            bani(w.cas_pillar2),
            FlowCode::CONTRIB_PENSION_PILLAR2,
        );
        post.pays_gov(Sector::Firms, bani(w.cass), FlowCode::CONTRIB_HEALTH);
        post.pays_gov(Sector::Firms, bani(w.income_tax), FlowCode::TAX_INCOME);
        post.pays_gov(
            Sector::Firms,
            bani(w.cam),
            FlowCode::TAX_EMPLOYER_CONTRIBUTION,
        );
    }

    /// A person's pension this month and its income tax.
    pub(super) fn pension_of(&self, p: usize) -> (Bani, Bani) {
        let start = self.pp.pension[p];
        if start <= Bani::ZERO {
            return (Bani::ZERO, Bani::ZERO);
        }
        let gross = self
            .law
            .transfers
            .as_ref()
            .map_or(start, |t| pension(self.fs.points[p], &t.pension));
        let tax = self.law.income.as_ref().map_or(Bani::ZERO, |l| {
            tax_pension(gross, l.pension_tax_free, &l.labour.income)
        });
        (gross, tax)
    }

    /// Income tax on the month's declared self-employment income, scaled as
    /// the income is.
    pub(super) fn self_employment_tax(&self, p: usize, scale_ppm: i128) -> Bani {
        let Some(law) = &self.law.income else {
            return Bani::ZERO;
        };
        let declared = self.fs.self_declared[p];
        if declared <= Bani::ZERO {
            return Bani::ZERO;
        }
        let amount = bani(i128::from(declared.get()) * scale_ppm / 1_000_000);
        law.labour.income.tax(amount)
    }

    /// A layoff opens an unemployment claim on the last wage.
    pub(super) fn open_claim(&mut self, p: usize, last_wage: Bani, layoff: bool) {
        let Some(t) = &self.law.transfers else {
            return;
        };
        let how = if layoff {
            Separation::Layoff
        } else {
            Separation::Quit
        };
        self.fs.claims[p] = start_claim(
            how,
            last_wage,
            self.fs.contribution_years[p],
            &t.unemployment,
        );
    }

    /// The month's unemployment benefits, child allowances and minimum
    /// incomes, paid by the government; `got` is each household's income of
    /// the pay day so far, and grows by what it receives.
    pub(super) fn pay_transfers(&mut self, got: &mut [Bani], post: &mut Postings) {
        let Some(t) = self.law.transfers.clone() else {
            return;
        };
        let w = |s: &Self, h: usize| i128::from(s.households.hh_weight[h]);
        // Unemployment benefit: stops when the person is hired.
        let mut ub = 0i128;
        for p in 0..self.persons.employer.len() {
            if let Some(mut c) = self.fs.claims[p] {
                let hired = self.persons.employer[p] != NO_EMPLOYER;
                let amount = claim_month(&mut c, hired);
                self.fs.claims[p] = (c.months_left > 0).then_some(c);
                self.slips.unemployment_benefit[p] = amount;
                if amount > Bani::ZERO {
                    let h = self.persons.household_id[p] as usize;
                    got[h] += amount;
                    ub += w(self, h) * i128::from(amount.get());
                }
            }
        }
        post.gov_pays(Sector::Households, bani(ub), FlowCode::BENEFIT_UNEMPLOYMENT);
        // Child allowance, to the child's household (not to collective ones).
        let mut family = 0i128;
        for p in 0..self.persons.employer.len() {
            let h = self.persons.household_id[p] as usize;
            if self.fs.collective[h] {
                continue;
            }
            let amount = child_allowance(self.pp.age[p], &t.child);
            if amount > Bani::ZERO {
                self.slips.child_allowance[h] += amount;
                got[h] += amount;
                family += w(self, h) * i128::from(amount.get());
            }
        }
        post.gov_pays(Sector::Households, bani(family), FlowCode::BENEFIT_FAMILY);
        // Minimum income: the floor less the month's income, for the
        // households that take it up.
        let mut assistance = 0i128;
        for h in 0..got.len() {
            if self.fs.collective[h] || !self.fs.take_up[h] {
                continue;
            }
            let amount = minimum_income(self.fs.size[h], got[h], &t.minimum_income);
            if amount > Bani::ZERO {
                self.slips.minimum_income[h] = amount;
                got[h] += amount;
                assistance += w(self, h) * i128::from(amount.get());
            }
        }
        post.gov_pays(
            Sector::Households,
            bani(assistance),
            FlowCode::BENEFIT_SOCIAL_ASSISTANCE,
        );
    }

    /// At a quarter's end: each unit's profit tax, or micro tax on its
    /// turnover, paid to the government; the quarter's figures start again.
    pub(super) fn quarter_taxes(&mut self, post: &mut Postings) {
        let Some(law) = self.law.income.clone() else {
            return;
        };
        let (mut profit_tax, mut micro_tax) = (0i128, 0i128);
        for u in 0..self.units.industry.len() {
            let fs = &mut self.fs;
            fs.sales_4q[u].rotate_left(1);
            fs.sales_4q[u][3] = fs.q_sales[u];
            let profit = bani(fs.q_value[u] - fs.q_inputs[u] - fs.q_labour[u]);
            let year_per_firm =
                fs.sales_4q[u].iter().sum::<i128>() / i128::from(fs.firm_count[u].max(1));
            let micro = (fs.micro[u]
                && year_per_firm < i128::from(law.profit.micro_threshold.get()))
            .then(|| (bani(fs.q_sales[u]), self.units.staff[u] > 0));
            let tax = tax_quarter(profit, &mut fs.losses[u], micro, &law.profit);
            if tax > Bani::ZERO {
                self.units.deposits[u] -= tax;
                if micro.is_some() {
                    micro_tax += i128::from(tax.get());
                } else {
                    profit_tax += i128::from(tax.get());
                }
            }
            fs.q_value[u] = 0;
            fs.q_inputs[u] = 0;
            fs.q_labour[u] = 0;
            fs.q_sales[u] = 0;
        }
        post.pays_gov(Sector::Firms, bani(profit_tax), FlowCode::TAX_PROFIT);
        post.pays_gov(Sector::Firms, bani(micro_tax), FlowCode::TAX_MICRO);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contribution_years_leave_out_schooling() {
        assert_eq!(contribution_years(40 * 12, EduLevel::Isced3), 22);
        assert_eq!(contribution_years(40 * 12, EduLevel::Isced5To6), 18);
        assert_eq!(contribution_years(17 * 12, EduLevel::Isced0To2), 0);
    }

    #[test]
    fn the_2021_law_names_its_overrides() {
        let law = Law::law_2021();
        let income = law.income.expect("taxes on income");
        assert_eq!(income.overrides.len(), 3);
        assert!(law.transfers.is_some());
        assert!(Law::placeholder_2021().income.is_none());
    }
}
