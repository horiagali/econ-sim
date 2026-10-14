---
id: society/population-inactivity-enrolment
title: Population Generator, Stage K — Kinds of Inactivity and Enrolment
status: draft
owner: horia
depends_on: [society/population-attributes, society/population-groups, society/population-generator, society/education, economy/social-transfers, economy/labor-market, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/inactivity-enrolment-romania]
updated: 2026-10-14
---

# Population Generator, Stage K — Kinds of Inactivity and Enrolment

## Purpose
Give every person whom [Stage C](population-attributes.md) made "other inactive" a kind of inactivity (`inactive_kind`: homemaker, disabled, other), and give every person the level of education they are enrolled at (`enrolled`), consistent with Stage C's pupils and students. It runs after Stage C, once, when a scenario is built. With it the three inactive values of [population-groups](population-groups.md) (inactive-homemaker, inactive-disabled, inactive-other) exist; [social transfers](../economy/social-transfers.md) can pay disability benefits and student grants to the right people; [education](education.md) starts from the school statistics level by level; and the [labour market](../economy/labor-market.md) can bring each kind back to work at its own pace.

## Real-world basis
Research note [`inactivity-enrolment-romania`](../../01-research/notes/inactivity-enrolment-romania.md).
- **Enrolment (measured):** pupils and students of the school year 2021/22 by level, sex and single year of age (Eurostat `educ_uoe_enra02`, 3.50 million, the same total as INS). Ages are on 1 January 2022.
- **Homemakers (measured):** 1,169,829 in Census 2021 (INS), "over 80%" of them women. They are 52% of the census's 2,252,621 persons "other inactive".
- **Inactive for reasons of health (guessed):** no table gives it. 415,000 adults of 18 to 64 hold a disability certificate, but most draw an invalidity pension (counted as retired) or work. 200,000 is assumed.
- **Other (the rest):** persons of 15 or more supported by others, by the state or from other sources, those in other situations, and the discouraged (140,000 of 15 to 64 believe no job is available).

> **Simplification:** the three kinds are national, by sex and age band, not by region. Region comes in only through Stage C's count of other inactive persons.

> **Simplification:** a kind does not depend on education, income or the household, except that homemakers are more often those who live with a young child (`homemaker_child_link`). A disabled person is not less educated than a homemaker of the same sex and age.

