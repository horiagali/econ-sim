//! Ministry budgets on the `World` (spec `game/lever-system`, "Ministry
//! budgets" and "How the `World` carries it"; feature `levers`): in a world
//! built from `law.json`, each budget line's running allowance pays its share
//! of its public units' labour cost, then inputs; its investment part is a
//! twelfth a month; public units spend the money over the next month and
//! staff to a target with a hiring freeze.
//!
//! The public units are the government's agents: at each month end, right
//! after wages, the government pays each unit its lines' running payment
//! (`gov.purchases`, the unit's non-market output at cost) and investment
//! part (`gov.investment`). The government's fixed purchases of the
//! input-output table's government column, its imports and its share of the
//! investment column stop.

use econ_ledger::{FlowCode, Sector};
use econ_types::{Bani, Date};

use super::World;
use super::money::{Postings, bani, ratio, split};
use crate::levers::{BudgetLineMonth, LeverId, LeverValue, PolicyEvent};

/// Millionths.
const ONE: i128 = 1_000_000;

/// The ministry budgets' state.
#[derive(Debug, Clone, Default)]
pub(super) struct BudgetState {
    /// Whether the rule acts (a world built from `law.json` with a budget).
    pub(super) active: bool,
    /// The lines, in the order of `spend.ministry_budget`'s keys.
    lines: Vec<String>,
    /// Per line: its public units and their weights (the line's 2021 wage
    /// cell split over them); inputs and investment go by these.
    line_units: Vec<Vec<(usize, i128)>>,
    /// Per firm unit: the lines sharing its labour cost and their weights.
    unit_lines: Vec<Vec<(usize, i128)>>,
    /// Per firm unit: whether it is a public unit the budget pays.
    public: Vec<bool>,
    /// Per firm unit: the key of `spend.public_pay` and `spend.public_headcount`.
    unit_key: Vec<Option<u32>>,
    /// Per firm unit: its staff on day 0 (weighted).
    start_staff: Vec<u64>,
    /// Per firm unit: `spend.public_headcount` on day 0 (posts).
    start_posts: Vec<i64>,
    /// Per firm unit: `spend.public_pay` in force, millionths of the start.
    pay_ppm: Vec<i128>,
    /// The reference wages on day 0, which public hires are paid at.
    start_reference: Vec<[Bani; 10]>,
    /// Per firm unit: its inputs' mix, its industry's input structure in
    /// the input-output table at the starting prices, and the imported
    /// share, millionths.
    input_mix: Vec<Vec<i128>>,
    input_import_ppm: Vec<i128>,
    /// Investment: the investment column's domestic part, and its imported share.
    invest_mix: Vec<i128>,
    invest_import_ppm: i128,
    /// Per firm unit: input and investment money not spent yet.
    input_cash: Vec<i128>,
    invest_cash: Vec<i128>,
    /// Per firm unit: the government's top-ups at cost since the last month end.
    topup: Vec<i128>,
    /// Per public unit: its market sales since the last month end, which go
    /// to the government at the month end (`gov.sales`).
    market_sales: Vec<i128>,
    /// Benefits in kind (D632) a year and the firm units that deliver them
    /// with their keys: the health programme's to the other units of
    /// `health_services`, the rest to those of `social_care`.
    in_kind: Vec<(i128, Vec<(usize, i128)>)>,
    /// The old-age and survivors' cash benefits the pension rule does not pay
    /// (the special pensions, budget-2021): the budget's less twelve times
    /// the pension bill at load, a year.
    pension_residual: i128,
    /// Per firm unit: the hiring freeze.
    freeze: Vec<bool>,
    /// Per line: running allowance and paid, year to date.
    ytd_allowance: Vec<i128>,
    ytd_paid: Vec<i128>,
    /// The budget year.
    year: i32,
    /// Every line in every closed month.
    pub(super) months: Vec<BudgetLineMonth>,
    /// Pay and job choice (spec `economy/labor-market`): each tier's mean
    /// wage by region and nationally at the last month end, and the persons
    /// looking for work while employed in the next day's matching.
    pub(super) market_wage: Vec<[i128; 3]>,
    pub(super) national_wage: [i128; 3],
    pub(super) job_seekers: Vec<u32>,
    /// `fx.regime`: trade's intervention share at load (the managed float).
    pub(super) fx_managed_ppm: Option<u32>,
}

