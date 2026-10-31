//! Migration, the diaspora and remittances on the `World` (spec
//! `economy/migration-remittances`, feature `migration`): the month's flows
//! and the diaspora, read-only, for the `[sim]` criteria AC-MIG-05 to 17.
//!
//! The API is the test writer's (ADR-0014), from the spec's API sketch.
//! At load the scenario's linked emigrants become person rows of their
//! households with `abroad` set: inert (no job, no search, no pension, no
//! income), skipped by every resident loop, ageing and dying with the People
//! phase; when their household ends they move to the stock.

use econ_popgen::EduLevel;
use econ_types::{Bani, Month};

use super::World;

/// The age bands of the month's emigrants, `[from, to)` in years, the last
/// open: those of the research note's table (Eurostat `migr_emi2`).
pub const MIGRATION_AGE_BANDS: [(u16, u16); 7] = [
    (0, 15),
    (15, 25),
    (25, 35),
    (35, 45),
    (45, 55),
    (55, 65),
    (65, u16::MAX),
];

/// A month of migration, written at the monthly close. Persons are weighted
/// (real persons); money is national (weighted) bani.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationMonth {
    /// The month.
    pub month: Month,
    /// Emigrants: persons who left Romania at this close, alone (linked) or
    /// with their whole household.
    pub emigrants: u64,
    /// The same by [`MIGRATION_AGE_BANDS`].
    pub emigrants_by_age: [u64; 7],
    /// Returns of Romanian citizens: linked emigrants who rejoined their
    /// household and persons back from the diaspora stock.
    pub returns: u64,
    /// The same by the development region they return to, in the
    /// scenario's order (AC-MIG-20).
    pub returns_by_region: Vec<u64>,
    /// Foreign immigrants.
    pub immigrants: u64,
    /// Births of the month (the demographics core's), for the balance of
    /// persons (AC-MIG-08).
    pub births: u64,
    /// Deaths of resident persons in the month (the demographics core's).
    pub deaths_resident: u64,
    /// Deaths abroad: linked emigrants and the diaspora stock.
    pub deaths_abroad: u64,
    /// Persons who moved into each county from another county, by county
    /// index (the scenario's order).
    pub moves_in: Vec<u64>,
    /// Persons who moved out of each county to another county.
    pub moves_out: Vec<u64>,
    /// Remittances paid to linked households this month, lei.
    pub remittances: Bani,
    /// The same in euro cents.
    pub remittances_eur_cents: i64,
    /// Each household record that emigrated whole at this close: its index,
    /// its weight before it left, and the net deposits per record it took to
    /// the rest of the world under `transfer.migrant_capital` (after its cash
    /// was deposited and its loans repaid or written off).
    pub emigrated_households: Vec<(u32, u32, Bani)>,
}

/// One cell of the diaspora stock: unlinked emigrants as counts of real
/// persons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StockCell {
    /// Origin development region.
    pub region: u8,
    /// Sex (`econ_popgen::Sex` as its index).
    pub sex: u8,
    /// Year of birth.
    pub birth_year: u16,
    /// Skill tier: low 0, mid 1, high 2.
    pub tier: u8,
    /// Real persons.
    pub persons: u64,
}

/// The diaspora now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiasporaView {
    /// Linked emigrants, weighted (persons with `abroad` in records of
    /// positive weight).
    pub linked: u64,
    /// The stock, by cell.
    pub stock: Vec<StockCell>,
}

