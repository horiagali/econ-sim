//! Phases 1, 12 and 13 (spec `game/world-loop`): the one-off transfer of a
//! starting edit, the monthly close (prices steps 5 and 6, the income
//! fifths, tightness, GDP three ways) and the daily checks; the state hash.

use econ_ledger::{FlowCode, Instrument, InvariantError, Sector};
use econ_mech_prices::expectations_ppm;
use econ_num::math::powf;
use econ_popgen::NO_EMPLOYER;
use econ_types::{Bani, Month};

use super::load::{qualifies, tier};
use super::money::{Postings, bani, ratio};
use super::{MonthAcc, MonthStats, World};
use crate::StateHasher;

/// `cb_credibility` (constant in M2) and `expectation_learning`, millionths.
const CREDIBILITY_PPM: u32 = 700_000;
const LEARNING_PPM: u32 = 100_000;
const ONE: i128 = 1_000_000;

/// Each household's income fifth (0 = poorest) by its value, weighted by
/// households, and the lowest value of fifths 1 to 4.
pub(super) fn fifths(values: &[i128], weights: &[u32]) -> (Vec<u8>, [i128; 4]) {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by_key(|&h| (values[h], h));
    let total: u128 = weights.iter().map(|&w| u128::from(w)).sum();
    let mut fifth = vec![0u8; values.len()];
    let mut bounds = [i128::MAX; 4];
    let mut seen = 0u128;
    for h in order {
        let q = seen
            .saturating_mul(5)
            .checked_div(total)
            .map_or(0, |q| q.min(4));
        let q = u8::try_from(q).expect("a fifth");
        if q > 0 && bounds[usize::from(q - 1)] == i128::MAX {
            bounds[usize::from(q - 1)] = values[h];
        }
        fifth[h] = q;
        seen += u128::from(weights[h]);
    }
    // A fifth nobody entered starts where the next does.
    for q in (0..3).rev() {
        bounds[q] = bounds[q].min(bounds[q + 1]);
    }
    (fifth, bounds)
}

/// The fifth of a value against the bounds of [`fifths`].
pub(super) fn fifth_of(value: i128, bounds: &[i128; 4]) -> u8 {
    u8::try_from(bounds.iter().filter(|&&b| value >= b).count()).expect("a fifth")
}

/// Vacancies over the seekers qualified for them: `vacancies` / Σ_t (share
/// of tier t in `mix`) × `seekers[t]`, millionths.
pub(super) fn tightness_ppm(vacancies: i128, mix: &[u64; 3], seekers: &[u64; 3]) -> u32 {
    let total: i128 = mix.iter().map(|&v| i128::from(v)).sum();
    if total == 0 || vacancies <= 0 {
        return 0;
    }
    let reach: i128 = mix
        .iter()
        .zip(seekers)
        .map(|(&m, &s)| i128::from(m) * i128::from(s))
        .sum::<i128>()
        / total;
    if reach <= 0 {
        return 0;
    }
    u32::try_from(ratio(vacancies, ONE, reach).min(i128::from(u32::MAX))).expect("a tightness")
}

/// FNV-1a over 64-bit words in four interleaved lanes, folded into one
/// [`StateHasher`] at the end: the same words in the same order give the same
/// hash, four times faster than one serial chain.
struct Lanes {
    lanes: [u64; 4],
    n: usize,
}

impl Lanes {
    fn new() -> Self {
        Lanes {
            lanes: [
                0xcbf2_9ce4_8422_2325,
                0x8422_2325_cbf2_9ce4,
                0x9ce4_cbf2_2325_8422,
                0x2325_8422_9ce4_cbf2,
            ],
            n: 0,
        }
    }

    fn write_word(&mut self, w: u64) {
        let lane = &mut self.lanes[self.n & 3];
        *lane = (*lane ^ w).wrapping_mul(0x0100_0000_01b3);
        self.n = self.n.wrapping_add(1);
    }

