---
id: society/population-generator
title: Population Generator — Starting Population from Census Margins
status: draft
owner: horia
depends_on: [society/population-groups, adr/0003-people-representation, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/population-modelling-deep-research]
updated: 2026-10-10
---

# Population Generator — Starting Population from Census Margins

## Purpose
Build the starting synthetic population (households and their members, with integer weights) from published census tables, at any `sample_scale`, so that the weighted population reproduces the census. It is the `synth_population` stage of the data pipeline ([ADR-0012](../../03-architecture/decisions/0012-data-pipeline-and-licensing.md)) and the first real mechanic of Phase 1b ([roadmap](../../roadmap.md)).

It runs once, when a scenario is built. It is not part of the monthly tick.

## Real-world basis
- Population synthesis is standard in microsimulation and transport modelling: draw or build a seed of households, then reweight it so its totals match known margins (iterative proportional fitting; for households with person-level targets, iterative proportional updating, Ye et al. 2009), then turn the weights into integers. See the [population research](../../01-research/notes/population-modelling-deep-research.md).
- **Data (verified 2026-10-10, Eurostat Census 2021 round, free reuse with attribution):**

| Table | Content | Level | Used for |
|---|---|---|---|
| `cens_21cobhs_r3` | persons by household status (private / not private), sex, 5-year age group | county (NUTS 3) | person margins |
| `cens_21hhcs_r3` | private households by size (1, 2, 3, 4, 5, 6–10, 11+) | county | household margins |

  Totals for Romania: 19,053,815 persons, of whom 18,935,010 live in 7,709,139 private households and 118,805 do not (institutions, homeless). 86 small not-private cells are suppressed for confidentiality.
- Further tables exist for later increments: household composition and tenure (`cens_21hhcs_r3`, `cens_21hhct_r3`), size of locality (`cens_21l_r3`) at county level; education, activity status, occupation and industry (`cens_21ae_r2`, `cens_21a_r2`, `cens_21empn_r2`) at region (NUTS 2) level. Ethnicity and religion are published only by INS.

## Scope
**Version 1 (this spec):** households with `hh_county`, `hh_weight`, `hh_collective`; persons with `household_id`, `age`, `sex`, `role`. Everything else in [population-groups](population-groups.md) (education, activity, income, wealth, identity, opinion) is added by later increments, each bringing its own margin tables. The fitting stage below is written for any list of margin tables, so increments add data and attributes, not a new algorithm.

> **Simplification:** until the IPUMS 2011 sample is available ([ADR-0003](../../03-architecture/decisions/0003-people-representation.md)), the seed households are **assembled by rules** from the margins themselves. Who lives with whom is therefore plausible but not observed. The seed is a swappable stage: the IPUMS draw replaces "Stage A" and nothing else.

> **Simplification:** people not living in private households are generated as one-person records flagged `hh_collective`. They keep the population total exact; a real model of institutions is out of scope.

## Inputs
Margin tables of **integer counts of real units**, written by the pipeline's normalise stage with provenance:

| Table | Cells | Meaning |
|---|---|---|
| `persons_private[county, sex, age_band]` | 42 × 2 × 21 | persons in private households |
| `persons_collective[county, sex, age_band]` | 42 × 2 × 21 | persons not in private households |
| `households[county, size_class]` | 42 × 7 | private households by size class |

Age bands are five-year groups from 0–4 to 95–99, then 100+. Size classes are 1, 2, 3, 4, 5, 6–10 and 11+.

**Suppressed cells.** The normalise stage, not the generator, fills them: not-private = total − private for each cell; where the total is also suppressed, the county residual is spread over the suppressed cells by largest remainder. The generator only ever sees complete integer tables.

Parameters: `sample_scale` (from the scenario, never a constant), `rng_seed`.

## Outputs
Two column tables (the persons and households tables of [ADR-0005](../../03-architecture/decisions/0005-simulation-core-architecture.md)) and a fit report.

| Table | Columns |
|---|---|
| households | `hh_weight` (integer ≥ 1), `hh_county`, `hh_collective` |
| persons | `household_id`, `age` (months), `sex`, `role` |
| fit report | sweeps used, largest remaining error per margin table, cells that could not be fitted |

A person's weight is the weight of their household. `hh_size` is the number of members.

## Update rule
There is no per-tick rule. The generator is a pure function of (margin tables, `sample_scale`, `rng_seed`). All arithmetic on counts and weights is integer; floats are not used, so results are identical on every platform and in the Python reference.