/// What [`World::start_budget`] needs from the starting state.
#[derive(Debug, Clone)]
pub(crate) struct BudgetStart {
    /// (line, public unit id, D11 a year).
    pub(crate) wage_shares: Vec<(String, String, Bani)>,
    pub(crate) investment_domestic: Vec<Bani>,
    pub(crate) investment_imported: Vec<Bani>,
    pub(crate) gov_investment_year: Bani,
    /// The budget's social benefits: (programme id, cash, in kind) a year.
    pub(crate) benefits: Vec<(String, Bani, Bani)>,
}

/// `wage / market`, millionths; 1 without a market wage.
fn rho(wage: i128, market: i128) -> u32 {
    if market <= 0 {
        1_000_000
    } else {
        u32::try_from(ratio(wage.max(0), ONE, market)).unwrap_or(u32::MAX)
    }
}

/// A year's amount's twelfth of calendar month `k` (0 = January): the
/// remainder to the first months (spec `economy/budget-2021`).
fn twelfth(year: i128, k: u8) -> i128 {
    let (q, r) = (year / 12, year % 12);
    q + i128::from(i128::from(k) < r)
}

impl World {
    /// At load, under `law.json`: the lines, their units, the mixes; the
    /// government's fixed purchases stop (spec, "What it replaces").
    pub(crate) fn start_budget(&mut self, start: &BudgetStart) {
        let n_u = self.units.industry.len();
        let n_i = self.industries.id.len();
        let Some(lev) = self.lever_state() else {
            return;
        };
        if start.wage_shares.is_empty() {
            return;
        }
        let reg = &lev.registry;
        let lines = reg.get(LeverId::SpendMinistryBudget).keys.clone();
        let unit_keys = reg.get(LeverId::SpendPublicPay).keys.clone();
        let headcount_keys = reg.get(LeverId::SpendPublicHeadcount).keys.clone();
        let posts_of = |k: usize| match lev
            .law
            .get(LeverId::SpendPublicHeadcount, u32::try_from(k).ok())
        {
            Some(LeverValue::Number(n)) => *n,
            _ => 0,
        };
        let unit_id = |u: usize| {
            format!(
                "{}.public",
                self.industries.id[usize::from(self.units.industry[u])]
            )
        };
        let mut b = BudgetState {
            active: true,
            line_units: vec![Vec::new(); lines.len()],
            unit_lines: vec![Vec::new(); n_u],
            public: vec![false; n_u],
            unit_key: vec![None; n_u],
            start_staff: self.units.staff.clone(),
            start_posts: vec![0; n_u],
            pay_ppm: vec![ONE; n_u],
            start_reference: self.mac.reference_wage.clone(),
            input_cash: vec![0; n_u],
            invest_cash: vec![0; n_u],
            topup: vec![0; n_u],
            market_sales: vec![0; n_u],
            freeze: vec![false; n_u],
            ytd_allowance: vec![0; lines.len()],
            ytd_paid: vec![0; lines.len()],
            year: self.calendar.date(self.today).year,
            ..BudgetState::default()
        };
        for u in 0..n_u {
            if !self.uu.public[u] {
                continue;
            }
            let id = unit_id(u);
            if let Some(k) = unit_keys.iter().position(|x| *x == id) {
                b.public[u] = true;
                b.unit_key[u] = u32::try_from(k).ok();
            }
            if let Some(k) = headcount_keys.iter().position(|x| *x == id) {
                b.start_posts[u] = posts_of(k);
            }
        }
        for (line, unit, d11) in &start.wage_shares {
            let (Some(l), Some(u)) = (
                lines.iter().position(|x| x == line),
                (0..n_u).find(|&u| b.public[u] && unit_id(u) == *unit),
            ) else {
                continue;
            };
            let w = i128::from(d11.get()).max(0);
            b.line_units[l].push((u, w));
            b.unit_lines[u].push((l, w));
        }
        // A line with no wage cell sends its inputs and investment to its
        // units equally.
        for units in &mut b.line_units {
            if units.iter().all(|&(_, w)| w == 0) {
                for (_, w) in units.iter_mut() {
                    *w = 1;
                }
            }
        }
        // Inputs: each public unit's industry's input structure, valued at
        // the starting prices.
        let share = |imp: i128, dom: i128| {
            if imp + dom > 0 {
                ratio(imp, ONE, imp + dom)
            } else {
                0
            }
        };
        b.input_mix = vec![Vec::new(); n_u];
        b.input_import_ppm = vec![0; n_u];
        for u in 0..n_u {
            if !b.public[u] {
                continue;
            }
            let j = usize::from(self.units.industry[u]);
            let mut mix = vec![0i128; n_i];
            for &(i, a) in &self.ii.inputs[j] {
                mix[i] += i128::from(a) * i128::from(self.industries.price_ppm[i]) / ONE;
            }
            b.input_import_ppm[u] = share(i128::from(self.ii.import_ppm[j]), mix.iter().sum());
            b.input_mix[u] = mix;
        }
        let sum = |v: &[Bani]| -> i128 { v.iter().map(|b| i128::from(b.get()).max(0)).sum() };
        b.invest_mix = start
            .investment_domestic
            .iter()
            .map(|v| i128::from(v.get()).max(0))
            .collect();
        let (inv_dom, inv_imp) = (
            sum(&start.investment_domestic),
            sum(&start.investment_imported),
        );
        b.invest_import_ppm = share(inv_imp, inv_dom);
        // The government's fixed purchases stop; its share of the
        // investment column leaves it (the lines buy it).
        for q in &mut self.ii.gov_q {
            *q = 0;
        }
        for m in &mut self.ii.gov_imp {
            *m = Bani::ZERO;
        }
        let column = inv_dom + inv_imp;
        if column > 0 {
            let keep = ONE
                - ratio(
                    i128::from(start.gov_investment_year.get()).clamp(0, column),
                    ONE,
                    column,
                );
            for q in &mut self.ii.inv_q {
                *q = i64::try_from(ratio(i128::from(*q), keep, ONE)).expect("a quantity");
            }
            for m in &mut self.ii.inv_imp {
                *m = bani(ratio(i128::from(m.get()), keep, ONE));
            }
        }
        // Benefits in kind and the pensions' residual (owner, 2026-10-29).
        let staff_of = |industry: &str| -> Vec<(usize, i128)> {
            (0..n_u)
                .filter(|&u| {
                    !self.uu.public[u]
                        && self.industries.id[usize::from(self.units.industry[u])] == industry
                })
                .map(|u| (u, i128::from(self.units.staff[u]).max(1)))
                .collect()
        };
        let (mut health, mut other) = (0i128, 0i128);
        let mut pension_cash = 0i128;
        for (id, cash, kind) in &start.benefits {
            if id == "health_in_kind" {
                health += i128::from(kind.get());
            } else {
                other += i128::from(kind.get());
            }
            if id == "old_age" || id == "survivors" {
                pension_cash += i128::from(cash.get());
            }
        }
        b.in_kind = vec![
            (health, staff_of("health_services")),
            (other, staff_of("social_care")),
        ];
        let bill: i128 = (0..self.persons.employer.len())
            .map(|p| {
                let w = i128::from(self.person_weight(u32::try_from(p).expect("a person id")));
                w * i128::from(self.pension_of(p).0.get())
            })
            .sum();
        b.pension_residual = if pension_cash > 0 {
            (pension_cash - 12 * bill).max(0)
        } else {
            0
        };
        b.lines = lines;
        self.bud = b;
        self.budget_market_wages();
    }

