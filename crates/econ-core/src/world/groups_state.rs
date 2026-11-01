//! Group queries on the `World` (feature `groups`; spec `game/group-queries`):
//! a filter over residents' attributes, its summary at the latest close, its
//! breakdowns, and watched groups recorded at every close. Read-only:
//! watching records outputs only.

use econ_popgen::{Activity, EduLevel, Enrolled, NO_EMPLOYER, NOT_EMPLOYED, Sex};
use econ_types::Bani;

use super::World;
use crate::groups::{Attribute, Condition, Filter, GroupSummary, MIN_RECORDS};

/// The groups' state: each household's locality type, and the watched
/// groups with their summaries.
#[derive(Debug, Clone, Default)]
pub(crate) struct GroupState {
    urban: Vec<bool>,
    county: Vec<u8>,
    n_regions: usize,
    n_counties: usize,
    watched: Vec<(String, Filter, Vec<GroupSummary>)>,
}

/// Age bands of the age breakdown, `[from, to)` in years, the last open.
const AGE_BANDS: [(u16, u16); 7] = [
    (0, 15),
    (15, 25),
    (25, 35),
    (35, 45),
    (45, 55),
    (55, 65),
    (65, u16::MAX),
];

const ACTIVITIES: [Activity; 7] = [
    Activity::Child,
    Activity::Pupil,
    Activity::Student,
    Activity::Employed,
    Activity::Unemployed,
    Activity::Retired,
    Activity::InactiveOther,
];

/// A person's attributes, read once.
struct Person {
    h: usize,
    county: u8,
    region: u8,
    urban: bool,
    age_years: u16,
    sex: Sex,
    edu: EduLevel,
    activity: Activity,
    occupation: u8,
    industry: Option<u16>,
    fifth: u8,
    class: Option<u8>,
    enrolled: Enrolled,
}

fn meets(c: &Condition, p: &Person) -> bool {
    match c {
        Condition::County(v) => v.contains(&p.county),
        Condition::Region(v) => v.contains(&p.region),
        Condition::Urban(u) => p.urban == *u,
        Condition::Age { from, to } => p.age_years >= *from && p.age_years < *to,
        Condition::Sex(v) => v.contains(&p.sex),
        Condition::Edu(v) => v.contains(&p.edu),
        Condition::Activity(v) => v.contains(&p.activity),
        Condition::Occupation(v) => v.contains(&p.occupation),
        Condition::Industry(v) => v.contains(&p.industry),
        Condition::Fifth(v) => v.contains(&p.fifth),
        Condition::Class(v) => p.class.is_some_and(|k| v.contains(&k)),
        Condition::Enrolled(v) => v.contains(&p.enrolled),
    }
}

/// `n / d` rounded half away from zero (`d` > 0).
fn rdiv(n: i128, d: i128) -> i128 {
    if n >= 0 {
        (2 * n + d) / (2 * d)
    } else {
        -((-2 * n + d) / (2 * d))
    }
}

impl World {
    /// Load: each household's locality type and county.
    pub(super) fn start_groups(&mut self, s: &super::StartState) {
        let n = s.hh_weight.len();
        self.grp = GroupState {
            urban: if s.hh_urban.len() == n {
                s.hh_urban.clone()
            } else {
                vec![true; n]
            },
            county: s.hh_county.clone(),
            n_regions: s
                .hh_region
                .iter()
                .copied()
                .max()
                .map_or(0, |r| usize::from(r) + 1),
            n_counties: s
                .hh_county
                .iter()
                .copied()
                .max()
                .map_or(0, |c| usize::from(c) + 1),
            watched: Vec::new(),
        };
    }

    /// A new household record like `like` (the life cycle, arrivals).
    #[cfg(feature = "lifecycle")]
    pub(super) fn grp_push_household(&mut self, like: usize) {
        let u = self.grp.urban.get(like).copied().unwrap_or(true);
        let c = self.grp.county.get(like).copied().unwrap_or(0);
        self.grp.urban.push(u);
        self.grp.county.push(c);
    }

    /// The yearly compaction of the People phase.
    #[cfg(feature = "demography")]
    pub(super) fn grp_compact(&mut self, keep_h: &[bool]) {
        super::money::retain_by(&mut self.grp.urban, keep_h);
        super::money::retain_by(&mut self.grp.county, keep_h);
    }

    fn grp_county(&self, h: usize) -> u8 {
        #[cfg(feature = "demography")]
        if let Some(&c) = self.dem.hh_life.hh_county.get(h) {
            return c;
        }
        self.grp.county.get(h).copied().unwrap_or(0)
    }

