---
id: society/education
title: Education
status: draft
owner: horia
depends_on: [society/population-groups, economy/fiscal-policy, society/population-inactivity-enrolment, society/population-attributes, economy/labor-market, economy/demographics, game/world-loop, adr/0006-determinism-contract, adr/0017-time-base]
research: [research/education-flows-romania, research/inactivity-enrolment-romania]
updated: 2026-10-26
---

# Education

## Purpose
Turn spending, student finance and family background into enrolment, graduation and skills. Skills feed labour productivity, wages, crime and health. Education is the main long-run growth lever.

## Real-world basis
- Mincer returns to schooling (~8–10% wage per extra year, more for tertiary in many countries).
- Enrolment responds to cost (tuition, foregone earnings), student support and parental income; dropout is higher for low-income students.
- Education quality (spending per pupil, teacher pay, class size) has a measurable but modest effect on outcomes.

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $Q^{edu}_{r,\ell}$ | `edu_quality` | Quality per region and level (primary, secondary, vocational, tertiary) | index 0–1 |
| $\text{Cap}_{r,\ell}$ | `edu_capacity` | Places available (schools, universities built) | students |
| $\text{Teach}_{r,\ell}$ | `edu_teachers` | Teachers employed (public sector employment) | persons |

Enrolment and attainment are person attributes (`enrolled`, `edu_level`, `edu_field`).

## Inputs
Education ministry budget and its split by level (fiscal-policy); school and university infrastructure (infrastructure); student finance and tuition (social-transfers); household income class and parents' education; graduate wage premium (labor-market); youth unemployment (low job prospects raise enrolment).

## Outputs
Transitions between education levels; skill composition of the labour force; `edu_quality` (read by labor-market productivity).

## Update rule (per tick; school-year events yearly)
**Enrolment probability** for a person eligible for level $\ell$ (aligned to capacity):
```math
p^{enrol}_{k,\ell} = \text{logistic}\big(\beta_0 + \beta_w \cdot \text{premium}_\ell + \beta_f \cdot \text{support}_k - \beta_c \cdot \text{cost}_{k,\ell} + \beta_q \cdot q_k + \beta_u \cdot u^{youth}_r\big)
```
capped by `edu_capacity`. If capacity binds, quality falls (crowding).

**Completion** (per year of study): $p^{complete} = f(Q^{edu}, \text{support}_k, q_k)$, else dropout back to the labour market with the previous education level.

**Quality:**
```math
Q^{edu}_{t+1} = Q^{edu}_t + \lambda \big(Q^*(\text{spend per student}, \text{teacher pay ratio}, \text{class size}) - Q^{edu}_t\big)
```
Slow adjustment (λ small: years).

**Skill effect:** worker effective skill $=$ base by education level × $(1 + \gamma \cdot Q^{edu}$ when educated$)$. Feeds the labour productivity index in [production](../economy/production.md).

> **Simplification:** primary and lower-secondary schooling is compulsory; in v1 the enrolment decision above applies from upper secondary up. It is not universal in Romania: the starting population leaves about 380,000 children and young people of 6 to 19 not enrolled ([stage K](population-inactivity-enrolment.md): pupils above the school statistics' places, owner 2026-10-14), standing for dropouts and early school leavers. They get no school place, no spending per pupil and no progression; a policy that brings them back (attendance-linked benefits, school meals) is a lever for M14; the first version (M5, below) keeps them out of school and sends them to the labour market at 16.

## First version (roadmap M5)
What is built first, on the `World` of [game/world-loop](../game/world-loop.md): persons move through school and university once a year, graduates gain their new `edu_level`, and those who leave enter the [labour market](../economy/labor-market.md). There is no quality, no enrolment decision and no lever yet (all M14); school places stay those of the start until [infrastructure](../economy/infrastructure.md) builds more (M11). Research: [education flows in Romania](../../01-research/notes/education-flows-romania.md).

**Periods.** A tick is a day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)); day 0 is 1 December 2021, in the school year 2021/22 that Stage K's enrolment describes. Education acts at two monthly closes only:
- **The June close (end of the school year):** dropout, progression, completion, continuation to the next level, and leaving. It runs after the month's statistics, so June's statistics show the school year before it ends; leavers are in the labour market from 1 July.
- **The August close (intake for 1 September):** children who start pre-school or primary school, and the continuing students placed in their new level, are checked against the school places.

In every other month nobody's `enrolled`, `study_year` or `edu_level` changes. Ageing, births, deaths and migration belong to [demographics](../economy/demographics.md); education reads the ages it keeps (age in months) and gives a newborn `enrolled` none, `study_year` 0 and `edu_level` 0. A person who dies or emigrates leaves the place they held.