    /// The pay day: a twelfth of the benefits in kind to the units that
    /// deliver them, by their staff, and of the pensions' residual to the
    /// pensioners, the same to each, into their deposits (spec
    /// `economy/budget-2021`, "Ledger mapping"; owner, 2026-10-29).
    fn budget_benefits(&mut self, k: u8) {
        let mut post = Postings::default();
        let in_kind = std::mem::take(&mut self.bud.in_kind);
        let mut firms = 0i128;
        for (year, units) in &in_kind {
            let w: Vec<i128> = units.iter().map(|&(_, w)| w).collect();
            for (&(u, _), part) in units.iter().zip(split(twelfth(*year, k), &w)) {
                self.units.deposits[u] += bani(part);
                firms += part;
            }
        }
        self.bud.in_kind = in_kind;
        post.gov_pays(Sector::Firms, bani(firms), FlowCode::BENEFIT_IN_KIND);
        let due = twelfth(self.bud.pension_residual, k);
        let n_h = self.households.hh_weight.len();
        let mut base = vec![0i128; n_h];
        for p in 0..self.persons.employer.len() {
            if self.pp.pension[p] > Bani::ZERO {
                base[self.persons.household_id[p] as usize] += 1;
            }
        }
        if due > 0
            && let Some(parts) =
                super::treasury::weighted_parts(due, &self.households.hh_weight, &base)
        {
            let mut paid = 0i128;
            for (h, &a) in parts.iter().enumerate() {
                if a > 0 {
                    self.households.hh_deposits[h] += bani(a);
                    // In the money received to date (AC-HH-03).
                    self.households.income_to_date[h] += bani(a);
                    paid += i128::from(self.households.hh_weight[h]) * a;
                }
            }
            post.gov_pays(Sector::Households, bani(paid), FlowCode::BENEFIT_PENSION);
        }
        post.commit(&mut self.ledger);
    }