/// Migration's state: per person (aligned with the persons table) and the
/// stock.
#[derive(Debug, Clone, Default)]
pub(crate) struct MigState {
    /// A linked emigrant (live, abroad).
    pub(crate) abroad: Vec<bool>,
    /// `months_abroad` of a linked emigrant.
    pub(crate) months_abroad: Vec<u16>,
    /// `diaspora_stock[k]`, in the scenario's order; new cells appended.
    pub(crate) stock: Vec<StockCell>,
    /// The scripted unemployment shocks by region (AC-MIG-15), millionths.
    pub(crate) shocks: Vec<(u8, u32)>,
    /// `emigration_base[a]` for bands 15–24 to 55–64, millionths a month.
    pub(crate) p0_ppm: [u32; 5],
    /// `migration_carry[cell]` of emigration, real persons.
    pub(crate) carry: Vec<i64>,
    /// Each tier's mean net wage in euro at the first close (×1e12).
    pub(crate) base_wage_eur: Option<[i128; 3]>,
    /// Each region's unemployment rate at the first close, millionths.
    pub(crate) base_unemp: Vec<u32>,
    /// Deaths of linked emigrants this month, weighted.
    pub(crate) deaths_abroad: u64,
    /// `world.wages_abroad` at the latest close, basis points.
    pub(crate) omega_bp: u32,
    /// The all-tier mean net wage in euro at the first close (×1e12).
    pub(crate) base_wage_all: Option<i128>,
    /// The all-tier home wage index in euro now, millionths.
    pub(crate) h_all_ppm: u32,
    /// The pull by tier now, millionths.
    pub(crate) pull: [u32; 3],
    /// `return_base[g]` for ages under 15, 15–64 and 65 and over,
    /// millionths a month.
    pub(crate) q0_ppm: [u32; 3],
    /// The carries of returns of links (group × tier), of the stock (region
    /// × group × tier) and of foreign immigration, real persons.
    pub(crate) link_carry: Vec<i64>,
    pub(crate) stock_carry: Vec<i64>,
    pub(crate) imm_carry: i64,
    /// The household records before the day's People phase: those formed
    /// since (splits, the life cycle) do not leave whole the same day.
    pub(crate) hh_before: usize,
    /// Moves between counties: `m0`, parts per billion a month; each
    /// county's appeal α; its region; travel times between county seats,
    /// hours (row-major); the carries by county, households.
    pub(crate) m0_ppb: u32,
    pub(crate) alpha: Vec<f64>,
    pub(crate) county_region: Vec<u8>,
    pub(crate) travel_hours: Vec<f64>,
    pub(crate) move_carry: Vec<i64>,
    /// The destination carries by (origin, destination), households ×1e6.
    pub(crate) dest_carry: Vec<i64>,
    /// Step 5b: each region's expected home quality, its wellbeing index at
    /// the first close, and the unemployment step 2 measured this month.
    pub(crate) home_quality: Vec<u32>,
    #[cfg_attr(not(feature = "home_quality"), allow(dead_code))]
    pub(crate) wellbeing0: Vec<u32>,
    pub(crate) unemp_now: Vec<u32>,
    /// Returns of the month by the region returned to.
    pub(crate) returns_by_region: Vec<u64>,
}

/// Migration's parameters on the `World`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationWorldParams {
    /// The rules' tuning parameters.
    pub rules: econ_mech_migration::MigrationParams,
    /// 2021's emigrants aged 15–24, 25–34, 35–44, 45–54 and 55–64 (Eurostat
    /// `migr_emi2`, the research note), the targets `p0` is fitted to.
    pub emigrants_2021: [u64; 5],
    /// 2021's returns of Romanian citizens under 15, 15–64 and 65 and over
    /// (Eurostat `migr_imm1ctz`).
    pub returns_2021: [u64; 3],
    /// 2021's foreign immigrants, EU and non-EU.
    pub immigrants_2021: u64,
    /// ε_i, `immigration_elasticity`, millionths.
    pub immigration_elasticity_ppm: u32,
    /// `return_savings`: what a returning adult brings, euro cents.
    pub return_savings_eur_cents: i64,
    /// The share of men among foreign immigrants, millionths (INS 2021).
    pub immigrant_men_ppm: u32,
    /// β^move_u, `move_unemployment`: the origin region's unemployment push
    /// on moves between counties, millionths.
    pub move_unemployment_ppm: u32,
    /// β_w, β_u and β_t of the destination choice (wages, unemployment,
    /// travel time per hour), millionths.
    pub move_wage_ppm: u32,
    pub move_unemployment_dest_ppm: u32,
    pub move_time_ppm: u32,
    /// Travel time between county seats until the transport network (M11):
    /// straight-line distance times `road_factor`, at `road_speed`.
    pub road_factor_ppm: u32,
    pub road_speed_kmh: u32,
    /// ε_q, `quality_return_elasticity`, millionths (step 5b). Test writer's
    /// addition (ADR-0014).
    pub quality_return_elasticity_ppm: u32,
    /// ε_q,e, `quality_emigration_elasticity`, millionths (step 5b).
    pub quality_emigration_elasticity_ppm: u32,
    /// ε_q,i, `quality_immigration_elasticity`, millionths (step 5b).
    pub quality_immigration_elasticity_ppm: u32,
    /// θ_u, `quality_jobs_weight`, millionths (step 5b).
    pub quality_jobs_weight_ppm: u32,
    /// m, `quality_memory`, months (step 5b).
    pub quality_memory_months: u16,
}

