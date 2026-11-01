//! The statistics registry's history on the `World` (feature `stats`; spec
//! `game/statistics-registry`): at the end of every day, after every phase,
//! the day indicators' points; on a close's day, the month's frame from the
//! state after the close and the mechanics' month records. Outputs only.

use econ_types::Bani;

use super::World;
use crate::stats::{
    Breakdown, Derive, DeriveOp, IndicatorId as I, Key, Period, StatsHistory, Status, derive_point,
};

/// A whole-ban value as a point.
fn b(x: Bani) -> i64 {
    x.get()
}

/// `n / d` rounded half away from zero; 0 for `d` = 0.
fn rdiv(n: i128, d: i128) -> i64 {
    if d == 0 {
        return 0;
    }
    let (an, ad) = (n.abs(), d.abs());
    let q = (2 * an + ad) / (2 * ad);
    i64::try_from(if (n < 0) == (d < 0) { q } else { -q }).unwrap_or(i64::MAX)
}

/// A month's points before they are pushed.
#[derive(Default)]
struct Frame {
    points: Vec<(I, Key, i64)>,
}

impl Frame {
    fn nat(&mut self, id: I, v: i64) {
        self.points.push((id, Key::NATIONAL, v));
    }
    fn at(&mut self, id: I, br: Breakdown, i: usize, v: i64) {
        self.points.push((id, Key::of(br, i), v));
    }
}

impl World {
    pub(crate) fn stats_history(&self) -> &StatsHistory {
        &self.stats
    }

    /// Day 0's CPI terms, the base of the first month's contributions.
    pub(super) fn stats_start(&mut self) {
        self.stats_cpi_terms = self.cpi_terms();
        #[cfg(feature = "explain")]
        self.xp_start();
    }

    /// Each good's term of the CPI now: basket share × consumer price.
    pub(super) fn cpi_terms(&self) -> Vec<i128> {
        let i = &self.industries;
        i.basket_share_ppm
            .iter()
            .zip(&i.consumer_price_ppm)
            .map(|(&s, &p)| i128::from(s) * i128::from(p))
            .collect()
    }

    /// `prices.cpi_contribution`: each good's part of the CPI's change over
    /// the month (against day 0 for month 0), its term's change rounded so
    /// that the parts add up to the change exactly.
    #[allow(clippy::cast_precision_loss)]
    fn cpi_contribution_points(&mut self, f: &mut Frame) {
        let t = self.months.len() - 1;
        let before = if t == 0 {
            1_000_000
        } else {
            self.months[t - 1].cpi_ppm
        };
        let change = i64::from(self.months[t].cpi_ppm) - i64::from(before);
        let now = self.cpi_terms();
        let parts: Vec<f64> = now
            .iter()
            .enumerate()
            .map(|(j, &x)| (x - self.stats_cpi_terms.get(j).copied().unwrap_or(0)) as f64 / 1e6)
            .collect();
        let c = crate::stats::round_to_sum(&parts, change);
        f.nat(I::PricesCpiContribution, change);
        for (j, &x) in c.iter().enumerate() {
            f.at(I::PricesCpiContribution, Breakdown::Good, j, x);
        }
        self.stats_cpi_terms = now;
    }

    /// The close keeps each industry's output of the month before it is
    /// reset (`industry.output_real`).
    pub(super) fn stats_capture_output(&mut self, j: usize, out: i64) {
        if self.stats_output.len() <= j {
            self.stats_output.resize(j + 1, 0);
        }
        self.stats_output[j] = out;
    }