    /// The residents' attributes.
    fn grp_persons(&self) -> Vec<(usize, Person)> {
        let act = self.activity();
        #[cfg(feature = "poverty")]
        let anchor_classes = self.pov_classes();
        let mut out = Vec::new();
        for p in 0..self.pp.age.len() {
            let h = self.persons.household_id[p] as usize;
            if !self.person_resident(p) || self.households.hh_weight[h] == 0 {
                continue;
            }
            let u = self.persons.employer[p];
            #[cfg(feature = "poverty")]
            let class = anchor_classes.get(h).copied().flatten();
            #[cfg(not(feature = "poverty"))]
            let class = None;
            out.push((
                p,
                Person {
                    h,
                    county: self.grp_county(h),
                    region: self.pp.region[p],
                    urban: self.grp.urban.get(h).copied().unwrap_or(true),
                    age_years: self.pp.age[p] / 12,
                    sex: self.people.sex.get(p).copied().unwrap_or(Sex::F),
                    edu: self.persons.edu_level[p],
                    activity: act.get(p).copied().unwrap_or(Activity::Child),
                    occupation: self.persons.occupation[p],
                    industry: (u != NO_EMPLOYER)
                        .then(|| u16::from(self.units.industry[usize::from(u)])),
                    fifth: self.households.hh_fifth[h],
                    class,
                    enrolled: self
                        .people
                        .enrolled
                        .get(p)
                        .copied()
                        .unwrap_or(Enrolled::None),
                },
            ));
        }
        out
    }

    /// The summary of the persons chosen (indices into `persons`).
    #[allow(clippy::too_many_lines)]
    fn grp_summary(&self, persons: &[(usize, Person)], chosen: &[usize]) -> GroupSummary {
        let n_hh = self.households.hh_weight.len();
        // Each household's resident members and its first one.
        let mut members = vec![0i64; n_hh];
        let mut first = vec![usize::MAX; n_hh];
        for (p, x) in persons {
            members[x.h] += 1;
            if first[x.h] == usize::MAX {
                first[x.h] = *p;
            }
        }
        let share = |total: i64, h: usize, p: usize| -> i64 {
            let n = members[h].max(1);
            let each = total.div_euclid(n);
            if p == first[h] {
                total - each * (n - 1)
            } else {
                each
            }
        };
        let s = &self.slips;
        let get = |v: &[Bani], i: usize| v.get(i).copied().unwrap_or(Bani::ZERO).get();
        #[cfg(feature = "banking")]
        let bank = self.bank();
        let mut g = GroupSummary::default();
        let mut seen = vec![false; n_hh];
        let (mut wg, mut wn, mut pe, mut ub, mut tr, mut tx, mut mo, mut de) =
            (0i128, 0i128, 0i128, 0i128, 0i128, 0i128, 0i128, 0i128);
        #[cfg(feature = "poverty")]
        let (mut inc_num, mut inc_den, mut sol_num, mut sol_den) = (0i128, 0i128, 0i128, 0i128);
        #[cfg(feature = "approval")]
        let mut adults: Vec<(u64, [i32; 5])> = Vec::new();
        for &i in chosen {
            let (p, x) = &persons[i];
            let (p, h) = (*p, x.h);
            let w = i128::from(self.households.hh_weight[h]);
            g.persons += u64::try_from(w).expect("a weight");
            g.person_records += 1;
            if !seen[h] {
                seen[h] = true;
                g.households += u64::try_from(w).expect("a weight");
                g.household_records += 1;
            }
            match x.activity {
                Activity::Employed => g.employed += u64::try_from(w).expect("a weight"),
                Activity::Unemployed => g.unemployed += u64::try_from(w).expect("a weight"),
                _ => {}
            }
            wg += w * i128::from(get(&s.gross_wage, p));
            wn += w * i128::from(get(&s.net_wage, p));
            pe += w * i128::from(get(&s.pension, p));
            ub += w * i128::from(get(&s.unemployment_benefit, p));
            let transfers = get(&s.child_allowance, h) + get(&s.minimum_income, h);
            tr += w * i128::from(share(transfers, h, p));
            let own_tax = get(&s.income_tax, p)
                + get(&s.pension_tax, p)
                + get(&s.cas_public, p)
                + get(&s.cas_pillar2, p)
                + get(&s.cass, p);
            tx += w * i128::from(own_tax + share(get(&s.self_employment_tax, h), h, p));
            let money = self.households.hh_cash[h].get() + self.households.hh_deposits[h].get();
            mo += w * i128::from(share(money, h, p));
            #[cfg(feature = "banking")]
            {
                let d = get(&bank.hh_mortgage, h) + get(&bank.hh_consumer_debt, h);
                de += w * i128::from(share(d, h, p));
            }
            #[cfg(feature = "poverty")]
            if self.pov_counted(h) {
                let e = i128::from(self.hh.eq_adults_milli[h].max(1));
                inc_num += w
                    * i128::from(self.hh_real_income_recent().get(h).map_or(0, |b| b.get()))
                    * 1_000
                    / e;
                inc_den += w;
                sol_num += w * i128::from(self.hh_sol_milli().get(h).copied().unwrap_or(0));
                sol_den += w;
            }
            #[cfg(feature = "approval")]
            if let Some((_, parts)) = self.approval_of(u32::try_from(p).expect("a person")) {
                adults.push((u64::try_from(w).expect("a weight"), parts));
            }
        }
        let bani = |x: i128| Bani(i64::try_from(x).expect("an amount"));
        g.wages_gross = bani(wg);
        g.wages_net = bani(wn);
        g.pensions = bani(pe);
        g.unemployment_benefits = bani(ub);
        g.household_transfers = bani(tr);
        g.taxes_paid = bani(tx);
        g.money = bani(mo);
        g.debt = bani(de);
        let lf = i128::from(g.employed + g.unemployed);
        g.unemployment_rate_ppm = if lf == 0 {
            0
        } else {
            u32::try_from(rdiv(i128::from(g.unemployed) * 1_000_000, lf)).expect("a rate")
        };
        g.small_sample = g.person_records < MIN_RECORDS;
        #[cfg(feature = "poverty")]
        {
            g.income_per_adult = (inc_den > 0).then(|| bani(rdiv(inc_num, inc_den)));
            g.sol_milli =
                (sol_den > 0).then(|| u32::try_from(rdiv(sol_num, sol_den)).expect("points"));
        }
        #[cfg(feature = "approval")]
        {
            g.approval_milli = super::approval_state::group_mean(&adults).map(|m| m.mean_milli);
        }
        g
    }