    /// [`Lanes::write_word`] for each word in turn, with the lanes held in
    /// locals: the same hash, without a store and load per word.
    fn write_words(&mut self, words: impl IntoIterator<Item = u64>) {
        const P: u64 = 0x0100_0000_01b3;
        let mut words = words.into_iter();
        // Up to the first lane.
        while self.n & 3 != 0 {
            let Some(w) = words.next() else { return };
            self.write_word(w);
        }
        let [mut a, mut b, mut c, mut d] = self.lanes;
        let mut n = self.n;
        while let Some(w) = words.next() {
            a = (a ^ w).wrapping_mul(P);
            n = n.wrapping_add(1);
            let Some(w) = words.next() else { break };
            b = (b ^ w).wrapping_mul(P);
            n = n.wrapping_add(1);
            let Some(w) = words.next() else { break };
            c = (c ^ w).wrapping_mul(P);
            n = n.wrapping_add(1);
            let Some(w) = words.next() else { break };
            d = (d ^ w).wrapping_mul(P);
            n = n.wrapping_add(1);
        }
        self.lanes = [a, b, c, d];
        self.n = n;
    }

    fn finish(&self) -> u64 {
        let mut h = StateHasher::default();
        for &lane in &self.lanes {
            h.write_word(lane);
        }
        h.write_word(self.n as u64);
        h.finish()
    }
}

impl World {
    /// Phase 1 on day 0: the one-off transfer of a starting edit, paid by the
    /// government as pensions are. Commands themselves come in M4.
    pub(super) fn pay_transfer(&mut self) {
        if std::mem::take(&mut self.mac.start_pay) {
            // November's pay, the day before the start: it is income received
            // before December, so it counts in no month's statistics.
            self.pay();
            // The public producers' November wages, paid by the government at
            // cost: November's output, not December's.
            let before = self.mac.month.clone();
            self.settle_credit();
            self.mac.month = before;
            self.mac.month.wages = 0;
            self.ii.wages_month.iter_mut().for_each(|w| *w = Bani::ZERO);
            self.hh
                .income_this_month
                .iter_mut()
                .for_each(|w| *w = Bani::ZERO);
        }
        let transfer = std::mem::take(&mut self.mac.transfer);
        if transfer.is_empty() {
            return;
        }
        let mut total = 0i128;
        for (h, &a) in transfer.iter().enumerate() {
            if a <= Bani::ZERO {
                continue;
            }
            let hh = &mut self.households;
            hh.hh_deposits[h] += a;
            hh.hh_income_received[h] += a;
            hh.income_to_date[h] += a;
            self.hh.income_this_month[h] += a;
            total += i128::from(hh.hh_weight[h]) * i128::from(a.get());
        }
        let mut post = Postings::default();
        post.gov_pays(Sector::Households, bani(total), FlowCode::BENEFIT_OTHER);
        post.commit(&mut self.ledger);
    }

    /// The central bank's credibility once it exists, else the constant of
    /// M2's expectations.
    fn expectations_credibility(&self) -> u32 {
        #[cfg(feature = "monetary")]
        if let Some(c) = self.cb_credibility() {
            return c;
        }
        CREDIBILITY_PPM
    }