impl MigrationWorldParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> MigrationWorldParams {
        MigrationWorldParams {
            rules: econ_mech_migration::MigrationParams::spec_defaults(),
            emigrants_2021: [39_259, 62_148, 45_638, 31_050, 13_111],
            returns_2021: [18_473, 125_609, 5_431],
            immigrants_2021: 45_043,
            immigration_elasticity_ppm: 1_000_000,
            return_savings_eur_cents: 300_000,
            immigrant_men_ppm: 547_000,
            move_unemployment_ppm: 5_000_000,
            move_wage_ppm: 1_000_000,
            move_unemployment_dest_ppm: 5_000_000,
            move_time_ppm: 100_000,
            road_factor_ppm: 1_300_000,
            road_speed_kmh: 70,
            quality_return_elasticity_ppm: 1_000_000,
            quality_emigration_elasticity_ppm: 500_000,
            quality_immigration_elasticity_ppm: 250_000,
            quality_jobs_weight_ppm: 1_000_000,
            quality_memory_months: 36,
        }
    }
}

/// The skill tier of an education level: low 0, mid 1, high 2.
fn tier_of(e: EduLevel) -> u8 {
    match e {
        EduLevel::Isced0To2 => 0,
        EduLevel::Isced3 | EduLevel::Isced4 => 1,
        EduLevel::Isced5To6 | EduLevel::Isced7To8 => 2,
    }
}

impl World {
    /// Whether person `p` is a linked emigrant.
    pub(crate) fn mig_abroad(&self, p: usize) -> bool {
        self.mig.abroad.get(p).copied().unwrap_or(false)
    }

    /// Load (after the People phase's start): the stock, then each linked
    /// emigrant as a person row of its household, inert.
    pub(super) fn start_migration(&mut self) {
        let n = self.pp.age.len();
        self.mig.abroad = vec![false; n];
        self.mig.months_abroad = vec![0; n];
        self.mig.stock = self
            .people
            .diaspora_stock
            .iter()
            .map(|c| StockCell {
                region: c.region,
                sex: c.sex as u8,
                birth_year: c.birth_year,
                // Stage L's tiers are high 0, mid 1, low 2.
                tier: 2 - c.tier.min(2),
                persons: c.persons,
            })
            .collect();
        let le = std::mem::take(&mut self.people.linked_emigrants);
        for i in 0..le.household_id.len() {
            let h = le.household_id[i];
            let like = (0..self.persons.household_id.len())
                .find(|&q| self.persons.household_id[q] == h)
                .unwrap_or(0);
            let p = self.push_person_keyed(h, le.sex[i], like, le.person_key[i]);
            self.pp.age[p] = le.age[i];
            self.people.role[p] = le.role[i];
            self.persons.edu_level[p] = le.edu_level[i];
            self.dem.life.activity[p] = econ_popgen::Activity::InactiveOther;
            self.set_base_activity(p, econ_popgen::Activity::InactiveOther);
            self.mig.abroad[p] = true;
            self.mig.months_abroad[p] = le.months_abroad[i];
        }
        self.people.next_person_key = self
            .people
            .next_person_key
            .max(le.person_key.iter().map(|&k| k + 1).max().unwrap_or(0));
        self.mig.omega_bp = self.mig_omega_bp();
        self.mig.hh_before = self.households.hh_weight.len();
        let n_r = usize::from(self.pp.region.iter().copied().max().unwrap_or(0)) + 1;
        self.mig.home_quality = vec![1_000_000; n_r];
        self.mig.returns_by_region = vec![0; n_r];
        self.mig_fit();
        self.mig_fit_returns();
        self.mig_fit_moves();
    }