**Notation.** $s$ = `sample_scale`. $H_{c,k}$ = census households in county $c$, size class $k$. $P_{c,x,a}$ = census persons in private households by county, sex, age band; $Q_{c,x,a}$ the same for persons not in private households. $\lfloor\cdot\rceil$ = round half away from zero. "Apportion $N$ over $T$" = split the integer $N$ in proportion to the integer targets $T$ by largest remainder, ties to the lower index (the rule of `split_largest_remainder`).

### Stage A — seed (rule-based, replaceable by the IPUMS draw)
1. **How many records.** Per county: $n^H_c = \max(1, \lfloor \sum_k H_{c,k} / s \rceil)$ private households, $n^Q_c = \lfloor \sum_{x,a} Q_{c,x,a} / s \rceil$ collective records (at least 1 if the county has any such person). Apportion $n^H_c$ over $H_{c,\cdot}$ to get households per size class.
2. **Household sizes.** Classes 1 to 5 have their size. Each household in class 6–10 starts at 6 and each in 11+ at 11. Let $n^P_c = \lfloor \sum_{x,a} P_{c,x,a} / s \rceil$ be the county's target number of synthetic persons. While the county has fewer members than $n^P_c$, add one member to the open-class household with the fewest members that is still below its cap (10, or `max_household_size`), lowest id first. If the open classes cannot absorb the difference, the county simply has more or fewer synthetic persons than $n^P_c$; Stage B corrects the weighted totals.
3. **Who the persons are.** Apportion the county's member count over $P_{c,\cdot,\cdot}$ to get synthetic persons per (sex, age band); the same for collective records over $Q_{c,\cdot,\cdot}$.
4. **Assembly.** Every synthetic person has a sequence number (creation order: county, private before collective, sex, age band) and one keyed draw context `draw(PopulationGen, tick 0, entity = sequence number)`. Its first `u64` is the person's priority. Within a county, persons are dealt into households in ascending (priority, sequence number), in three passes:
   1. one **head** per household, from persons aged 20 or more (15–19 only if the county runs out);
   2. **children** (under 15) go to households of size ≥ 2 whose head is aged 20–59, filling free places evenly;
   3. everyone left fills the remaining places, household by household. A person under 20 whose head is at least 18 years older is a **child**; the first person aged 20 or more added to a household whose head is of the other sex and within 15 years of age becomes **partner**; others are **other relative**.
5. **Exact age.** Band start plus `below(months in band)` from the same draw context; the 100+ band is 100–104.
6. Every record starts with weight $s$.

### Stage B — fit (kept when the seed changes)
7. **Person targets.** A county cell (sex, age band) that has people but no synthetic record cannot be fitted. So the country's total for each (sex, age band) is shared among the county cells that do have records, and the table is balanced both ways with integers (50 alternating passes: rows to the national totals, columns to each county's own person total), then each county's column is apportioned to its exact total. A band with no record anywhere in the country is first added to the nearest band of the same sex that has one.
8. **Integer raking.** Weights are held in millionths of a household. Constraints per county, in this order: each household size class (target $H_{c,k}$, a household counts 1), then each (sex, age band) cell (target from step 7, a household counts its members in the cell). One sweep visits every constraint once: with current weighted total $\hat T$ and target $T$, every household that counts toward the constraint has its weight multiplied by $T/\hat T$ (exact integer multiply-divide, rounded half away from zero). Stop when every constraint with $T \ge$ `min_cell_records` $\cdot s$ is within `raking_tolerance`, or after `max_sweeps`.
9. **Integer weights.** Per county: round weights down to whole households, then give the missing units to the largest remainders (lowest id on ties), so that $\sum_h w_h = \sum_k H_{c,k}$ exactly. A household whose weight would be 0 is given 1, taken from the largest weight in the county.
10. **Person total.** Per county, while $\sum_h w_h \cdot size_h \ne \sum_{x,a} P_{c,x,a}$: move one unit of weight from one private household to another of a different size, in the direction that reduces the gap and by as large a size difference as fits. The donor is the household (with weight ≥ 2) that rounding favoured most, the receiver the one it favoured least; ties by id. Both county totals are then exact.
11. **Collective records.** Each cell's persons are split evenly over the cell's records; persons of cells without a record go to the county's other collective records in proportion to their weights. The county total is exact.