    /// Phase 12: the month's statistics and what next month runs on.
    #[allow(clippy::too_many_lines)]
    pub(super) fn monthly_close(&mut self) -> Vec<InvariantError> {
        let t = self.months.len();
        // CPI, inflation, expectations.
        let i = &self.industries;
        let cpi: i128 = i
            .basket_share_ppm
            .iter()
            .zip(&i.consumer_price_ppm)
            .map(|(&s, &p)| i128::from(s) * i128::from(p))
            .sum::<i128>()
            / ONE;
        let cpi = u32::try_from(cpi).expect("a price index");
        self.mac.cpi.push(cpi);
        let inflation_ppm = if t >= 12 {
            let base = i128::from(self.mac.cpi[t - 12]);
            i32::try_from(ratio(i128::from(cpi), ONE, base) - ONE).expect("an inflation rate")
        } else {
            let level = f64::from(cpi) / 1e6;
            let exponent = 12.0 / f64::from(u32::try_from(t + 1).expect("a month"));
            let annual = powf(level, exponent) - 1.0;
            // A few hundred thousand millionths at most.
            #[allow(clippy::cast_possible_truncation)]
            let v = (annual * 1e6).round().clamp(-1e9, 1e9) as i32;
            v
        };
        // The previous expectation without last month's push of monetary
        // financing: the push is a level, not a ratchet.
        #[cfg(feature = "monetary")]
        let previous = self.mac.expected_inflation_ppm - self.cb_expectations_push();
        #[cfg(not(feature = "monetary"))]
        let previous = self.mac.expected_inflation_ppm;
        self.mac.expected_inflation_ppm = expectations_ppm(
            previous,
            inflation_ppm,
            self.mac.target_ppm,
            self.expectations_credibility(),
            LEARNING_PPM,
        );
        // The central bank's financing of the deficit unanchors them upward.
        #[cfg(feature = "monetary")]
        {
            self.mac.expected_inflation_ppm += self.cb_next_push();
        }
        // People: employment, seekers by tier, tightness by group.
        let n_p = self.persons.employer.len();
        let weight =
            |p: usize| u64::from(self.households.hh_weight[self.persons.household_id[p] as usize]);
        let (mut employed, mut unemployed) = (0u64, 0u64);
        let mut seekers = [0u64; 3];
        let groups = self.mac.normal_tightness.len();
        let mut mix = vec![[0u64; 3]; groups];
        for p in 0..n_p {
            let w = weight(p);
            if self.pp.seeking[p] {
                unemployed += w;
                for (k, s) in seekers.iter_mut().enumerate() {
                    if qualifies(k, self.persons.edu_level[p]) {
                        *s += w;
                    }
                }
            } else if self.pp.status[p] != 0 {
                employed += w;
            }
            let e = self.persons.employer[p];
            if e != NO_EMPLOYER
                && let Some(k) = tier(self.persons.occupation[p])
            {
                let g = usize::from(
                    self.industries.group[usize::from(self.units.industry[usize::from(e)])],
                );
                mix[g][k] += w;
            }
        }
        let mut vacancies = vec![0i128; groups];
        let mut tier_vacancies = [0u64; 3];
        for u in 0..self.units.industry.len() {
            let g = usize::from(self.industries.group[usize::from(self.units.industry[u])]);
            for (k, v) in self.uu.vacancies_month[u].iter_mut().enumerate() {
                vacancies[g] += i128::from(*v);
                tier_vacancies[k] += *v;
                *v = 0;
            }
        }
        for g in 0..groups {
            let theta = tightness_ppm(vacancies[g], &mix[g], &seekers);
            self.mac.tightness[g].push(theta);
            let len = self.mac.tightness[g].len();
            if len > 12 {
                self.mac.tightness[g].drain(..len - 12);
            }
        }
        self.mac.tier_seekers = seekers;
        self.mac.tier_vacancies = tier_vacancies;
        for p in 0..n_p {
            if self.pp.seeking[p] {
                self.persons.months_unemployed[p] =
                    self.persons.months_unemployed[p].saturating_add(1);
            }
        }
        // Wage cost per unit of output for the prices of next month.
        for j in 0..self.industries.id.len() {
            let out = self.ii.output_month[j];
            if out > 0 {
                let c = ratio(
                    i128::from(self.ii.wages_month[j].get()),
                    ONE,
                    i128::from(out),
                );
                self.ii.wage_cost_ppm[j] =
                    u32::try_from(c.clamp(0, i128::from(u32::MAX))).expect("small");
            }
            self.ii.output_month[j] = 0;
            self.ii.wages_month[j] = Bani::ZERO;
        }
        // The income fifths of next month.
        let n_hh = self.households.hh_weight.len();
        let per_adult: Vec<i128> = (0..n_hh)
            .map(|h| {
                i128::from(self.hh.income_this_month[h].get()) * 1_000
                    / i128::from(self.hh.eq_adults_milli[h].max(1))
            })
            .collect();
        let (_, bounds) = fifths(&per_adult, &self.households.hh_weight);
        self.mac.fifth_bounds = bounds;
        self.hh.income_last_month =
            std::mem::replace(&mut self.hh.income_this_month, vec![Bani::ZERO; n_hh]);
        // GDP three ways.
        let m = std::mem::take(&mut self.mac.month);
        let MonthAcc {
            output_value,
            inputs_home,
            inputs_imported,
            vat,
            consumption,
            government,
            investment,
            exports,
            imports,
            sales_value,
            wages,
            employer_contributions,
        } = m;
        let production = output_value - inputs_home - inputs_imported + vat;
        let expenditure =
            consumption + government + investment + exports + (output_value - sales_value)
                - imports;
        let surplus = output_value - inputs_home - inputs_imported - wages - employer_contributions;
        let income = wages + employer_contributions + surplus + vat;
        let consumption = bani(consumption);
        if t == 0 {
            self.mac.first_consumption = consumption;
        }
        self.mac.last_consumption = consumption;
        let hardship = (0..n_hh)
            .filter(|&h| self.households.hh_hardship[h])
            .map(|h| u64::from(self.households.hh_weight[h]))
            .sum();
        let shortages = std::mem::replace(
            &mut self.ii.shortage_month,
            vec![0; self.industries.id.len()],
        );
        let stats = MonthStats {
            month: Month(u32::try_from(t).expect("a month")),
            cpi_ppm: cpi,
            inflation_ppm,
            expected_inflation_ppm: self.mac.expected_inflation_ppm,
            employed,
            unemployed,
            wage_bill: bani(wages),
            consumption,
            gdp_production: bani(production),
            gdp_expenditure: bani(expenditure),
            gdp_income: bani(income),
            shortages,
            hardship_households: hardship,
        };
        let mut failed = Vec::new();
        if stats.gdp_production != stats.gdp_expenditure || stats.gdp_production != stats.gdp_income
        {
            failed.push(InvariantError::GdpMismatch {
                production: stats.gdp_production,
                expenditure: stats.gdp_expenditure,
                income: stats.gdp_income,
            });
        }
        self.months.push(stats);
        // The treasury: the units' shortfalls of the day settled first, so
        // its deposits at the central bank end the day at their target.
        #[cfg(feature = "fiscal")]
        if self.tr_active() {
            self.settle_credit();
            self.treasury_payments();
        }
        // The banks after it (they move no government money): interest on
        // what the month's payments left, repayments, write-offs, rates.
        #[cfg(feature = "banking")]
        {
            self.bank_month_end();
            self.bank_close();
        }
        // The other taxes on production, then each unit's profit and capital,
        // its interest known.
        #[cfg(feature = "revenue")]
        self.rev_close();
        #[cfg(feature = "investment")]
        self.invest_close();
        // The central bank's interest and its profit, remitted before the
        // treasury's financing.
        #[cfg(feature = "monetary")]
        self.cb_month_end();
        // The exchange rate: the month's current account is complete.
        #[cfg(feature = "trade")]
        self.trade_close();
        // The treasury's financing last: the day's shortfalls settled, so its
        // deposits at the central bank end the day at their target.
        #[cfg(feature = "fiscal")]
        if self.tr_active() {
            self.settle_credit();
            self.treasury_financing();
        }
        #[cfg(feature = "monetary")]
        self.cb_close();
        failed
    }