    /// The end of a day: the day indicators' points, and on a close's day the
    /// month's frame.
    pub(super) fn stats_day_end(&mut self, closed: bool) {
        if !self.params.stats {
            return;
        }
        // The month the day belongs to: the one just closed on a close's
        // day, else the open one.
        let t = u32::try_from(self.months.len() - usize::from(closed)).expect("a month");
        let day = self.day_points();
        for &(id, key, v) in &day {
            self.stats.push_day(id, key, t, v);
        }
        if !closed {
            return;
        }
        let mut f = Frame::default();
        f.points.extend(day);
        self.month_points(&mut f);
        self.cpi_contribution_points(&mut f);
        #[cfg(feature = "explain")]
        self.xp_capture();
        for (id, key, v) in f.points {
            self.stats.push_month(id, key, t, v);
        }
        // Derived indicators, national, in the registry's order.
        for m in crate::stats::indicators() {
            let Some(d) = m.derive else { continue };
            if m.status != Status::Active {
                continue;
            }
            let (op, a, bb) = match d {
                Derive::Ratio(a, b) => (DeriveOp::Ratio, a, Some(b)),
                Derive::Real(a) => (DeriveOp::Real, a, Some(I::PricesCpi)),
                Derive::GrowthYoy(a) => (DeriveOp::GrowthYoy, a, None),
                Derive::GrowthMom(a) => (DeriveOp::GrowthMom, a, None),
                Derive::PerCapita(a) => (DeriveOp::PerCapita, a, Some(I::PeoplePopulation)),
            };
            let av = self.stats.national_from_zero(a, t);
            let bv = bb
                .map(|x| self.stats.national_from_zero(x, t))
                .unwrap_or_default();
            if let Some(v) = derive_point(op, &av, &bv, usize::try_from(t).expect("a month")) {
                self.stats.push_month(m.id, Key::NATIONAL, t, v);
            }
        }
        debug_assert!(
            crate::stats::indicators()
                .iter()
                .all(|m| m.period == Period::Month || m.period == Period::Day)
        );
    }

    /// The day indicators' points now.
    fn day_points(&self) -> Vec<(I, Key, i64)> {
        let mut v = Vec::new();
        #[cfg(feature = "trade")]
        v.push((
            I::FxEurLei,
            Key::NATIONAL,
            i64::from(self.foreign().exchange_rate_ppm),
        ));
        #[cfg(feature = "monetary")]
        v.push((
            I::MoneyPolicyRate,
            Key::NATIONAL,
            i64::from(self.central_bank().policy_rate_ppm),
        ));
        let cpi = self.months.last().map_or(1_000_000, |m| m.cpi_ppm);
        v.push((I::PricesGood, Key::NATIONAL, i64::from(cpi)));
        for (j, &p) in self.industries.price_ppm.iter().enumerate() {
            v.push((I::PricesGood, Key::of(Breakdown::Good, j), i64::from(p)));
        }
        v
    }