    /// The summary of a filter at the latest close.
    pub(crate) fn grp_group(&self, f: &Filter) -> GroupSummary {
        let persons = self.grp_persons();
        let chosen: Vec<usize> = (0..persons.len())
            .filter(|&i| f.conditions.iter().all(|c| meets(c, &persons[i].1)))
            .collect();
        self.grp_summary(&persons, &chosen)
    }

    /// One summary per value of `by`, in the attribute's order.
    pub(crate) fn grp_breakdown(&self, f: &Filter, by: Attribute) -> Vec<(u32, GroupSummary)> {
        let persons = self.grp_persons();
        let chosen: Vec<usize> = (0..persons.len())
            .filter(|&i| f.conditions.iter().all(|c| meets(c, &persons[i].1)))
            .collect();
        let n_i = self.industries.id.len();
        let n = match by {
            Attribute::County => self.grp.n_counties,
            Attribute::Region => self.grp.n_regions,
            Attribute::Urban | Attribute::Sex => 2,
            Attribute::AgeBand | Attribute::Activity => 7,
            Attribute::Edu | Attribute::Fifth => 5,
            Attribute::Occupation => 11,
            Attribute::Industry => n_i + 1,
            Attribute::Class => 4,
            Attribute::Enrolled => 8,
        };
        let value = |x: &Person| -> usize {
            match by {
                Attribute::County => usize::from(x.county),
                Attribute::Region => usize::from(x.region),
                Attribute::Urban => usize::from(!x.urban),
                Attribute::AgeBand => AGE_BANDS
                    .iter()
                    .position(|&(a, b)| x.age_years >= a && x.age_years < b)
                    .unwrap_or(6),
                Attribute::Sex => usize::from(x.sex != Sex::F),
                Attribute::Edu => x.edu as usize,
                Attribute::Activity => ACTIVITIES
                    .iter()
                    .position(|&a| a == x.activity)
                    .unwrap_or(6),
                Attribute::Occupation => {
                    if x.occupation == NOT_EMPLOYED {
                        10
                    } else {
                        usize::from(x.occupation.min(9))
                    }
                }
                Attribute::Industry => x.industry.map_or(n_i, usize::from),
                Attribute::Fifth => usize::from(x.fifth),
                Attribute::Class => usize::from(x.class.unwrap_or(0)),
                Attribute::Enrolled => x.enrolled as usize,
            }
        };
        let mut parts: Vec<Vec<usize>> = vec![Vec::new(); n];
        for &i in &chosen {
            let v = value(&persons[i].1);
            if v < n {
                parts[v].push(i);
            }
        }
        parts
            .iter()
            .enumerate()
            .map(|(v, ch)| {
                (
                    u32::try_from(v).expect("a value"),
                    self.grp_summary(&persons, ch),
                )
            })
            .collect()
    }

    pub(crate) fn grp_watch(&mut self, name: &str, f: Filter) {
        if let Some(w) = self.grp.watched.iter_mut().find(|(n, _, _)| n == name) {
            w.1 = f;
            w.2.clear();
        } else {
            self.grp.watched.push((name.to_string(), f, Vec::new()));
        }
    }

    pub(crate) fn grp_watched(&self, name: &str) -> Option<&[GroupSummary]> {
        self.grp
            .watched
            .iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, _, v)| v.as_slice())
    }

    pub(crate) fn grp_watched_names(&self) -> Vec<&str> {
        self.grp
            .watched
            .iter()
            .map(|(n, _, _)| n.as_str())
            .collect()
    }

    /// A close's day: each watched group's summary.
    pub(super) fn grp_record(&mut self) {
        if self.grp.watched.is_empty() {
            return;
        }
        let persons = self.grp_persons();
        let mut out = Vec::with_capacity(self.grp.watched.len());
        for (_, f, _) in &self.grp.watched {
            let chosen: Vec<usize> = (0..persons.len())
                .filter(|&i| f.conditions.iter().all(|c| meets(c, &persons[i].1)))
                .collect();
            out.push(self.grp_summary(&persons, &chosen));
        }
        for (w, s) in self.grp.watched.iter_mut().zip(out) {
            w.2.push(s);
        }
    }
}