After step 11, in every county and at every `sample_scale`, weighted households and weighted persons equal the census exactly; cells inside the county match within the tolerances below.

**State hash** (for golden and differential tests): FNV-1a 64 over, in order, the household count (`u32`), each household's weight (`u32`), county index and collective flag (one byte each), the person count (`u32`), then each person's household id (`u32`), age in months (`u16`), sex (F = 0, M = 1) and role (head 0, partner 1, child 2, other 3); integers little-endian.

## Player levers
None. `sample_scale` and `rng_seed` are scenario parameters, set before the game starts.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `sample_scale` | 100 | ≥ 1 | Real people per synthetic person. Read from the scenario |
| `max_household_size` | 15 | 11–30 | Cap for the 11+ class in the rule-based seed |
| `raking_tolerance` | 1e-4 | 1e-6–1e-2 | Relative error at which raking stops early |
| `max_sweeps` | 50 | 10–2000 | Upper bound on raking sweeps (more than 50 does not improve the fit today; see open questions) |
| `min_cell_records` | 30 | 5–200 | A cell is checked for convergence and tolerance only if its target is at least this many synthetic records |

## Interactions
- **Before:** the pipeline's fetch and normalise stages produce the margin tables.
- **After:** imputation of education, activity, income and wealth (later increments); the scenario assembler; [demographics](../economy/demographics.md) and [labour market](../economy/labor-market.md) read the result.
- **Invariants it must hand over** ([population-groups](population-groups.md)): every person in exactly one household; integer weights ≥ 1; no negative ages.