    /// Add persons to the stock's cell.
    pub(super) fn mig_stock_add(&mut self, cell: StockCell) {
        match self.mig.stock.iter_mut().find(|c| {
            (c.region, c.sex, c.birth_year, c.tier)
                == (cell.region, cell.sex, cell.birth_year, cell.tier)
        }) {
            Some(c) => c.persons += cell.persons,
            None => self.mig.stock.push(cell),
        }
    }

    /// Append migration's columns for a new person row: a copy of person
    /// `from`'s, or a resident's.
    pub(super) fn mig_push_person(&mut self, from: Option<usize>) {
        let n = self.pp.age.len();
        let m = &mut self.mig;
        let (a, k) = from.map_or((false, 0), |p| {
            (
                m.abroad.get(p).copied().unwrap_or(false),
                m.months_abroad.get(p).copied().unwrap_or(0),
            )
        });
        m.abroad.resize(n - 1, false);
        m.months_abroad.resize(n - 1, 0);
        m.abroad.push(a);
        m.months_abroad.push(k);
    }

    /// The yearly compaction of the People phase.
    pub(super) fn mig_compact(&mut self, keep_p: &[bool]) {
        super::money::retain_by(&mut self.mig.abroad, keep_p);
        super::money::retain_by(&mut self.mig.months_abroad, keep_p);
    }

    /// A linked emigrant died (the People phase): no longer abroad.
    pub(super) fn mig_died(&mut self, p: usize) {
        if self.mig_abroad(p) {
            self.mig.abroad[p] = false;
            self.mig.deaths_abroad +=
                u64::from(self.households.hh_weight[self.persons.household_id[p] as usize]);
        }
    }

    /// Household `h` ends: its linked emigrants join the stock with their
    /// counts (the record's weight) and their rows end.
    pub(super) fn mig_household_ends(&mut self, h: usize) {
        let w = u64::from(self.households.hh_weight[h]);
        let year = self.calendar.date(self.today).year;
        for p in 0..self.pp.age.len() {
            if self.persons.household_id[p] as usize != h
                || !self.mig_abroad(p)
                || !self.dem.life.alive[p]
            {
                continue;
            }
            let birth_year = u16::try_from(year - i32::from(self.pp.age[p] / 12)).unwrap_or(0);
            let cell = StockCell {
                region: self.pp.region[p],
                sex: self.people.sex[p] as u8,
                birth_year,
                tier: tier_of(self.persons.edu_level[p]),
                persons: w,
            };
            self.mig_stock_add(cell);
            self.mig.abroad[p] = false;
            self.dem.life.alive[p] = false;
        }
    }

    /// The monthly close: a month more abroad, the month's emigration, and
    /// the month's report (with the People phase's births and deaths).
    pub(super) fn migration_close(&mut self) {
        for p in 0..self.mig.abroad.len() {
            if self.mig.abroad[p] {
                self.mig.months_abroad[p] = self.mig.months_abroad[p].saturating_add(1);
            }
        }
        let d = self.mig_emigration();
        let returns = self.mig_link_returns() + self.mig_stock_returns();
        let immigrants = self.mig_immigration();
        let n_c = self.people.county_moves.as_ref().map_or_else(
            || {
                usize::from(
                    self.dem
                        .hh_life
                        .hh_county
                        .iter()
                        .copied()
                        .max()
                        .unwrap_or(0),
                ) + 1
            },
            |c| c.net_2021.len(),
        );
        let moves = self.mig_moves(n_c);
        // Linked emigrants' deaths (in the People phase's report), then the
        // stock's of the year, each January.
        let link_deaths = std::mem::take(&mut self.mig.deaths_abroad);
        let stock_deaths = if self.calendar.date(self.today).month == 1 {
            self.mig_stock_deaths()
        } else {
            0
        };
        self.mig.omega_bp = self.mig_omega_bp();
        let t = u32::try_from(self.months.len() - 1).expect("a month");
        let births: u64 = self
            .dem
            .report
            .births
            .iter()
            .map(|b| u64::from(b.weight))
            .sum();
        let deaths: u64 = self
            .dem
            .report
            .deaths
            .iter()
            .map(|x| u64::from(x.weight))
            .sum();
        let months = &mut self.rem.migration_months;
        if months.last().is_none_or(|m| m.month.0 != t) {
            months.push(MigrationMonth {
                month: Month(t),
                ..MigrationMonth::default()
            });
        }
        let m = months.last_mut().expect("the month");
        m.emigrants = d.emigrants;
        m.returns = returns;
        m.returns_by_region = std::mem::replace(
            &mut self.mig.returns_by_region,
            vec![0; self.mig.home_quality.len()],
        );
        m.immigrants = immigrants;
        m.emigrants_by_age = d.by_age;
        m.emigrated_households = d.households;
        m.births = births;
        m.deaths_resident = deaths - link_deaths;
        m.deaths_abroad = link_deaths + stock_deaths;
        m.moves_in = moves.moves_in;
        m.moves_out = moves.moves_out;
    }