**State.**

| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| — | `enrolled` | Level the person is enrolled at (Stage K, now kept by the `World`) | category (8) |
| — | `study_year` | Year of study within that level, from 0 | years |
| $\text{Cap}_\ell$ | `school_places[ℓ]` | Places at level $\ell$, national, weighted persons; fixed in M5 | persons |
| — | `activity` | Stage C's activity, now kept by the `World`; education moves a leaver from pupil or student to unemployed or other inactive | category |

**The levels.**

| $\ell$ (`enrolled`) | Years | Typical starting age | Completion sets `edu_level` to at least | After completion |
|---|---|---|---|---|
| 1 pre-school | 3 | 3 | — | primary at 6 |
| 2 primary (with the preparatory grade) | 5 | 6 | — | lower secondary |
| 3 lower secondary | 4 | 11 | — (0, ISCED 0–2) | upper secondary with `continue_rate[3→4]`, else leave |
| 4 upper secondary | 4 | 15 | 1 (ISCED 3) | tertiary with `continue_rate[4→6]`, post-secondary with `continue_rate[4→5]`, else leave |
| 5 post-secondary non-tertiary | 2 | 19 | 2 (ISCED 4) | leave |
| 6 tertiary (bachelor) | 3 | 19 | 3 (ISCED 5–6) | master or doctorate with `continue_rate[6→7]`, else leave |
| 7 master or doctorate | 2 | 22 | 4 (ISCED 7–8) | leave |

> **Simplification:** one duration per level. Vocational upper secondary (three years), four-year engineering and six-year medical degrees, and three-year doctorates are folded into the averages above. Graduates are therefore right in number but not in timing for those programmes.

> **Simplification:** completing upper secondary gives ISCED 3 and the right to go on; the baccalaureate (passed by about two thirds of candidates) is not modelled separately. Its effect is inside `continue_rate[4→6]`.

**Starting state.** At load, `enrolled` and `activity` come from the scenario's persons table (Stages C and K). Each enrolled person's `study_year` is their age on 1 September 2021 (age in months at day 0, less 3, in whole years) less the level's typical starting age, if that lies between 0 and the level's years less 1. A person outside that range (repeating, late or adult students: about 360,000 enrolled persons who are employed, unemployed or other inactive) gets a uniform draw over the level's years: `draw(Education, tick 0, entity = person)`, first word, modulo the years. `school_places[ℓ]` is the weighted count of persons enrolled at $\ell$ at load, so the 380,000 pupils above the school statistics' places (Stage K) have none.