    /// Each tier's weighted mean wage by region and nationally (pay and job
    /// choice, step 1).
    pub(super) fn budget_market_wages(&mut self) {
        let n_r = 1 + usize::from(self.pp.region.iter().copied().max().unwrap_or(0));
        let mut sum = vec![[0i128; 3]; n_r];
        let mut weight = vec![[0i128; 3]; n_r];
        for p in 0..self.persons.employer.len() {
            if self.persons.employer[p] == econ_popgen::NO_EMPLOYER {
                continue;
            }
            let Some(t) = super::load::tier(self.persons.occupation[p]) else {
                continue;
            };
            let r = usize::from(self.pp.region[p]);
            let w = i128::from(self.person_weight(u32::try_from(p).expect("a person id")));
            sum[r][t] += w * i128::from(self.persons.wage[p].get());
            weight[r][t] += w;
        }
        let mean = |s: i128, w: i128| if w > 0 { s / w } else { 0 };
        let mut national = [0i128; 3];
        for t in 0..3 {
            let (s, w) = (0..n_r).fold((0, 0), |(s, w), r| (s + sum[r][t], w + weight[r][t]));
            national[t] = mean(s, w);
        }
        self.bud.market_wage = (0..n_r)
            .map(|r| {
                let mut row = [0i128; 3];
                for t in 0..3 {
                    row[t] = if weight[r][t] > 0 {
                        mean(sum[r][t], weight[r][t])
                    } else {
                        national[t]
                    };
                }
                row
            })
            .collect();
        self.bud.national_wage = national;
    }

    /// Whether pay and job choice acts (a world built from `law.json`).
    pub(super) fn pay_choice_active(&self) -> bool {
        self.bud.active
    }

    /// A person's relative pay: wage over the market wage of its tier in its
    /// region, millionths (1 without a market wage).
    pub(super) fn person_rho_ppm(&self, p: usize) -> u32 {
        let Some(t) = super::load::tier(self.persons.occupation[p]) else {
            return 1_000_000;
        };
        let market = self
            .bud
            .market_wage
            .get(usize::from(self.pp.region[p]))
            .map_or(0, |row| row[t]);
        rho(i128::from(self.persons.wage[p].get()), market)
    }

    /// A unit's relative pay in a tier: its employees' mean wage there (the
    /// wage it offers if it has none) over the national market wage.
    pub(super) fn unit_rho_ppm(&self, u: usize, t: usize, offered: Bani) -> u32 {
        let (mut s, mut w) = (0i128, 0i128);
        for &i in &self.uu.members[u] {
            let p = i as usize;
            if super::load::tier(self.persons.occupation[p]) == Some(t) {
                let wt = i128::from(self.person_weight(i));
                s += wt * i128::from(self.persons.wage[p].get());
                w += wt;
            }
        }
        let wage = if w > 0 {
            s / w
        } else {
            i128::from(offered.get())
        };
        rho(wage, self.bud.national_wage[t])
    }

    /// Whether unit `u` is a public unit the budget pays.
    pub(super) fn budget_public(&self, u: usize) -> bool {
        self.bud.active && self.bud.public[u]
    }