    /// The month indicators of the close just made.
    #[allow(clippy::too_many_lines, clippy::needless_range_loop)]
    fn month_points(&self, f: &mut Frame) {
        let t = self.months.len() - 1;
        let tm = u32::try_from(t).expect("a month");
        let m = &self.months[t];
        f.nat(I::GdpNominal, b(m.gdp_production));
        f.nat(I::PricesCpi, i64::from(m.cpi_ppm));
        f.nat(I::PricesInflationYoy, i64::from(m.inflation_ppm));
        let before = if t == 0 {
            1_000_000
        } else {
            self.months[t - 1].cpi_ppm
        };
        f.nat(
            I::PricesInflationMom,
            rdiv(
                (i128::from(m.cpi_ppm) - i128::from(before)) * 1_000_000,
                i128::from(before),
            ),
        );
        f.nat(
            I::PricesExpectedInflation,
            i64::from(m.expected_inflation_ppm),
        );
        f.nat(
            I::LabourEmployed,
            i64::try_from(m.employed).unwrap_or(i64::MAX),
        );
        f.nat(
            I::LabourUnemploymentRate,
            rdiv(
                i128::from(m.unemployed) * 1_000_000,
                i128::from(m.employed + m.unemployed),
            ),
        );
        f.nat(
            I::WagesMeanNominal,
            rdiv(i128::from(m.wage_bill.get()), i128::from(m.employed)),
        );
        // Households' money, weighted.
        let h = &self.households;
        let weighted = |col: &[Bani]| -> i64 {
            i64::try_from(
                h.hh_weight
                    .iter()
                    .zip(col)
                    .map(|(&w, x)| i128::from(w) * i128::from(x.get()))
                    .sum::<i128>(),
            )
            .unwrap_or(i64::MAX)
        };
        f.nat(I::MoneyDepositsHouseholds, weighted(&h.hh_deposits));
        f.nat(I::MoneyCash, weighted(&h.hh_cash));
        // Industries: output (captured at the close), price, staff.
        let n_i = self.industries.id.len();
        let out: i64 = (0..n_i)
            .map(|j| self.stats_output.get(j).copied().unwrap_or(0))
            .sum();
        f.nat(I::IndustryOutputReal, out);
        f.nat(I::IndustryPrice, i64::from(m.cpi_ppm));
        let mut staff = vec![0i64; n_i];
        for (u, &s) in self.units.staff.iter().enumerate() {
            staff[usize::from(self.units.industry[u])] += i64::try_from(s).unwrap_or(0);
        }
        f.nat(I::IndustryEmployment, staff.iter().sum());
        for j in 0..n_i {
            f.at(
                I::IndustryOutputReal,
                Breakdown::Industry,
                j,
                self.stats_output.get(j).copied().unwrap_or(0),
            );
            f.at(
                I::IndustryPrice,
                Breakdown::Industry,
                j,
                i64::from(self.industries.price_ppm[j]),
            );
            f.at(I::IndustryEmployment, Breakdown::Industry, j, staff[j]);
        }
        // Residents by region (and county with the People phase).
        self.population_points(f);
        #[cfg(feature = "fiscal")]
        self.fiscal_points(f, tm);
        #[cfg(feature = "levers")]
        self.line_points(f, tm);
        #[cfg(feature = "revenue")]
        if let Some(r) = self.revenue_months().iter().find(|r| r.month.0 == tm) {
            f.nat(I::BudgetEuNetBalance, b(r.eu_net_balance));
        }
        #[cfg(feature = "banking")]
        {
            let bank = self.bank();
            let debt: Vec<Bani> = (0..h.hh_weight.len())
                .map(|k| {
                    bank.hh_mortgage.get(k).copied().unwrap_or(Bani::ZERO)
                        + bank.hh_consumer_debt.get(k).copied().unwrap_or(Bani::ZERO)
                })
                .collect();
            f.nat(I::MoneyCreditHouseholds, weighted(&debt));
            f.nat(I::MoneyCreditFirms, b(bank.firm_loans));
        }
        #[cfg(feature = "trade")]
        self.trade_points(f, tm);
        #[cfg(feature = "investment")]
        if let Some(inv) = self.investment_months().iter().find(|x| x.month.0 == tm) {
            let mut by = vec![0i64; n_i];
            for (u, x) in inv.spent.iter().enumerate() {
                if let Some(&j) = self.units.industry.get(u) {
                    by[usize::from(j)] += b(*x);
                }
            }
            f.nat(I::IndustryInvestment, by.iter().sum());
            for (j, &x) in by.iter().enumerate() {
                f.at(I::IndustryInvestment, Breakdown::Industry, j, x);
            }
        }
        #[cfg(feature = "poverty")]
        self.poverty_points(f, tm);
        #[cfg(feature = "approval")]
        self.approval_points(f, tm);
        #[cfg(feature = "wellbeing")]
        if let Some(w) = self.wellbeing_stats().iter().find(|x| x.month.0 == tm) {
            f.nat(I::PeopleWellbeing, i64::from(w.national.index_ppm));
            for (r, x) in w.by_region.iter().enumerate() {
                f.at(
                    I::PeopleWellbeing,
                    Breakdown::Region,
                    r,
                    i64::from(x.index_ppm),
                );
            }
            f.nat(I::PeopleLifeExpectancy, i64::from(w.life_expectancy_milli));
        }
        #[cfg(feature = "migration")]
        self.migration_points(f, tm);
        #[cfg(feature = "education")]
        self.education_points(f);
    }

