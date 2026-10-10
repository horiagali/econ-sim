//! Stage A — the rule-based seed (spec steps 1–6).
//!
//! Builds one county's synthetic households and persons from its margins.
//! Replaceable by a draw from an observed sample (IPUMS) without touching
//! the fit stage.

use econ_rng::{KeyedRng, Stream};

use crate::arith::{Int, apportion, count, int, rdiv, straddle};
use crate::{GenError, Role, Sex};

/// Years from which a person can head a household without the county running
/// out of adults, and the "adult in the household" age of the spec.
const ADULT_AGE: u16 = 20;
/// Youngest head when a county has too few adults.
const YOUNG_HEAD_AGE: u16 = 15;
const MONTHS: u16 = 12;

/// One synthetic person.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Member {
    pub sex: Sex,
    /// Age in months.
    pub age: u16,
    pub role: Role,
    /// Index of the person's (sex, age band) cell: `sex * n_bands + band`.
    pub cell: usize,
}

/// One county's seed.
#[derive(Debug)]
pub(crate) struct CountySeed {
    /// Size class each private household was created in.
    pub klass: Vec<usize>,
    /// Members of each private household, head first.
    pub members: Vec<Vec<Member>>,
    /// Per (sex, age band) cell: `(household, members in the cell)`, by household.
    pub by_cell: Vec<Vec<(usize, Int)>>,
    /// One-person records for people not in private households.
    pub collective: Vec<Member>,
    /// Household targets per size class, after classes without a synthetic
    /// household handed theirs to a neighbour.
    pub class_targets: Vec<Int>,
}

/// A person before assembly.
struct Candidate {
    sex: Sex,
    band: usize,
    cell: usize,
    priority: u64,
    age: u16,
}

/// The margins of one county, plus the shared dimensions.
pub(crate) struct CountyMargins<'a> {
    pub name: &'a str,
    /// Private households per size class.
    pub households: &'a [Int],
    /// Persons in private households per (sex, age band) cell.
    pub private: &'a [Int],
    /// Persons not in private households, same cells.
    pub collective: &'a [Int],
    /// Age bands `[from, to)` in years, the open band closed.
    pub bands: &'a [(u16, u16)],
    /// Size classes `[min, max]`, the open class capped.
    pub classes: &'a [(Int, Int)],
}