    /// Phase 2 for a public unit, after its plan: its staff target, or its
    /// staff while the hiring freeze holds (its output follows its market
    /// sales, which the government no longer buys from).
    pub(super) fn budget_plan(&mut self, u: usize) {
        let posts = self.bud.unit_key[u]
            .and_then(|k| {
                match self
                    .lever_state()?
                    .law
                    .get(LeverId::SpendPublicHeadcount, Some(k))
                {
                    Some(LeverValue::Number(n)) => Some(*n),
                    _ => None,
                }
            })
            .unwrap_or(self.bud.start_posts[u]);
        let start = i128::from(self.bud.start_staff[u]);
        let target = if self.bud.start_posts[u] > 0 {
            ratio(
                start,
                i128::from(posts.max(0)),
                i128::from(self.bud.start_posts[u]),
            )
        } else {
            start
        };
        let target = u64::try_from(target.max(0)).unwrap_or(u64::MAX);
        self.units.desired_employment[u] = if self.bud.freeze[u] {
            target.min(self.units.staff[u])
        } else {
            target
        };
    }

    /// Phase 5 for a public unit: today's part of its input and investment
    /// money, as demand on the industries and as imports. The imports, and
    /// the product taxes and duties on its home purchases (paid at phase 8),
    /// come out of the firm sector's free deposits (the government tops the
    /// unit up at the day's end): when the sector holds less, the unit buys
    /// what it can and keeps the rest of its money for later days (no lever
    /// may overdraw the sector; in a normal run this never binds).
    pub(super) fn budget_buy(&mut self, u: usize, post: &mut Postings) {
        let date = self.calendar.date(self.today);
        let days_left = i128::from(date.days_in_month() + 1 - date.day);
        let b = &self.bud;
        let input_day = b.input_cash[u].max(0) / days_left;
        let invest_day = b.invest_cash[u].max(0) / days_left;
        let (mut input_imp, mut invest_imp) = (
            ratio(input_day, b.input_import_ppm[u], ONE),
            ratio(invest_day, b.invest_import_ppm, ONE),
        );
        let input_dom = split(input_day - input_imp, &b.input_mix[u]);
        let invest_dom = split(invest_day - invest_imp, &b.invest_mix);
        // Home orders: (industry, inputs, investment), in real units; and
        // which of the two it orders at all.
        let mut orders: Vec<(usize, i64, i64)> = Vec::new();
        let mut wants: Vec<(bool, bool)> = Vec::new();
        for (i, (&m, &k)) in input_dom.iter().zip(&invest_dom).enumerate() {
            let price = i128::from(self.industries.price_ppm[i].max(1));
            let q = |money: i128| i64::try_from(money * ONE / price).expect("a quantity");
            if m > 0 || k > 0 {
                orders.push((
                    i,
                    if m > 0 { q(m) } else { 0 },
                    if k > 0 { q(k) } else { 0 },
                ));
                wants.push((m > 0, k > 0));
            }
        }
        let rate = self.investment_tax_ppm();
        let taxes = |w: &Self, orders: &[(usize, i64, i64)]| -> i128 {
            orders
                .iter()
                .map(|&(i, m, k)| {
                    let p = i128::from(w.industries.price_ppm[i]);
                    let (vm, vk) = (ratio(i128::from(m), p, ONE), ratio(i128::from(k), p, ONE));
                    w.input_tax(u, vm)
                        + w.fuel_duty(u, i, m)
                        + if vk > 0 { ratio(vk, rate, ONE) } else { 0 }
                })
                .sum()
        };
        let mut owed = taxes(self, &orders);
        let free = self.uu.firm_free.max(0);
        if input_imp + invest_imp + owed > free {
            let (n, mut d) = (free, input_imp + invest_imp + owed);
            let original = orders.clone();
            let imp = (input_imp, invest_imp);
            loop {
                for (o, &(_, m, k)) in orders.iter_mut().zip(&original) {
                    o.1 = i64::try_from(i128::from(m) * n / d).expect("a quantity");
                    o.2 = i64::try_from(i128::from(k) * n / d).expect("a quantity");
                }
                (input_imp, invest_imp) = (imp.0 * n / d, imp.1 * n / d);
                owed = taxes(self, &orders);
                let total = input_imp + invest_imp + owed;
                if total <= free || n == 0 {
                    break;
                }
                d += total - free;
            }
        }
        for (&(i, m, k), &(wm, wk)) in orders.iter().zip(&wants) {
            if wm {
                self.demand.units[i].push((u, m));
            }
            if wk {
                self.demand.agents[i].push((u, k));
            }
        }
        self.uu.firm_free -= input_imp + invest_imp + owed;
        let imported = input_imp + invest_imp;
        if imported > 0 {
            self.units.deposits[u] -= bani(imported);
            post.deposits(
                Sector::Firms,
                Sector::RestOfWorld,
                bani(imported),
                FlowCode::IMPORTS,
            );
            self.mac.month.imports += imported;
            self.mac.month.inputs_imported += input_imp;
            self.mac.month.investment += invest_imp;
            self.fs.add_inputs(u, input_imp);
            self.bud.input_cash[u] -= input_imp;
            self.bud.invest_cash[u] -= invest_imp;
        }
    }