    /// Phase 13: the ledger's invariants, the tables against the ledger
    /// (I-4), no negative money or inventory, the clearing accounts (I-8).
    pub(super) fn checks(&mut self) -> Vec<InvariantError> {
        self.settle_credit();
        #[cfg(feature = "banking")]
        {
            self.bank_note_firm_credit();
            self.bank_day_end();
        }
        let mut failed = match self.ledger.check_invariants() {
            Ok(()) => Vec::new(),
            Err(e) => e,
        };
        let h = &self.households;
        let weighted = |col: &[Bani]| -> Bani {
            bani(
                h.hh_weight
                    .iter()
                    .zip(col)
                    .map(|(&w, v)| i128::from(w) * i128::from(v.get()))
                    .sum(),
            )
        };
        let total = |col: &[Bani]| -> Bani { bani(col.iter().map(|v| i128::from(v.get())).sum()) };
        for (records, sector, instrument) in [
            (weighted(&h.hh_cash), Sector::Households, Instrument::Cash),
            (
                weighted(&h.hh_deposits),
                Sector::Households,
                Instrument::Deposits,
            ),
            (total(&self.units.currency), Sector::Firms, Instrument::Cash),
            (
                total(&self.units.deposits),
                Sector::Firms,
                Instrument::Deposits,
            ),
        ] {
            let ledger = self.ledger.balance(sector, instrument);
            if records != ledger {
                failed.push(InvariantError::SubLedgerMismatch {
                    sector,
                    instrument,
                    records,
                    ledger,
                });
            }
        }
        for (column, values) in [
            ("hh_cash", &h.hh_cash),
            ("hh_deposits", &h.hh_deposits),
            ("currency", &self.units.currency),
            ("deposits", &self.units.deposits),
        ] {
            if let Some(record) = values.iter().position(|v| v.get() < 0) {
                failed.push(InvariantError::NegativeRecord { column, record });
            }
        }
        if let Some(record) = self.units.inventory.iter().position(|&v| v < 0) {
            failed.push(InvariantError::NegativeRecord {
                column: "inventory",
                record,
            });
        }
        for (code, industry, sum) in std::mem::take(&mut self.mac.clearing) {
            failed.push(InvariantError::ClearingNotZero {
                code,
                industry,
                sum,
            });
        }
        failed
    }