## Edge cases & failure modes
- **Inconsistent margins** (a county's private persons cannot fit its households even with every open-class household at its minimum or cap): rejected with a named error; nothing is produced.
- **`sample_scale` larger than a county's households:** the county still gets one household; its weight carries the whole county.
- **Cells too small to be represented** (fewer than one synthetic record): no record exists, so the cell's weighted total is 0 and its people are carried by the same sex and age band in other counties (step 7). Such cells are counted in the fit report, not treated as failures.
- **Zero targets:** apportionment never creates a record in a zero cell, so raking never divides by zero. With a future observed seed, records in a zero-target cell get weight 0 and are dropped.
- **Raking does not converge** within `max_sweeps`: not an error by itself; the fit report says so and the acceptance tolerances decide.
- **`sample_scale` = 0, empty tables, negative counts:** rejected.

## Acceptance tests
IDs are stable. Tests live in `crates/econ-popgen/tests/acceptance/` (protected; written in a test-authoring session). "The fixture" is the normalised Romanian margin file committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10.

- [ ] **AC-POP-01** `[unit]` At each of the three scales the number of synthetic persons is within 0.5% of census persons ÷ `sample_scale`, and no table is sized by a constant.
- [ ] **AC-POP-02** `[unit]` At each of the three scales, in every county: Σ `hh_weight` over private households equals the census household count exactly; Σ `hh_weight` × members equals census persons in private households exactly; Σ weights of collective records equals census persons not in private households exactly. The national totals are therefore the same at every scale.
- [ ] **AC-POP-03** `[unit]` At each of the three scales the weighted margins are within the tolerances of the table below.
- [ ] **AC-POP-04** `[unit]` Structure: every person belongs to exactly one household; every weight is an integer ≥ 1; every private household's size is in the range of a size class; every private household has exactly one head, aged at least 15; no child under 15 lives in a household without a member aged 20 or more; collective records have exactly one member; ages are within their band.
- [ ] **AC-POP-05** `[unit]` Same margins, scale and seed give identical tables (equal state hash). A different seed changes who lives with whom but none of the totals in AC-POP-02.
- [ ] **AC-POP-06** `[golden]` The state hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POP-07** `[diff]` On the fixture at 1:1000 and 1:200 the Rust generator and the independent Python reference produce identical households and persons, record for record.
- [ ] **AC-POP-08** `[unit]` Inconsistent margins, `sample_scale` = 0, an empty table or a negative count are rejected with a named error and produce no output.

**Tolerances for AC-POP-03.** Relative error of the weighted total against the census, by how many synthetic records stand behind the cell (census count ÷ `sample_scale`). The same rule applies at every scale and to every margin: national sex, age band and household size class; county × sex; county × broad age (0–14, 15–64, 65+); county × size class; county × sex × age band.

| Expected synthetic records behind the cell | Tolerance | Worst measured (reference, real margins) |
|---|---|---|
| 1,000 or more | 1% | 0.7% |
| 100 to 999 | 4% | 2.9% |
| 30 to 99 | 5% | 2.6% |
| fewer than 30 | not checked | — |

Measured on 2026-10-10 with the Python reference on the Romanian margins:

| Scale | Synthetic households | Synthetic persons | Weighted households | Weighted persons |
|---|---|---|---|---|
| 1:1000 | 7,831 | 19,057 | 7,709,139 | 19,053,815 |
| 1:100 | 78,281 | 190,540 | 7,709,139 | 19,053,815 |
| 1:10 | 782,798 | 1,905,388 | 7,709,139 | 19,053,815 |

## API sketch
For the test writer and the implementer (Spike 9 lesson). Names may change before lock; shapes should not.

```rust
// crate econ-popgen (depends on econ-types, econ-rng, econ-num; no I/O)
pub struct Margins {
    pub counties: Vec<String>,               // NUTS 3 codes, in table order
    pub age_bands: Vec<(u16, u16)>,          // [from, to) in years; last band open
    pub size_classes: Vec<(u8, u8)>,         // [min, max] members; last class open
    pub persons_private: Vec<u64>,           // [county][sex][age_band], row-major
    pub persons_collective: Vec<u64>,        // same shape
    pub households: Vec<u64>,                // [county][size_class]
}
pub struct GenParams { pub sample_scale: u32, pub rng_seed: u64, pub max_household_size: u8,  // defaults: 15,
                       pub raking_tolerance_ppm: u32, pub max_sweeps: u32, pub min_cell_records: u32 }  // 100, 50, 30
pub struct Households { pub hh_weight: Vec<u32>, pub hh_county: Vec<u8>, pub hh_collective: Vec<bool> }
pub struct Persons { pub household_id: Vec<u32>, pub age: Vec<u16>, pub sex: Vec<Sex>, pub role: Vec<Role> }
pub struct FitReport { pub sweeps: u32, pub converged: bool, pub max_error_ppm: u32, pub unfitted_cells: u32 }
pub struct Population { pub households: Households, pub persons: Persons, pub report: FitReport }
pub enum GenError { InconsistentMargins { county: String }, ZeroScale, EmptyTable, NegativeCount }

pub fn generate(margins: &Margins, params: &GenParams) -> Result<Population, GenError>;
impl Population { pub fn state_hash(&self) -> u64; }
```

- Command line: `econ-cli synth-population --margins FILE --sample-scale N --seed S --out DIR` (the pipeline stage; writes Arrow tables through `econ-io`).
- Margin file: JSON with the fields of `Margins` plus a `provenance` block.
- Python reference: `python/reference/popgen_reference.py` (standard library only, exists). It has its own ChaCha8 to reproduce the keyed draws, checked against the known-answer vectors in `econ-rng`. Where this text and the reference disagree on a detail of ordering or tie-breaking, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/census2021_margins_ro.json`, built by `python/pipeline/normalise_census.py` from the two Eurostat tables.

## Open questions
- [ ] **Tolerances** in the table above: they come from the reference's measured errors with some headroom. Confirm or change before lock.
- [ ] **Raking stops at `max_sweeps` instead of converging.** The open-ended size classes (6–10, 11+) are given integer sizes in the seed, so the household targets and the person targets of a county contradict each other slightly, and step 10 then moves weight to make the person total exact. The fit is inside the tolerances, but a seed whose open-class sizes straddle the county's real average would let raking converge and tighten them. Worth doing before lock?
- [ ] **Household composition is crude.** Heads are drawn at random among adults, so one-person households are not older than average as they are in reality. Fixed by the IPUMS seed, or earlier by using the household-composition table (`cens_21hhcs_r3`).
- [ ] **Where the generator lives.** Proposed: Rust (`econ-popgen`, under the determinism contract, with golden hashes) called by the pipeline as its `synth_population` stage, with the Python reference for the differential test. The alternative is a Python-only stage; then there would be no golden hashes under the determinism contract.
- [ ] Is an **integer** raking scheme acceptable instead of textbook floating-point IPF? It is what makes the Rust and Python results identical bit for bit.
- [x] `hh_collective`: institutional residents are kept, as flagged one-person records (owner decision, 2026-10-10).
- [ ] Next increment after version 1: locality size (urban/rural) and household tenure at county level, or education and activity at region level?
- [ ] Oversampling of minorities ([population-groups](population-groups.md)) needs ethnicity tables from INS; not in version 1.