/// Build the seed of one county. `next_seq` is the running person sequence
/// number (creation order over the whole country), which keys the draws.
pub(crate) fn build(
    m: &CountyMargins<'_>,
    sample_scale: Int,
    rng: KeyedRng,
    next_seq: &mut u64,
) -> Result<CountySeed, GenError> {
    let n_bands = m.bands.len();
    let inconsistent = || GenError::InconsistentMargins {
        county: m.name.to_string(),
    };
    let p_total: Int = m.private.iter().sum();
    let h_total: Int = m.households.iter().sum();
    let q_total: Int = m.collective.iter().sum();
    let fewest: Int = m
        .households
        .iter()
        .zip(m.classes)
        .map(|(n, c)| n * c.0)
        .sum();
    let most: Int = m
        .households
        .iter()
        .zip(m.classes)
        .map(|(n, c)| n * c.1)
        .sum();
    if h_total == 0 || p_total < fewest || p_total > most {
        return Err(inconsistent());
    }

    // Steps 1-2: households and their sizes.
    let n_hh = count(rdiv(h_total, sample_scale).max(1));
    let per_class: Vec<usize> = apportion(int(n_hh), m.households)
        .into_iter()
        .map(count)
        .collect();
    let (class_targets, sizes) = household_sizes(m.classes, m.households, &per_class, p_total);
    let klass: Vec<usize> = per_class
        .iter()
        .enumerate()
        .flat_map(|(k, &n)| std::iter::repeat_n(k, n))
        .collect();

    // Step 3: who the persons are, in creation order (private, then collective).
    let mut draw_person = |cell: usize| {
        let band = cell % n_bands;
        let mut d = rng.draw(Stream::PopulationGen, 0, *next_seq);
        *next_seq += 1;
        let priority = d.u64();
        let (from, to) = m.bands[band];
        let months = d.below(u64::from((to - from) * MONTHS));
        Candidate {
            sex: if cell < n_bands { Sex::F } else { Sex::M },
            band,
            cell,
            priority,
            age: from * MONTHS + u16::try_from(months).expect("below a u16 bound"),
        }
    };
    let mut cells_of = |n: Int, table: &[Int]| -> Vec<Candidate> {
        let mut out = Vec::with_capacity(count(n));
        for (cell, &records) in apportion(n, table).iter().enumerate() {
            for _ in 0..records {
                out.push(draw_person(cell));
            }
        }
        out
    };
    let persons = cells_of(sizes.iter().sum(), m.private);
    let n_coll = rdiv(q_total, sample_scale).max(Int::from(q_total > 0));
    let collective = cells_of(n_coll, m.collective)
        .into_iter()
        .map(|c| Member {
            sex: c.sex,
            age: c.age,
            role: Role::Head,
            cell: c.cell,
        })
        .collect();

    // Step 4: assembly.
    let members = assemble(m.bands, &persons, &sizes);
    let mut by_cell: Vec<Vec<(usize, Int)>> = vec![Vec::new(); 2 * n_bands];
    for (h, household) in members.iter().enumerate() {
        for person in household {
            let cell = &mut by_cell[person.cell];
            match cell.last_mut() {
                Some(last) if last.0 == h => last.1 += 1,
                _ => cell.push((h, 1)),
            }
        }
    }
    Ok(CountySeed {
        klass,
        members,
        by_cell,
        collective,
        class_targets,
    })
}

/// Step 2: the household targets per class after hand-over, and the size of
/// every synthetic household (class by class).
fn household_sizes(
    classes: &[(Int, Int)],
    households: &[Int],
    per_class: &[usize],
    p_total: Int,
) -> (Vec<Int>, Vec<Int>) {
    let n_k = classes.len();
    // A size class with real households but no synthetic one hands its
    // target to the nearest class that has one (the smaller on a tie).
    let mut targets = households.to_vec();
    for k in 0..n_k {
        if targets[k] > 0 && per_class[k] == 0 {
            let nearest = (0..n_k)
                .filter(|&i| per_class[i] > 0)
                .min_by_key(|&i| (i.abs_diff(k), i))
                .expect("a county has at least one synthetic household");
            targets[nearest] += targets[k];
            targets[k] = 0;
        }
    }
    // Classes of one size have that size. The open classes must hold the
    // rest of the county's persons, so their sizes straddle the average
    // they need: then weights exist that give both the right number of
    // households and the right number of persons.
    let mut sizes_of: Vec<Vec<Int>> = (0..n_k).map(|k| vec![classes[k].0; per_class[k]]).collect();
    let is_open = |k: usize| classes[k].1 > classes[k].0 && per_class[k] > 0;
    let open: Vec<usize> = (0..n_k).filter(|&k| is_open(k)).collect();
    let mut need = p_total
        - (0..n_k)
            .filter(|&k| !is_open(k))
            .map(|k| classes[k].0 * targets[k])
            .sum::<Int>();
    for (i, &k) in open.iter().enumerate() {
        let (lo, hi) = classes[k];
        let later_min: Int = open[i + 1..]
            .iter()
            .map(|&j| classes[j].0 * targets[j])
            .sum();
        let num = need - later_min;
        if num <= hi * targets[k] || i + 1 == open.len() {
            sizes_of[k] = straddle(per_class[k], num, targets[k], lo, hi);
            break;
        }
        sizes_of[k] = vec![hi; per_class[k]];
        need -= hi * targets[k];
    }
    (targets, sizes_of.into_iter().flatten().collect())
}