    /// The goods market: what a public unit paid for inputs and investment.
    pub(super) fn budget_paid(&mut self, u: usize, inputs: i128, investment: i128) {
        if self.budget_public(u) {
            self.bud.input_cash[u] -= inputs;
            self.bud.invest_cash[u] -= investment;
        }
    }

    /// The goods market: an industry's sales over its sellers. Where public
    /// units under the ministry budgets sell beside other units, the other
    /// units sell first, as far as their stocks go, and the public units
    /// serve the rest (spec game/lever-system, "Market sales"; owner,
    /// 2026-10-29); within each group by stocks. Elsewhere by stocks
    /// (production's rule).
    pub(super) fn budget_share_sales(
        &self,
        sold: i64,
        sellers: &[usize],
        stocks: &[i64],
    ) -> Vec<i64> {
        let public: Vec<bool> = sellers.iter().map(|&u| self.budget_public(u)).collect();
        if !public.iter().any(|&p| p) || public.iter().all(|&p| p) {
            return econ_mech_production::share_sales(sold, stocks);
        }
        let of = |p: bool| -> Vec<i64> {
            stocks
                .iter()
                .zip(&public)
                .map(|(&s, &q)| if q == p { s.max(0) } else { 0 })
                .collect()
        };
        let (others, publics) = (of(false), of(true));
        let other_sold = sold.min(others.iter().sum());
        let part = |total: i64, s: &[i64]| -> Vec<i64> {
            if total > 0 {
                econ_mech_production::share_sales(total, s)
            } else {
                vec![0; s.len()]
            }
        };
        part(other_sold, &others)
            .into_iter()
            .zip(part(sold - other_sold, &publics))
            .map(|(a, b)| a + b)
            .collect()
    }

    /// The goods market: what a public unit received for its market sales.
    pub(super) fn budget_sold(&mut self, u: usize, revenue: i128) {
        if self.budget_public(u) {
            self.bud.market_sales[u] += revenue;
        }
    }

    /// The government's top-up at cost of a public unit short of money.
    pub(super) fn budget_topup(&mut self, u: usize, amount: i128) {
        if self.budget_public(u) {
            self.bud.topup[u] += amount;
        }
    }