    /// Every closed month's migration, in order (one per monthly close).
    #[must_use]
    pub fn migration_months(&self) -> &[MigrationMonth] {
        // Remittances only, so far (spec step 3); the other flows come with
        // emigration, returns, immigration and moves.
        &self.rem.migration_months
    }

    /// The diaspora now: linked emigrants and the stock.
    #[must_use]
    pub fn diaspora(&self) -> DiasporaView {
        let linked = (0..self.mig.abroad.len())
            .filter(|&p| self.mig.abroad[p])
            .map(|p| u64::from(self.households.hh_weight[self.persons.household_id[p] as usize]))
            .sum();
        DiasporaView {
            linked,
            stock: self.mig.stock.clone(),
        }
    }

    /// Each person's `abroad` flag (a linked emigrant), aligned with
    /// [`super::PersonColumns`].
    #[must_use]
    pub fn abroad(&self) -> &[bool] {
        &self.mig.abroad
    }

    /// `home_quality[r]`: each development region's expected home quality
    /// (step 5b) after the latest close, millionths (1 at the start), in the
    /// scenario's order of regions.
    #[must_use]
    pub fn home_quality_ppm(&self) -> &[u32] {
        &self.mig.home_quality
    }

    /// A return to region `r` (the month's report).
    pub(super) fn mig_count_return(&mut self, r: usize, persons: u64) {
        if self.mig.returns_by_region.len() <= r {
            self.mig.returns_by_region.resize(r + 1, 0);
        }
        self.mig.returns_by_region[r] += persons;
    }

    /// The residents' mean expected home quality, millionths.
    #[cfg(feature = "home_quality")]
    pub(super) fn mig_mean_quality(&self) -> u32 {
        let n_r = self.mig.home_quality.len();
        let mut res = vec![0u128; n_r];
        for p in 0..self.pp.age.len() {
            let r = usize::from(self.pp.region[p]);
            if r < n_r && self.person_resident(p) {
                res[r] +=
                    u128::from(self.households.hh_weight[self.persons.household_id[p] as usize]);
            }
        }
        let all: u128 = res.iter().sum();
        if all == 0 {
            return 1_000_000;
        }
        let q: u128 = (0..n_r)
            .map(|r| res[r] * u128::from(self.mig.home_quality[r]))
            .sum();
        u32::try_from((q + all / 2) / all).expect("a quality")
    }

    /// Step 5b at the end of the close, after the wellbeing index: each
    /// region's instant quality and its expected quality's update, read by
    /// the next month's flows.
    #[cfg(feature = "home_quality")]
    pub(super) fn mig_home_quality_close(&mut self) {
        let Some(w) = self.wellbeing_stats().last().map(|s| {
            s.by_region
                .iter()
                .map(|x| x.index_ppm)
                .collect::<Vec<u32>>()
        }) else {
            return;
        };
        if self.mig.wellbeing0.is_empty() {
            self.mig.wellbeing0.clone_from(&w);
        }
        let q = self.params.migration;
        for r in 0..self.mig.home_quality.len() {
            let instant = econ_mech_migration::instant_quality_ppm(
                w.get(r).copied().unwrap_or(0),
                self.mig.wellbeing0.get(r).copied().unwrap_or(0),
                self.mig.unemp_now.get(r).copied().unwrap_or(0),
                self.mig.base_unemp.get(r).copied().unwrap_or(0),
                q.rules.unemployment_response_ppm,
                q.quality_jobs_weight_ppm,
            );
            self.mig.home_quality[r] = econ_mech_migration::home_quality_ppm(
                self.mig.home_quality[r],
                instant,
                q.quality_memory_months,
            );
        }
    }
}