/// Step 4: deal the persons into households in ascending (priority, sequence
/// number), in three passes: heads, children, everyone else.
fn assemble(bands: &[(u16, u16)], persons: &[Candidate], sizes: &[Int]) -> Vec<Vec<Member>> {
    let n_hh = sizes.len();
    let capacity: Vec<usize> = sizes.iter().map(|&s| count(s)).collect();
    // Persons are stored in sequence order, so the index is the tie-break.
    let mut order: Vec<usize> = (0..persons.len()).collect();
    order.sort_by_key(|&i| (persons[i].priority, i));
    let band_from = |i: usize| bands[persons[i].band].0;
    let member = |i: usize, role: Role| Member {
        sex: persons[i].sex,
        age: persons[i].age,
        role,
        cell: persons[i].cell,
    };
    let mut placed = vec![false; persons.len()];
    let mut members: Vec<Vec<Member>> = capacity.iter().map(|&c| Vec::with_capacity(c)).collect();

    // Pass 1: one head per household, adults first.
    let mut heads: Vec<usize> = order
        .iter()
        .copied()
        .filter(|&i| band_from(i) >= ADULT_AGE)
        .take(n_hh)
        .collect();
    if heads.len() < n_hh {
        let missing = n_hh - heads.len();
        heads.extend(
            order
                .iter()
                .copied()
                .filter(|&i| (YOUNG_HEAD_AGE..ADULT_AGE).contains(&band_from(i)))
                .take(missing),
        );
    }
    for &i in &heads {
        placed[i] = true;
    }
    if heads.len() < n_hh {
        let missing = n_hh - heads.len();
        let extra: Vec<usize> = order
            .iter()
            .copied()
            .filter(|&i| !placed[i])
            .take(missing)
            .collect();
        for &i in &extra {
            placed[i] = true;
        }
        heads.extend(extra);
    }
    for (h, &i) in heads.iter().enumerate() {
        members[h].push(member(i, Role::Head));
    }

    // Pass 2: children under 15 go to households of two or more whose head is
    // aged 20-59, one per household per round.
    let children: Vec<usize> = order
        .iter()
        .copied()
        .filter(|&i| !placed[i] && bands[persons[i].band].1 <= YOUNG_HEAD_AGE)
        .collect();
    let mut next_child = 0;
    let mut eligible: Vec<usize> = (0..n_hh)
        .filter(|&h| {
            capacity[h] >= 2 && (ADULT_AGE * MONTHS..60 * MONTHS).contains(&members[h][0].age)
        })
        .collect();
    while next_child < children.len() && !eligible.is_empty() {
        let mut still = Vec::with_capacity(eligible.len());
        for &h in &eligible {
            if next_child == children.len() {
                break;
            }
            if members[h].len() < capacity[h] {
                let i = children[next_child];
                next_child += 1;
                members[h].push(member(i, Role::Child));
                placed[i] = true;
            }
            if members[h].len() < capacity[h] {
                still.push(h);
            }
        }
        eligible = still;
    }

    // Pass 3: everyone left fills the remaining places, household by household.
    let mut rest = order.iter().copied().filter(|&i| !placed[i]);
    'households: for h in 0..n_hh {
        let (head_sex, head_age) = (members[h][0].sex, i32::from(members[h][0].age));
        let mut has_partner = false;
        while members[h].len() < capacity[h] {
            let Some(i) = rest.next() else {
                break 'households;
            };
            let age = i32::from(persons[i].age);
            let adult = persons[i].age >= ADULT_AGE * MONTHS;
            let role = if !adult && head_age - age >= 18 * i32::from(MONTHS) {
                Role::Child
            } else if adult
                && persons[i].sex != head_sex
                && (age - head_age).abs() <= 15 * i32::from(MONTHS)
                && !has_partner
            {
                has_partner = true;
                Role::Partner
            } else {
                Role::Other
            };
            members[h].push(member(i, role));
        }
    }
    members
}