    /// The pay day, right after wages: each line's wages, inputs and
    /// investment; the government pays the units; the month's rows.
    #[allow(clippy::too_many_lines)]
    pub(super) fn budget_month_end(&mut self, labour: &[i128]) {
        if !self.bud.active {
            return;
        }
        let date: Date = self.calendar.date(self.today);
        // The pay phase also runs on day 0 (the starting slips); the budget
        // settles at month ends only.
        if !date.is_month_end() {
            return;
        }
        self.budget_market_wages();
        let month = self.calendar.month(self.today);
        if date.year != self.bud.year {
            self.bud.year = date.year;
            self.bud.ytd_allowance.iter_mut().for_each(|v| *v = 0);
            self.bud.ytd_paid.iter_mut().for_each(|v| *v = 0);
        }
        let k = date.month - 1;
        let n_l = self.bud.lines.len();
        let n_u = labour.len();
        let law = &self.lever_state().expect("a world under law.json").law;
        let money = |l: usize| match law.get(LeverId::SpendMinistryBudget, u32::try_from(l).ok()) {
            Some(LeverValue::Money(b)) => i128::from(b.get()).max(0),
            _ => 0,
        };
        let share_bp =
            |l: usize| match law.get(LeverId::SpendInvestmentShare, u32::try_from(l).ok()) {
                Some(LeverValue::Rate(r)) => i128::from(*r).clamp(0, 10_000),
                _ => 0,
            };
        let mut running = vec![0i128; n_l];
        let mut invest = vec![0i128; n_l];
        for l in 0..n_l {
            let year = money(l);
            let run_year = year * (10_000 - share_bp(l)) / 10_000;
            running[l] = twelfth(run_year, k);
            invest[l] = twelfth(year - run_year, k);
        }
        // Each unit's labour cost (and top-ups) shared among its lines.
        let mut wages = vec![0i128; n_l];
        let mut topped = vec![0i128; n_l];
        for u in 0..n_u {
            let lines = &self.bud.unit_lines[u];
            if lines.is_empty() {
                continue;
            }
            let w: Vec<i128> = lines.iter().map(|&(_, w)| w).collect();
            for (&(l, _), (p, t)) in lines.iter().zip(
                split(labour[u], &w)
                    .into_iter()
                    .zip(split(self.bud.topup[u], &w)),
            ) {
                wages[l] += p;
                topped[l] += t;
            }
        }
        let mut inputs = vec![0i128; n_l];
        let mut overrun = vec![0i128; n_l];
        let b = &mut self.bud;
        for l in 0..n_l {
            b.ytd_allowance[l] += running[l];
            let paid = wages[l] + topped[l];
            let avail = b.ytd_allowance[l] - b.ytd_paid[l];
            if paid > avail {
                overrun[l] = paid - avail.max(0);
            } else {
                inputs[l] = avail - paid;
            }
            b.ytd_paid[l] += paid + inputs[l];
        }
        // The government pays each unit: its labour cost, its lines' inputs
        // (running) and investment.
        let mut run_pay: Vec<i128> = (0..n_u)
            .map(|u| if b.public[u] { labour[u] } else { 0 })
            .collect();
        let mut inv_pay = vec![0i128; n_u];
        for l in 0..n_l {
            let units = &b.line_units[l];
            let w: Vec<i128> = units.iter().map(|&(_, w)| w).collect();
            for ((&(u, _), i), v) in units
                .iter()
                .zip(split(inputs[l], &w))
                .zip(split(invest[l], &w))
            {
                run_pay[u] += i;
                inv_pay[u] += v;
                b.input_cash[u] += i;
                b.invest_cash[u] += v;
            }
        }
        // The hiring freeze: a unit's labour cost above its lines' running
        // allowance of the month.
        for u in 0..n_u {
            if !b.public[u] {
                continue;
            }
            let allowance: i128 = (0..n_l)
                .map(|l| {
                    let units = &b.line_units[l];
                    let w: Vec<i128> = units.iter().map(|&(_, w)| w).collect();
                    units
                        .iter()
                        .zip(split(running[l], &w))
                        .filter(|((x, _), _)| *x == u)
                        .map(|(_, p)| p)
                        .sum::<i128>()
                })
                .sum();
            b.freeze[u] = labour[u] > allowance;
            b.topup[u] = 0;
        }
        self.budget_benefits(k);
        let mut post = Postings::default();
        let (mut run_total, mut inv_total) = (0i128, 0i128);
        for u in 0..n_u {
            if run_pay[u] > 0 {
                // Its non-market output, in real units at the day's price,
                // counts in its industry's output for the wage cost per unit
                // (owner, 2026-10-29).
                let j = usize::from(self.units.industry[u]);
                let price = i128::from(self.industries.price_ppm[j].max(1));
                self.ii.output_month[j] +=
                    i64::try_from(run_pay[u] * ONE / price).unwrap_or(i64::MAX);
                self.units.deposits[u] += bani(run_pay[u]);
                self.fs.add_value(u, run_pay[u]);
                self.fs.add_sales(u, run_pay[u]);
                run_total += run_pay[u];
            }
            if inv_pay[u] > 0 {
                self.units.deposits[u] += bani(inv_pay[u]);
                inv_total += inv_pay[u];
            }
        }
        // Public health and education in kind, for the standard of living.
        #[cfg(feature = "poverty")]
        {
            let (mut health, mut education) = (0i128, 0i128);
            for (u, &x) in run_pay.iter().enumerate() {
                match self.industries.id[usize::from(self.units.industry[u])].as_str() {
                    "health_services" => health += x,
                    "education_services" => education += x,
                    _ => {}
                }
            }
            self.pov_public_services(bani(health), bani(education));
        }
        post.gov_pays(Sector::Firms, bani(run_total), FlowCode::GOV_PURCHASES);
        post.gov_pays(Sector::Firms, bani(inv_total), FlowCode::GOV_INVESTMENT);
        // The units' market sales of the month go to the government (ESA
        // P11 + P12 + P131; owner, 2026-10-29), never more than a unit holds.
        let mut swept = 0i128;
        for u in 0..n_u {
            let s = std::mem::take(&mut self.bud.market_sales[u])
                .min(i128::from(self.units.deposits[u].get()))
                .max(0);
            if s > 0 {
                self.units.deposits[u] -= bani(s);
                self.fs.add_value(u, -s);
                self.fs.add_sales(u, -s);
                swept += s;
            }
        }
        post.pays_gov(Sector::Firms, bani(swept), FlowCode::GOV_SALES);
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
        // Non-market output at cost less the market sales (budget-2021; ESA
        // P13): output, sold to the government. Its market output is counted
        // where it is produced and sold.
        let non_market = run_total - swept;
        self.mac.month.output_value += non_market;
        self.mac.month.sales_value += non_market;
        self.mac.month.government += non_market;

        let rows: Vec<BudgetLineMonth> = (0..n_l)
            .map(|l| BudgetLineMonth {
                month,
                line: self.bud.lines[l].clone(),
                running_allowance_to_date: bani(self.bud.ytd_allowance[l]),
                running_paid_to_date: bani(self.bud.ytd_paid[l]),
                wages: bani(wages[l] + topped[l]),
                inputs: bani(inputs[l]),
                investment: bani(invest[l]),
                overrun: bani(overrun[l]),
            })
            .collect();
        let overruns: Vec<PolicyEvent> = rows
            .iter()
            .filter(|r| r.overrun > Bani::ZERO)
            .map(|r| PolicyEvent::BudgetOverrun {
                date,
                line: r.line.clone(),
                overrun: r.overrun,
            })
            .collect();
        self.bud.months.extend(rows);
        if let Some(st) = self.lever_state_mut() {
            st.events.extend(overruns);
        }
    }