> **Simplification:** the age pattern of each kind (below, "Inputs") and the split of homemakers and of the disabled by sex are educated guesses (confidence low, the homemakers' sex split medium). The census totals by sex and age band, and the census count of homemakers, are kept exact.

> **Simplification:** enrolment is national, by sex and age group, not by region. Every pupil and student of Stage C is enrolled; others are enrolled only where the school statistics have more places than Stage C has pupils and students (working students, adults back in school). The retired are never enrolled.

## Scope
**This increment:** `inactive_kind` for every person, `enrolled` for every person.

**Not in this increment:** `edu_field`, `years_in_level` and the year of study; full-time or part-time; public or private school; region of the school; a split of "other" into discouraged and the rest (open question); grade repetition and drop-out (the [education](education.md) mechanic).

## Inputs
- The population of Stages A and B (household weights, ages in months, sexes, households) and the attributes of Stage C (`edu_level`, `activity`). **None is changed.**
- The margin file, integer counts of real persons:

| Table | Cells | Meaning |
|---|---|---|
| `inactive_kind[sex, age_band, kind]` | 2 × 21 × 3 | persons other inactive by kind; each row equals the census count of the sex and age band (Stage C's margin file, summed over regions) |
| `enrolment[sex, enrolment_age, level]` | 2 × 37 × 7 | pupils and students by level |
| `population[sex, enrolment_age]` | 2 × 37 | census residents, to turn the counts into enrolment rates |

  Age bands are those of Stage C. Enrolment ages: under 2, each year from 2 to 29, five-year groups from 30 to 64, and 65 or over (the groups Eurostat publishes for every age). The normalise stage builds `inactive_kind` by balancing a guessed age pattern of the three kinds to two sets of totals: the census row of each sex and age band, and the kind totals of each sex (homemakers 959,260 women and 210,569 men; disabled 90,000 women and 110,000 men; other the rest). The guessed pattern (relative weight of homemaker, disabled, other) is, for women: 15–19 0.05, 0.05, 0.90; 20–24 0.35, 0.05, 0.60; 25–29 0.60, 0.06, 0.34; 30–49 0.75, 0.07, 0.18; 50–64 0.75, 0.10, 0.15; 65–74 0.75, 0.12, 0.13; 75 or over 0.60, 0.25, 0.15; for men: 0.01, 0.06, 0.93; 0.05, 0.08, 0.87; 0.12, 0.10, 0.78; 0.25, 0.13, 0.62; 0.30, 0.18, 0.52; 0.35, 0.20, 0.45; 0.30, 0.30, 0.40.
- `rng_seed`.

## Outputs
Two columns aligned with the persons table, and their hash.

| Column | Values |
|---|---|
| `inactive_kind` | 0 none (activity is not other inactive), 1 homemaker, 2 disabled, 3 other |
| `enrolled` | 0 not enrolled, 1 pre-school (ISCED 0), 2 primary (1), 3 lower secondary (2), 4 upper secondary (3), 5 post-secondary non-tertiary (4), 6 tertiary (5–6), 7 master or doctorate (7–8) |

Stage C's `activity` with `inactive_kind` gives the twelve activities of [population-groups](population-groups.md) for the inactive: other inactive and homemaker is inactive-homemaker, and so on.

## Update rule
There is no per-tick rule. The stage is a pure function of its inputs and `rng_seed`, in integer arithmetic. "Split" and "apportion" are those of [Stage C](population-attributes.md). A person's weight is the weight of their household; their year of age is age in months ÷ 12, rounded down.

**Draw.** `draw(PopulationGen, tick 11, entity = index in the persons table)`. Its first `u64` is the person's priority; its second, in millionths (the top 20 bits × 10⁶, shifted right by 20, as in Stage H), is the person's link draw. Ticks 0 to 10 are taken by Stages A to J and the scale world.

### Step 1 — kinds of inactivity
For each sex, with the carry starting at zero, go through the age bands from youngest to oldest. For one band:
1. The members are the band's persons whose `activity` is other inactive. A band without members is skipped.
2. The targets are the band's three counts. If all are zero, the sex's totals over all bands; if those are zero too, rejected (`NoMargin`).
3. Order: first the members who live in a household with someone younger than 7 years (age under 84 months) and whose link draw is below `homemaker_child_link`; then the others. Inside each part, ascending (priority, index).
4. **Split** the members over homemaker, disabled, other, in that order. The carry passes to the next band.

The first members in the order become homemakers, so a person who lives with a young child is more likely to be one.

### Step 2 — enrolment
For each sex, with the rounding carry (one per level) and the owed places (one per level) starting at zero, go through the enrolment age groups from youngest to oldest. For one group, whose persons have weight $W$:
1. **Targets.** Apportion $W$ over the group's seven counts and what the census population $P$ has beyond them, $\max(P - \sum \text{counts}, 0)$; keep the seven levels (all zero if the eight numbers are zero). Add the owed places, and set them to zero. The targets are the school statistics at this group's enrolment rates.
2. **Who must be enrolled:** every pupil and student. Their weight $M$ splits into $M_{hi}$, those with `edu_level` ≥ 1 (upper secondary completed), and $M_{lo}$, the rest.
3. **Who may be enrolled:** persons whose `activity` is child, employed, unemployed or other inactive. They form an upper pool (`edu_level` ≥ 1, weight $V_{hi}$) and a lower pool ($V_{lo}$), each in ascending (priority, index).
4. **How many more.** With $T_{after}$ the targets of post-secondary and above and $T_{before}$ those of the four levels before:
   - from the upper pool $\min(\max(T_{after} - M_{hi}, 0), V_{hi})$;
   - from the lower pool $\min(\max(T_{before} - M_{lo} - \max(M_{hi} - T_{after}, 0), 0), V_{lo})$.

   From each pool, **split** with the two targets (take, pool weight − take) and no carry; the members in the first part are taken.
5. **Levels.** Order the enrolled (step 2 and the taken) ascending by (`edu_level`, age in months, priority, index), and **split** them over the seven levels with the targets and the rounding carry. If every target is zero, the group's counts are the targets; if those are zero too, rejected (`NoMargin`). The lowest levels go to the least educated and the youngest.
6. **Eligibility.** A person given post-secondary or above with `edu_level` 0 is enrolled in upper secondary instead, and their weight is added to the owed places of the level they were given: the next age group fills it.

Everyone not enrolled in step 5 has `enrolled` 0.

> **Simplification:** the owed places matter at 19. Stage C gives the in-education persons of the band 15 to 19 their upper-secondary diplomas in priority order, not by age, so many 19-year-olds have `edu_level` 0 although the school statistics have most of them in tertiary. Those places move to age 20 (about 50,000 places), where they go to working persons who have the diploma.

**Inactivity and enrolment hash** (for golden and differential tests): FNV-1a 64 over the person count (`u32`, little-endian), then each person's `inactive_kind` and `enrolled` (one byte each).

## Player levers
None.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `homemaker_child_link` | 0.5 | 0–1 | 0: who is a homemaker is unrelated to the household. 1: persons other inactive who live with a child younger than 7 come first among the homemakers of their sex and age band |

The guessed totals and age pattern of the kinds are data in the margin file (`provenance.guesses`), not parameters of the stage.

## Interactions
- **Before:** Stages A to C; the pipeline's normalise stage (`normalise_inactivity_enrolment.py`, which reads Stage C's margin file). The stage reads nothing from Stages D to J; it is lettered K because it was specified after them.
- **After:** [social transfers](../economy/social-transfers.md) (disability and sickness benefit to the disabled; student grants and loans to the enrolled in tertiary and post-secondary), [education](education.md) (enrolment, capacity and spending per pupil by level), the [labour market](../economy/labor-market.md) (re-entry of the inactive by kind; working students), [households](../economy/households.md) (childcare: homemakers live with young children more often), the scenario builder.

## Edge cases & failure modes
- **A person other inactive in a band with no counts** (cannot happen with the fixture: nobody under 15 is other inactive): the sex's totals are the targets.
- **A group whose pupils and students outnumber the school statistics** (ages 6 to 17: Stage C makes everyone of 6 to 14 a pupil, and the census has more persons in education at 15 to 17 than the schools count): all are enrolled, split over the levels in proportion to the targets. Primary, lower secondary and upper secondary then have about 18% more pupils nationally than the school statistics (open question).
- **A pool smaller than what it should give** (few working 19-year-olds): everyone in it is taken; the shortfall is not carried.
- **A place after upper secondary that falls to someone without the diploma:** upper secondary instead; the place is owed to the next age group of the same sex. What is still owed after the last group (65 or over) is dropped.
- **No bands or no enrolment ages, tables whose length does not match, sexes or kinds or levels other than those listed, all counts zero, columns of another population:** rejected with a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/inactivity_enrolment.rs` (protected; written in a test-authoring session before the Rust code), compiled only with a crate feature `inactivity_enrolment`. "The fixtures" are the margin files committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42. "Targets" for enrolment are the school statistics at each sex and age group's enrolment rate, on the synthetic population (count × weight of the group ÷ census population of the group).

- [ ] **AC-POPK-01** `[unit]` Structure, at each of the three scales: a person has a kind exactly when their activity is other inactive; every pupil and student is enrolled; nobody retired is enrolled; a child (activity) is enrolled only in pre-school; everyone enrolled at post-secondary or above has `edu_level` ≥ 1; everyone enrolled whose activity is employed, unemployed or other inactive is 15 or older.
- [ ] **AC-POPK-02** `[unit]` The stage changes nothing before it: the population's state hash and the attribute hash are those of their own golden tests.
- [ ] **AC-POPK-03** `[unit]` At each of the three scales, each kind's national weighted total by sex is within 2% of the margin file's total, and in every sex and age band with at least 50 expected synthetic persons other inactive, each kind's share is within 3 points of its share in the margin file.
- [ ] **AC-POPK-04** `[unit]` At each of the three scales, the national weighted count of pre-school, post-secondary, tertiary, and master and doctorate is each within 6% of its target; and in every sex and enrolment age group with at least 50 expected synthetic enrolled persons, except ages 19 and 20, each level's share of the enrolled is within 3 points of its share in the school statistics.
- [ ] **AC-POPK-05** `[unit]` With `homemaker_child_link` = 1 the share of homemakers who live with a child younger than 7 is at least 10 points above that with 0 (1:1000 and 1:100); with 0 it is within 3 points of the same share among all persons other inactive. AC-POPK-03 holds for 0, 0.5 and 1.
- [ ] **AC-POPK-06** `[unit]` Same inputs and seed give identical columns (equal hash). A different seed changes who has which kind and level; AC-POPK-03 and AC-POPK-04 still hold.
- [ ] **AC-POPK-07** `[golden]` The inactivity and enrolment hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPK-08** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns.
- [ ] **AC-POPK-09** `[unit]` Tables of the wrong length, the wrong sexes, kinds or levels, no age bands, all counts zero, and attributes of a different length are each rejected with a named error and produce no output.

Measured on 2026-10-13 with the Python reference on the fixtures (seed 42; the targets of enrolment as defined above):

| | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|
| Largest gap of a kind's total by sex, % | 0.5 | 0.2 | 0.0 |
| Largest gap of a kind's share in a band, points | 1.3 | 1.1 | 0.4 |
| Share living with a child under 7: homemaker, disabled, other | 27%, 14%, 10% | 26%, 12%, 10% | 26%, 11%, 10% |
| Pre-school, post-secondary, tertiary, master and doctorate against targets, % | +1.0, +0.0, +0.7, −1.0 | +0.8, −0.1, +1.6, +0.4 | +0.8, +0.3, +1.9, +0.5 |
| Primary, lower secondary, upper secondary against targets, % | +18, +19, +17 | +19, +18, +19 | +19, +18, +19 |
| Largest gap of a level's share in an age group, other than 19 and 20, points | 1.2 | 1.1 | 0.9 |
| Enrolled who are not pupils or students (employed, unemployed, other inactive), thousand | 269, 25, 71 | 289, 26, 67 | 293, 24, 70 |

With seed 7 at 1:1000 the largest gaps are 1.5 points (kinds) and 2.2 points (levels), and tertiary is 4.4% above its target. With `homemaker_child_link` = 0 and 1, the share of homemakers living with a young child is 20% and 35% (1:1000) and 19% and 34% (1:100).

Hashes of the reference, for the test writer to record as golden values (the earlier hashes are unchanged):

| Scale | Inactivity and enrolment hash |
|---|---|
| 1:1000 | `d23518ac1ea5168c` |
| 1:200 | `29a74f43a637a5de` |
| 1:100 | `e80168c0de0e39ab` |
| 1:10 | `fd1f841d318212d1` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. There is no Rust code yet (tests come first, ADR-0014); the skeleton will be `crates/econ-popgen/src/inactivity.rs`.

```rust
// crate econ-popgen, module `inactivity` (no I/O), re-exported at the crate root
pub enum InactiveKind { None = 0, Homemaker = 1, Disabled = 2, Other = 3 }
pub enum Enrolled { None = 0, PreSchool = 1, Primary = 2, LowerSecondary = 3, UpperSecondary = 4,
                    PostSecondary = 5, Tertiary = 6, MasterDoctorate = 7 }
pub const INACTIVE_KINDS: usize = 3;
pub const ENROLMENT_LEVELS: usize = 7;
pub struct InactivityEnrolmentMargins {
    pub age_bands: Vec<(u16, u16)>,          // as in `Margins` (Stage C's bands); the last is open
    pub enrolment_ages: Vec<(u16, u16)>,     // years of age, [first, first after); the last is open
    pub inactive_kind: Vec<u64>,             // [sex][age_band][kind], row-major
    pub enrolment: Vec<u64>,                 // [sex][enrolment_age][level], row-major
    pub population: Vec<u64>,                // [sex][enrolment_age]
}
pub struct InactivityParams { pub rng_seed: u64, pub homemaker_child_link_ppm: u32 }   // default 500_000
pub struct PersonInactivityEnrolment { pub inactive_kind: Vec<InactiveKind>, pub enrolled: Vec<Enrolled> }
pub enum InactivityError { EmptyTable, ShapeMismatch, NoMargin }

pub fn assign_inactivity_enrolment(pop: &Population, attrs: &PersonAttributes,
                                   margins: &InactivityEnrolmentMargins, params: &InactivityParams)
    -> Result<PersonInactivityEnrolment, InactivityError>;
impl PersonInactivityEnrolment { pub fn state_hash(&self) -> u64; }
```

- Command line: `econ-cli synth-population … --attributes FILE --enrolment FILE` adds the two columns to `persons.arrow` and the hash to `fit_report.json`.
- Margin file: JSON with `sexes`, `age_bands`, `inactive_kinds`, `inactive_kind`, `enrolment_levels`, `enrolment_ages`, `enrolment`, `population`, plus `totals` and `provenance` (sources, `manual`, `guesses`).
- Python reference: `python/reference/popgen_reference.py`, functions `assign_inactivity_enrolment`, `inactivity_enrolment_hash` and `inactivity_enrolment_fit` (`--enrolment FILE` on its command line, with `--attributes FILE`). Where this text and the reference disagree on a detail of ordering or rounding, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/inactivity_enrolment2021_ro.json`, built by `python/pipeline/normalise_inactivity_enrolment.py` from Eurostat `educ_uoe_enra02` (dataset `ro_enrolment_level_age_2021_22`), Stage C's margin file and the values entered by hand.

## Open questions
- [x] **Kept for now (owner, 2026-10-14).** **200,000 inactive for reasons of health** (low confidence), 55% men. Keep, or look for the INS census table of inactive persons by category, age and sex (on the census website, not on Eurostat), which would also give the homemakers by age?
- [x] **Decided (owner, 2026-10-14): this stage leaves that share of pupils not enrolled.** **Pupils of 7 to 14 above the school statistics.** Stage C makes everyone of 6 to 14 a pupil, so primary and lower secondary are 18% above the school count (10–20% of children of those ages are not counted in school). Accept; or let this stage leave that share of pupils not enrolled (a "pupil" who is not enrolled); or change Stage C so that some children of 6 to 14 are not pupils (changes Stage C's golden hashes).
- [x] **Accepted for now (owner, 2026-10-14).** **19-year-olds without the diploma.** Stage C gives diplomas to the in-education persons of 15 to 19 in priority order, so tertiary places of 19-year-olds move to 20 (owed places). Accept; or have a later version of Stage C give diplomas by age (changes Stage C's golden hashes).
- [x] **Not now (owner, 2026-10-14).** **Split "other" further** into the discouraged (want to work, 252,000 aged 15 to 64 in the Labour Force Survey) and the rest, if the labour market needs it.
- [x] **Accepted (owner, 2026-10-14).** **Bands of AC-POPK-03 and AC-POPK-04** (3 points, 2%, 6%): owner to accept.