    fn population_points(&self, f: &mut Frame) {
        let n_r = usize::from(self.pp.region.iter().copied().max().unwrap_or(0)) + 1;
        let mut by_r = vec![0i64; n_r];
        #[cfg(feature = "demography")]
        let n_c = usize::from(
            self.dem
                .hh_life
                .hh_county
                .iter()
                .copied()
                .max()
                .unwrap_or(0),
        ) + 1;
        #[cfg(feature = "demography")]
        let mut by_c = vec![0i64; n_c];
        for p in 0..self.pp.age.len() {
            if !self.person_resident(p) {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            let w = i64::from(self.households.hh_weight[h]);
            by_r[usize::from(self.pp.region[p])] += w;
            #[cfg(feature = "demography")]
            {
                by_c[usize::from(self.dem.hh_life.hh_county[h])] += w;
            }
        }
        f.nat(I::PeoplePopulation, by_r.iter().sum());
        for (r, &x) in by_r.iter().enumerate() {
            f.at(I::PeoplePopulation, Breakdown::Region, r, x);
        }
        #[cfg(feature = "demography")]
        for (c, &x) in by_c.iter().enumerate() {
            f.at(I::PeoplePopulation, Breakdown::County, c, x);
        }
    }

    #[cfg(feature = "fiscal")]
    fn fiscal_points(&self, f: &mut Frame, tm: u32) {
        let Some(fm) = self.fiscal_months().iter().find(|x| x.month.0 == tm) else {
            return;
        };
        f.nat(I::BudgetDeficit, b(fm.deficit));
        f.nat(I::BudgetBalance, -b(fm.deficit));
        f.nat(I::BudgetInterest, b(fm.interest));
        for (id, lines) in [
            (I::BudgetRevenue, &fm.revenue),
            (I::BudgetSpending, &fm.spending),
        ] {
            let mut total = 0i64;
            for code in econ_ledger::FlowCode::ALL {
                let v: i64 = lines
                    .iter()
                    .filter(|(c, _)| *c == code)
                    .map(|(_, x)| b(*x))
                    .sum();
                total += v;
                f.at(id, Breakdown::Flow, usize::from(code.0), v);
            }
            // Codes outside the ledger's list, if any, still count nationally.
            let rest: i64 = lines
                .iter()
                .filter(|(c, _)| !econ_ledger::FlowCode::ALL.contains(c))
                .map(|(_, x)| b(*x))
                .sum();
            f.nat(id, total + rest);
        }
        let debt = self.treasury().book.total();
        f.nat(I::DebtStock, b(debt));
        let t = usize::try_from(tm).expect("a month");
        let from = t.saturating_sub(11);
        let gdp: i128 = self.months[from..=t]
            .iter()
            .map(|m| i128::from(m.gdp_production.get()))
            .sum();
        let n = i128::try_from(t - from + 1).expect("months");
        f.nat(
            I::DebtGdpRatio,
            rdiv(i128::from(debt.get()) * 1_000_000 * n, gdp * 12),
        );
    }

    #[cfg(feature = "levers")]
    fn line_points(&self, f: &mut Frame, tm: u32) {
        let lines: Vec<&crate::levers::BudgetLineMonth> = self
            .budget_months()
            .iter()
            .filter(|l| l.month.0 == tm)
            .collect();
        let mut total = 0i64;
        for (k, l) in lines.iter().enumerate() {
            let v = b(l.wages) + b(l.inputs) + b(l.investment);
            total += v;
            f.at(I::BudgetLinePaid, Breakdown::Line, k, v);
        }
        f.nat(I::BudgetLinePaid, total);
    }

    #[cfg(feature = "trade")]
    fn trade_points(&self, f: &mut Frame, tm: u32) {
        f.nat(
            I::MoneyCbForeignAssets,
            b(self.ledger.balance(
                econ_ledger::Sector::CentralBank,
                econ_ledger::Instrument::ForeignAssets,
            )),
        );
        if let Some(x) = self.trade_months().iter().find(|x| x.month.0 == tm) {
            f.nat(I::TradeCurrentAccount, b(x.current_account));
            f.nat(I::TradeCapitalAccount, b(x.capital_account));
            f.nat(
                I::TradeCurrentAccountProper,
                b(x.current_account) - b(x.capital_account),
            );
            f.nat(I::TradeExportsVolume, x.exports_volume);
            f.nat(I::FxIntervention, b(x.intervention));
        }
        if let Some(r) = self.remittance_months().iter().find(|x| x.month.0 == tm) {
            f.nat(I::PeopleRemittances, b(r.lei));
        }
    }

    #[cfg(feature = "poverty")]
    fn poverty_points(&self, f: &mut Frame, tm: u32) {
        let Some(s) = self.poverty_stats().iter().find(|x| x.month.0 == tm) else {
            return;
        };
        f.nat(I::PeopleGini, i64::from(s.national.gini_ppm));
        f.nat(I::PeoplePovertyRate, i64::from(s.national.poverty_rate_ppm));
        for (r, x) in s.by_region.iter().enumerate() {
            f.at(
                I::PeoplePovertyRate,
                Breakdown::Region,
                r,
                i64::from(x.poverty_rate_ppm),
            );
        }
        for (c, x) in s.by_county.iter().enumerate() {
            f.at(
                I::PeoplePovertyRate,
                Breakdown::County,
                c,
                i64::from(x.poverty_rate_ppm),
            );
        }
        f.nat(
            I::PeoplePovertyRateAnchored,
            i64::from(s.class_share_ppm[0]),
        );
        f.nat(I::PeopleClassShare, i64::from(s.class_share_ppm[0]));
        for (k, &x) in s.class_share_ppm.iter().enumerate() {
            f.at(I::PeopleClassShare, Breakdown::Class, k, i64::from(x));
        }
        f.nat(I::PeopleHardshipRate, i64::from(s.hardship_rate_ppm));
        f.nat(
            I::PeopleS80S20,
            i64::from(s.national.s80_s20_ppm.unwrap_or(0)),
        );
        let sol = &s.sol;
        f.nat(I::PeopleSol, i64::from(sol.national_milli));
        for (br, v) in [
            (Breakdown::Region, &sol.by_region[..]),
            (Breakdown::County, &sol.by_county[..]),
            (Breakdown::Class, &sol.by_class[..]),
            (Breakdown::Fifth, &sol.by_fifth[..]),
            (Breakdown::AgeBand, &sol.by_age[..]),
        ] {
            for (k, &x) in v.iter().enumerate() {
                f.at(I::PeopleSol, br, k, i64::from(x));
            }
        }
    }

    #[cfg(feature = "approval")]
    fn approval_points(&self, f: &mut Frame, tm: u32) {
        let Some(s) = self.approval_stats().iter().find(|x| x.month.0 == tm) else {
            return;
        };
        if let Some(n) = &s.national {
            f.nat(I::ApprovalNational, i64::from(n.mean_milli));
            f.nat(I::ApprovalByGroup, i64::from(n.mean_milli));
        }
        for (br, v) in [
            (Breakdown::Activity, &s.by_activity[..]),
            (Breakdown::Fifth, &s.by_fifth[..]),
            (Breakdown::Region, &s.by_region[..]),
            (Breakdown::County, &s.by_county[..]),
            (Breakdown::AgeBand, &s.by_age[..]),
            (Breakdown::Edu, &s.by_edu[..]),
            (Breakdown::Sex, &s.by_sex[..]),
        ] {
            for (k, g) in v.iter().enumerate() {
                if let Some(g) = g {
                    f.at(I::ApprovalByGroup, br, k, i64::from(g.mean_milli));
                }
            }
        }
    }

    #[cfg(feature = "migration")]
    fn migration_points(&self, f: &mut Frame, tm: u32) {
        let Some(m) = self.migration_months().iter().find(|x| x.month.0 == tm) else {
            return;
        };
        let p = |x: u64| i64::try_from(x).unwrap_or(i64::MAX);
        f.nat(I::PeopleBirths, p(m.births));
        f.nat(I::PeopleDeaths, p(m.deaths_resident + m.deaths_abroad));
        f.nat(I::PeopleEmigrants, p(m.emigrants));
        f.nat(I::PeopleReturns, p(m.returns));
        for (r, &x) in m.returns_by_region.iter().enumerate() {
            f.at(I::PeopleReturns, Breakdown::Region, r, p(x));
        }
        f.nat(I::PeopleImmigrants, p(m.immigrants));
        let d = self.diaspora();
        f.nat(
            I::PeopleDiaspora,
            p(d.linked + d.stock.iter().map(|c| c.persons).sum::<u64>()),
        );
    }

    #[cfg(feature = "education")]
    fn education_points(&self, f: &mut Frame) {
        let mut by = [0i64; 7];
        for (p, &e) in self.people.enrolled.iter().enumerate() {
            if e == econ_popgen::Enrolled::None || !self.person_resident(p) {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            by[e as usize - 1] += i64::from(self.households.hh_weight[h]);
        }
        f.nat(I::EducationEnrolled, by.iter().sum());
        for (l, &x) in by.iter().enumerate() {
            f.at(I::EducationEnrolled, Breakdown::Level, l, x);
        }
        f.nat(
            I::EducationEarlyLeaversRate,
            i64::from(self.early_leavers_rate_ppm()),
        );
        f.nat(
            I::EducationTertiaryAttainment,
            i64::from(self.tertiary_attainment_ppm()),
        );
    }
}