**The June close.** For each enrolled person, in index order, with `draw(Education, tick = the day of the close, entity = person)`, first word in millionths $u_1$, second $u_2$:
1. **Dropout.** If $u_1$ < `dropout_rate[ℓ]`, the person leaves the level: `enrolled` none, `study_year` 0, `edu_level` unchanged.
2. **Progression.** Otherwise `study_year` + 1. If it reaches the level's years, the person **completes** it: `edu_level` ← max(`edu_level`, the level's completion value), and with $u_2$ against the continuation rates (in the table's order) they are given the next level at `study_year` 0, or leave. Pre-school completers go to primary and primary completers to lower secondary with no draw.
3. **Leaving.** A person who left (step 1 or 2), and any person not enrolled whose `activity` is pupil or student, becomes a **leaver** if 16 or older (`min_leaving_age`); a younger one keeps the activity pupil until a June close at which they are 16. A leaver with no job becomes a job seeker (`activity` unemployed, looking for work, `months_unemployed` 0) with probability `leaver_seek_share[edu_level]` (the draw's third word), else other inactive (`inactive_kind` other). A leaver who already has a job keeps it (`activity` employed). With this rule the start's pupils not enrolled reach the labour market at 16, as [Stage K](population-inactivity-enrolment.md) foresaw (from 15 they may look for work).

**The August close.** Intake for 1 September:
1. **Pre-school:** each child not enrolled who is 3, 4 or 5 on 1 September is given pre-school with probability `preschool_intake[age]` (draw of the close).
2. **Primary:** each child who is 6 on 1 September and not in primary is given primary with probability `primary_intake`.
3. **Places.** For each level, the persons given it at this close (continuing from the level below in June, or starting now) are the **entrants**; those already in it (`study_year` > 0) the **continuing**. Continuing students keep their place even above `school_places[ℓ]`. Entrants are admitted in ascending (keyed priority, index) while the level's enrolled weight stays within its places; the rest are refused: `enrolled` none. A refused entrant of 16 or more becomes a leaver at once (step 3 of June); a younger one stays a pupil not enrolled, as Stage K's pupils above the places.

With the start's places, primary binds at first: a year of 6-year-olds (about 190,000) times `primary_intake` is above primary's places per grade (867,000 ÷ 5), so about one child in ten is left out, as at the start. As cohorts shrink (births of about 150,000 a year from 2023, [demographics](../economy/demographics.md)) the places stop binding. This is the shortage that school building fixes in M11.

**Labour-market entry.** A leaver looking for work is matched by the [labour market](../economy/labor-market.md) with their new `edu_level`, which sets the tiers they qualify for (ISCED 3–4: mid; 5–8: high). Hires go to the longest unemployed first, so graduates queue behind those already looking (young people's unemployment is higher). School leavers have no unemployment benefit: they have no contribution record ([social transfers](../economy/social-transfers.md)). Student grants follow `enrolled` and stop when a person leaves.

**Report.** Each June and August close writes an `EducationYear`: by level, enrolled before, dropouts, completers, continuing to each next level, entrants, refused, enrolled after; leavers by `edu_level`, seeking and not. For each level: enrolled after = enrolled before − dropouts − completers + entrants admitted (weighted, exact).

**Derived indicators** (each June close, for the statistics registry of M6): `early_leavers_rate`, the weighted share of persons of 18 to 24 with `edu_level` 0 who are not enrolled; `tertiary_attainment`, the share of persons of 25 to 34 with `edu_level` 3 or 4; graduates by level.

> **Simplification:** one national pool of places per level, as Stage K's enrolment is national. A child in a county with spare places and one in a county without are treated alike. Places by county come with M11's schools.

> **Simplification:** nobody returns to school in M5: adults do not enrol, and pupils not enrolled are never enrolled (the lever that brings them back is M14's). The start's adult students graduate or drop out and are not replaced, so tertiary and post-secondary enrolment fall in the first years towards the flow of young entrants.

> **Simplification:** dropout and continuation rates are the same for everyone at a level: they do not depend on household income, parents' education, region or grades (M14).

**Not in M5:** the enrolment decision and its logistic rule, quality, capacity by region, teachers, fields of study, grade repetition, private schools, adults returning, student finance beyond Stage K's grants, the skill effect and wage returns, emigration of graduates (that is [migration](../economy/migration-remittances.md)'s, which reads `edu_level`).

## Player levers
**M5:** none. The education budget lever of M4 pays staff and inputs but does not yet move enrolment, completion or quality.

**From M14:** education budget, its split by level, and its split between running (staff, inputs: below the cost of the schools they are understaffed, above it the extra raises quality with diminishing returns) and investment (new schools and maintenance through [infrastructure](../economy/infrastructure.md); owner, 2026-10-22); teacher pay (public wages); building schools and universities; tuition fee (tertiary); student grants and loans; vocational training subsidies.

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `beta_*` | to calibrate | Enrolment sensitivities |
| `edu_quality_lambda` | 0.02 / month | Speed of quality change |
| `skill_gamma` | 0.2 | Quality impact on skill |

First version (M5). All are educated guesses (confidence low unless stated), tuned so that the stocks of the start roughly hold and AC-EDU-04 and 06 pass; see the [research note](../../01-research/notes/education-flows-romania.md).

| Parameter | Default | Range | Effect |
|---|---|---|---|
| `dropout_rate[ℓ]` | pre-school 0, primary 0.005, lower secondary 0.01, upper secondary 0.02, post-secondary 0.08, tertiary 0.12, master or doctorate 0.08 (a year) | 0–0.5 | leaving before completion; compulsory levels medium confidence (INS 1.2% for both, 2020/21, partly returning) |
| `continue_rate[3→4]` | 0.98 | 0–1 | lower to upper secondary |
| `continue_rate[4→6]`, `continue_rate[4→5]` | 0.65, 0.20 | 0–1, sum ≤ 1 | upper secondary to tertiary and to post-secondary; keeps about 100,000 bachelor entrants a year |
| `continue_rate[6→7]` | 0.70 | 0–1 | bachelor to master or doctorate |
| `primary_intake` | 0.97 | 0–1 | share of a year of 6-year-olds who seek a primary place (the rest never enrol; places may refuse more) |
| `preschool_intake[a]` | 3: 0.66, 4: 0.35, 5: 0.20 | 0–1 | chance that a child not enrolled starts pre-school at that age (Stage K: about 66%, 77% and 81% enrolled at 3, 4 and 5) |
| `leaver_seek_share[e]` | `edu_level` 0: 0.40, 1: 0.70, 2: 0.80, 3: 0.85, 4: 0.90 | 0–1 | leavers who look for work; the rest are inactive (Romania's NEET rate of 15–29 was about 20% in 2021) |
| `min_leaving_age` | 16 | 15–18 | legal working age; younger leavers stay pupils |

## Interactions
→ labour market (supply by skill), production (productivity), social outcomes (crime ↓, health ↑), opinion (educated people have different ideology and interests), demographics (fertility ↓ with education).

## Edge cases
**First version (M5):**
- **A level with no places** (none enrolled at the start; cannot happen with the fixture): every entrant is refused.
- **A person completing a level below their `edu_level`** (an adult with a diploma in upper secondary): `edu_level` does not fall.
- **A person enrolled at post-secondary or above with `edu_level` 0** (Stage K's owed places, step 6, make them upper secondary instead, so none should exist): treated as upper secondary in their last year.
- **A leaver who is retired or has a pension:** cannot happen (Stage K never enrols the retired); if a pension appears it does not change the activity rule.
- **No June close** (a run shorter than seven months): nothing moves; the August close without a June close before it admits only new pre-school and primary entrants.
- **A run started on another date:** the two closes are the June and August closes of the calendar, whatever the start; `study_year` uses 1 September before day 0.
- **Every person enrolled dies or emigrates:** the level is empty; its places stay.

**Later versions:**
- Capacity shortage → waiting lists, quality drops, not negative enrolment.
- Over-education: graduates exceeding high-skill jobs → underemployment (employed in mid-skill jobs) → lower measured return and lower approval.

## Acceptance tests
IDs are stable. The M5 criteria (first version) go in `crates/econ-core/tests/acceptance/education.rs` (protected; written by the test-writer subagent before the code), compiled with a crate feature `education`. "1:100" and "1:1000" are the Romania fixture scenario at that scale, seed 42, under `Law::law_2021()`, with no lever changes.

**First version (M5):**
- [ ] **AC-EDU-01** `[unit]` Starting `study_year`: for a person whose age on 1 September 2021 less the level's starting age lies within the level's years, it is that difference; otherwise it is the keyed draw modulo the years. `school_places[ℓ]` equals the weighted enrolled count of each level at load. Same inputs and seed give the same columns.
- [ ] **AC-EDU-02** `[unit]` The June rule for one person, for given draws: a dropout keeps `edu_level` and leaves; a completer's `edu_level` becomes the maximum of the old one and the level's completion value, never lower; continuation follows the rates in the table's order; a leaver of 16 or more without a job becomes a seeker or other inactive by `leaver_seek_share`, one with a job stays employed; a leaver under 16 stays a pupil not enrolled.
- [ ] **AC-EDU-03** `[unit]` Places at the August close: continuing students are never removed; entrants are admitted in ascending (keyed priority, index) while the level's weight stays within its places; with a cohort of 6-year-olds above primary's free places, exactly the surplus (to within one person's weight) is refused and stays not enrolled; with places to spare nobody is refused.
- [ ] **AC-EDU-04** `[sim]` At 1:100, the weighted completers of each of the June closes of 2022, 2023 and 2024 are within 25% of 2021's graduates for upper secondary (175,934) and bachelor (76,593), and within 30% for post-secondary (31,705) and master and doctorate (54,941).
- [ ] **AC-EDU-05** `[sim]` At 1:1000: in the month after a June close, the weighted job seekers of each tier rise by at least the leavers who seek and qualify for it, less those hired that month; every leaver hired in a tier qualifies for it with their new `edu_level`; in the months other than June and August no person's `enrolled`, `study_year` or `edu_level` changes.
- [ ] **AC-EDU-06** `[sim]` Over a 10-year run at 1:100, at each June close the early leavers rate of 18 to 24 is between 10% and 22% (2021: 15.3%) and the tertiary attainment of 25 to 34 between 18% and 32% (2021: 23.3%).
- [ ] **AC-EDU-07** `[sim]` At 1:1000, no person who is a pupil not enrolled at the start is ever enrolled; each becomes a leaver at the first June close at which they are 16 or older.
- [ ] **AC-EDU-08** `[unit]` The `EducationYear` report balances at every June and August close of a 3-year run at 1:1000: for each level, enrolled after = enrolled before − dropouts − completers + entrants admitted; leavers = dropouts + completers who do not continue + refused entrants of 16 or more + the not-enrolled pupils and students who turned 16; all weighted, exact.
- [ ] **AC-EDU-09** `[unit]` Determinism: two runs with the same seed give the same education hash (FNV-1a 64 over the person count, then each person's `enrolled`, `study_year`, `edu_level` and `activity`, one byte each) after three school years; at 1:1000 a different seed changes it, and AC-EDU-04 still holds at 1:100 with seed 7.

**Later versions (M14):**
- [ ] Raising tertiary student grants by 50% raises tertiary enrolment within 1–2 school years, more for low-income households.
- [ ] Cutting education spending by 30% lowers education quality gradually over 5–10 years, not instantly.
- [ ] A cohort with higher tertiary attainment has higher average wages 5–10 years later, holding demand constant.
- [ ] Graduates beyond high-skill demand show up as underemployment, not as unemployment only.

## API sketch
First version (M5). Names may change before lock; shapes should not. There is no code yet (tests come first, ADR-0014).

```rust
// crate econ-core, module `world::education` (feature `education`)
pub struct EducationParams {
    pub dropout_ppm: [u32; 7],            // by level, pre-school first
    pub continue_lower_to_upper_ppm: u32, // continue_rate[3→4]
    pub continue_upper_to_tertiary_ppm: u32,
    pub continue_upper_to_post_ppm: u32,
    pub continue_bachelor_to_master_ppm: u32,
    pub primary_intake_ppm: u32,
    pub preschool_intake_ppm: [u32; 3],   // ages 3, 4, 5
    pub leaver_seek_ppm: [u32; 5],        // by edu_level
    pub min_leaving_age: u8,              // years
}
pub const LEVEL_YEARS: [u8; 7] = [3, 5, 4, 4, 2, 3, 2];
pub const LEVEL_START_AGE: [u8; 7] = [3, 6, 11, 15, 19, 19, 22];

pub struct LevelYear { pub before: u64, pub dropouts: u64, pub completers: u64,
                       pub continuing_to: [u64; 7], pub entrants: u64, pub refused: u64, pub after: u64 }
pub struct EducationYear { pub levels: [LevelYear; 7], pub leavers_seeking: [u64; 5],
                           pub leavers_inactive: [u64; 5], pub leavers_employed: [u64; 5] }

/// Pure rule for one person at the June close (AC-EDU-02).
pub fn june_step(enrolled: Enrolled, study_year: u8, edu_level: EduLevel, age_months: u16,
                 has_job: bool, draws: [u32; 3], p: &EducationParams) -> JuneOutcome;
/// Admission against places (AC-EDU-03): who is admitted, in input order.
pub fn admit(places: u64, continuing: u64, entrants: &[(u64 /*priority*/, u32 /*index*/, u32 /*weight*/)]) -> Vec<bool>;

impl World {
    pub fn education_year(&self) -> Option<&EducationYear>;   // the latest June or August report
    pub fn education_hash(&self) -> u64;                      // AC-EDU-09
    pub fn early_leavers_rate_ppm(&self) -> u32;
    pub fn tertiary_attainment_ppm(&self) -> u32;
}
```

- `StartState` gains `enrolled: Vec<Enrolled>` (Stage K's column of `persons.arrow`); the `World` keeps `activity`, `enrolled` and `study_year` in `PersonColumns`.
- A new keyed stream `Education` (ADR-0006); its draws are keyed by the day of the close and the person's index.
- The monthly close calls the June and August steps after the month's statistics.

## Open questions
**First version (M5):**
- [ ] **Rates for new cohorts against the start's pattern.** `primary_intake` is 0.97, so that early leavers come out near 15%; at the start only 82–85% of a year of age is enrolled (Stage K, partly children living abroad). With 0.97 the primary places bind for the first cohorts and leave about one child in ten out, as at the start. *Proposed:* keep 0.97 and let the places do the rest; revisit when [demographics](../economy/demographics.md) has its first births.
- [ ] **Leavers' choice between looking for work and inactivity** (`leaver_seek_share`): a fixed share by education, or a labour-market rule for the re-entry of the inactive by kind? *Proposed:* the fixed shares now; the labour market's rule when it has one (none in M4).
- [ ] **Adult students.** The start's 360,000 adults enrolled graduate or leave and are not replaced, so tertiary and post-secondary enrolment fall in the first years. *Proposed:* accept for M5 (display only, no budget effect); M14 adds adult entry with student finance.
- [ ] **Graduates' emigration.** Tertiary attainment of 25–34 has been flat at about 23% because young graduates emigrate. *Proposed:* [migration](../economy/migration-remittances.md) reads `edu_level` and age; AC-EDU-06's band (18–32%) passes with or without it.

**Later versions:**
- [ ] Private schools and universities as an industry (education industry) in addition to public?
- [ ] Brain drain: emigration of graduates when domestic premium is low?