    /// `spend.public_pay` in force: a change scales the unit's employees'
    /// pay by new over old.
    pub(crate) fn budget_refresh_pay(&mut self) {
        if !self.bud.active {
            return;
        }
        let n_u = self.units.industry.len();
        for u in 0..n_u {
            let Some(k) = self.bud.unit_key[u] else {
                continue;
            };
            let Some(LeverValue::Rate(bp)) = self
                .lever_state()
                .and_then(|s| s.law.get(LeverId::SpendPublicPay, Some(k)))
                .cloned()
            else {
                continue;
            };
            let new = i128::from(bp) * 100;
            let old = self.bud.pay_ppm[u];
            if new == old || old == 0 {
                continue;
            }
            self.bud.pay_ppm[u] = new;
            for p in 0..self.persons.employer.len() {
                if usize::from(self.persons.employer[p]) == u {
                    let scale = |b: Bani| bani(ratio(i128::from(b.get()), new, old));
                    self.persons.wage[p] = scale(self.persons.wage[p]);
                    self.persons.wage_declared[p] = scale(self.persons.wage_declared[p]);
                }
            }
        }
    }

    /// The pay of a public hire: the starting reference wage of its group and
    /// occupation times the unit's pay in force.
    pub(super) fn budget_hire_wage(
        &self,
        u: usize,
        group: usize,
        occupation: usize,
    ) -> Option<Bani> {
        if !self.budget_public(u) {
            return None;
        }
        let w = self.bud.start_reference.get(group)?.get(occupation)?;
        Some(bani(ratio(i128::from(w.get()), self.bud.pay_ppm[u], ONE)))
    }

    /// Every line in every closed month.
    pub(crate) fn budget_rows(&self) -> &[BudgetLineMonth] {
        &self.bud.months
    }
}