    /// The hash of the whole state (FNV-1a 64 over every column in a fixed
    /// order).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut h = Lanes::new();
        // Word by word: the persons' columns are millions of values a day.
        #[allow(clippy::cast_sign_loss)]
        let b = |h: &mut Lanes, v: &[Bani]| h.write_words(v.iter().map(|x| x.get() as u64));
        #[allow(clippy::cast_sign_loss)]
        let i = |h: &mut Lanes, v: &[i64]| h.write_words(v.iter().map(|&x| x as u64));
        let u = |h: &mut Lanes, v: &[u64]| h.write_words(v.iter().copied());
        // Byte columns eight bytes to a word.
        let bytes = |h: &mut Lanes, v: &mut dyn Iterator<Item = u8>| {
            let mut word = 0u64;
            let mut n = 0u32;
            for x in v {
                word |= u64::from(x) << (8 * n);
                n += 1;
                if n == 8 {
                    h.write_word(word);
                    (word, n) = (0, 0);
                }
            }
            h.write_word(word ^ u64::from(n) << 56);
        };
        h.write_word(u64::from(self.today.0));
        let hh = &self.households;
        b(&mut h, &hh.hh_cash);
        b(&mut h, &hh.hh_deposits);
        b(&mut h, &hh.hh_income_received);
        h.write_words((0..hh.hh_weight.len()).map(|k| {
            u64::from(hh.hh_stagger_day[k])
                | u64::from(hh.hh_hardship[k]) << 8
                | u64::from(hh.hh_fifth[k]) << 16
        }));
        b(&mut h, &self.hh.income_this_month);
        b(&mut h, &self.hh.income_last_month);
        let p = &self.persons;
        // A person's small fields in one word.
        h.write_words((0..p.employer.len()).map(|i| {
            u64::from(p.employer[i])
                | u64::from(p.months_unemployed[i]) << 16
                | u64::from(p.occupation[i]) << 32
                | u64::from(self.pp.status[i]) << 40
                | u64::from(self.pp.seeking[i]) << 48
        }));
        b(&mut h, &p.wage);
        b(&mut h, &p.wage_declared);
        h.write_words(
            p.hired_on
                .iter()
                .map(|x| u64::from(x.map_or(u32::MAX, |d| d.0))),
        );
        b(&mut h, &p.last_pay);
        let f = &self.units;
        u(&mut h, &f.staff);
        u(&mut h, &f.desired_employment);
        i(&mut h, &f.expected_demand);
        i(&mut h, &f.capacity);
        i(&mut h, &f.inventory);
        i(&mut h, &f.sales_since_plan);
        b(&mut h, &f.currency);
        b(&mut h, &f.deposits);
        i(&mut h, &self.uu.plan_output);
        b(&mut h, &self.uu.owes);
        self.uu.input_stock.iter().for_each(|s| i(&mut h, s));
        b(&mut h, &self.uu.lent);
        bytes(&mut h, &mut self.uu.plan_days.iter().copied());
        bytes(&mut h, &mut self.uu.plan_done.iter().copied());
        self.uu
            .inputs_served_ppm
            .iter()
            .for_each(|&x| h.write_word(u64::from(x)));
        self.uu
            .hire_carry
            .iter()
            .flatten()
            .for_each(|&x| h.write_word(u64::from(x)));
        let ind = &self.industries;
        ind.price_ppm
            .iter()
            .for_each(|&x| h.write_word(u64::from(x)));
        self.ii
            .wage_cost_ppm
            .iter()
            .for_each(|&x| h.write_word(u64::from(x)));
        h.write_word(
            u64::from(self.mac.expected_inflation_ppm.unsigned_abs()) << 1
                | u64::from(self.mac.expected_inflation_ppm < 0),
        );
        self.mac
            .cpi
            .iter()
            .for_each(|&x| h.write_word(u64::from(x)));
        #[allow(clippy::cast_sign_loss)]
        self.mac
            .reference_wage
            .iter()
            .flatten()
            .for_each(|x| h.write_word(x.get() as u64));
        // The taxes' and transfers' own state, under a law that has them (M2's
        // hash is the same as before they existed).
        if self.law.income.is_some() || self.law.transfers.is_some() {
            self.fs.hash(&mut |w| h.write_word(w));
        }
        #[cfg(feature = "fiscal")]
        self.treasury_hash(&mut |w| h.write_word(w));
        #[cfg(feature = "banking")]
        self.bank_hash(&mut |w| h.write_word(w));
        #[cfg(feature = "trade")]
        self.trade_hash(&mut |w| h.write_word(w));
        #[cfg(feature = "monetary")]
        self.cb_hash(&mut |w| h.write_word(w));
        #[cfg(feature = "investment")]
        self.invest_hash(&mut |w| h.write_word(w));
        #[cfg(feature = "revenue")]
        self.rev_hash(&mut |w| h.write_word(w));
        let mut ledger = StateHasher::default();
        self.ledger.hash_into(&mut |bytes| ledger.write(bytes));
        h.write_word(ledger.finish());
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifths_are_by_weight() {
        // Five households of equal weight: one in each fifth.
        let (f, b) = fifths(&[50, 10, 40, 20, 30], &[1; 5]);
        assert_eq!(f, vec![4, 0, 3, 1, 2]);
        assert_eq!(b, [20, 30, 40, 50]);
        assert_eq!(fifth_of(5, &b), 0);
        assert_eq!(fifth_of(35, &b), 2);
        assert_eq!(fifth_of(99, &b), 4);
    }

    #[test]
    fn tightness_is_vacancies_over_qualified_seekers() {
        assert_eq!(tightness_ppm(50, &[0, 1, 0], &[10, 100, 400]), 500_000);
        assert_eq!(tightness_ppm(0, &[1, 1, 1], &[1, 1, 1]), 0);
        assert_eq!(tightness_ppm(10, &[0, 0, 0], &[1, 1, 1]), 0);
    }
}
