---
id: society/population-employers
title: Population Generator, Stage J — Industry and Employer
status: draft
owner: horia
depends_on: [society/population-jobs, society/population-wages, society/population-generator, economy/industries, adr/0016-firm-representation, adr/0006-determinism-contract, adr/0014-agent-workflow-guardrails]
research: []
updated: 2026-10-11
---

# Population Generator, Stage J — Industry and Employer

## Purpose
Give every employed person of the starting population a catalogue industry (`industry`, one of 90) and an employer (`employer`, one of the firm units), inside the industry group that [Stage D](population-jobs.md) gave them. It runs after Stages D and F, once, when a scenario is built. From it the [labour market](../economy/labor-market.md) knows who works where, each firm unit knows its workers and its wage bill, and the wage clearing accounts of [accounting](../economy/accounting.md) have two sides.

## Real-world basis
- The firm units ([industries](../economy/industries.md), "The firm units"): 349 units over 90 industries, with persons employed and wage bill by size class from the structural business statistics of 2021 and the input-output table.
- Larger firms pay more. In the firm units, the wage bill per person employed rises from micro to small to large in most industries, and public units pay above the private average.
- Almost all self-employed people run or work in very small firms.

> **Simplification:** inside an industry group, which industry a person works in does not depend on their occupation, education or county. A welder and an accountant of the group "industry" are equally likely to be in car parts or in food. Occupation by industry exists in the Labour Force Survey at a coarser level than the catalogue; it can sharpen this later.

> **Simplification:** a unit's share of the group's employees is its share of persons employed in the business statistics. For agriculture and for public units that figure is itself a guess (see the firm units).

## Scope
**This increment:** `industry` and `employer` for every person whose `employment_status` is not "not employed".

**Not in this increment:** the place of work (county; a unit's capacity is spread over regions by a vector, [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)), `employer_type` (it follows from the unit's ownership), tenure, hours.

## Inputs
- The population of Stages A and B, the jobs of Stage D and the wages of Stage F. **None is changed.** Without wages, employees are placed by the draw alone.
- The firm-unit file: `industries` (id and industry group of each, in a fixed order) and `units` (industry, size class, controller, persons employed, wage bill).
- `rng_seed`.

## Outputs
Two columns aligned with the persons table, and their hash.

| Column | Values |
|---|---|
| `industry` | index into the file's `industries`; 255 for a person who is not employed |
| `employer` | index into the file's `units`; 65535 for a person who is not employed |

## Update rule
There is no per-tick rule. The stage is a pure function of its inputs and `rng_seed`, in integer arithmetic. "Split" is that of [Stage C](population-attributes.md), "deal" that of [Stage D](population-jobs.md), "mixed order" that of [Stage H](population-wealth.md). A person's weight is the weight of their household.

**Draw.** `draw(PopulationGen, tick 10, entity = index in the persons table)`; its first `u64` is the priority, and in millionths (as in Stage H) the draw of the mixed order.

For each industry group, with the units of the group's industries:

### Step 1 — employees
1. Order the group's units from the lowest to the highest pay per person: $\lfloor \text{wages} \cdot U / \max(\text{persons employed}, 1) \rfloor$, ties by the unit's index.
2. Take the group's employees in mixed order of (`wage`, draw) with `employer_wage_link`: low wages first.
3. **Split** them over the units in that order, with the units' persons employed as targets and no carry (all ones if every target is zero).

A unit gets its share of the group's employees, and the group's best-paid employees are in its best-paying units.

### Step 2 — employers, own-account and family workers
Take them in ascending (priority, index) and **deal** them over the group's domestic micro units (if it has none: its cohort units; if none: all its units), with the units' persons employed as targets.

A person's `industry` is the industry of their unit.

**Employer hash** (for golden and differential tests): FNV-1a 64 over the person count (`u32`, little-endian), then each person's `industry` (one byte) and `employer` (`u16`, little-endian).

## Player levers
None.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `employer_wage_link` | 0.5 | 0–1 | 0: who works where is unrelated to pay. 1: strictly the lowest-paid in the lowest-paying units |

## Interactions
- **Before:** Stages A to D and F; the firm units and the input-output table of the pipeline.
- **After:** [labour market](../economy/labor-market.md), [production](../economy/production.md) (each unit's workers by occupation), [accounting](../economy/accounting.md) (wage clearing per industry), the [informal economy](../economy/informal-economy.md) (a unit's undeclared wage share is what its employees' declared wages say), the scenario builder.
- The wages of Stage F were set by industry group, not by unit. A unit's wage bill in the starting population (its employees' wages) therefore differs from its wage bill in the firm-unit file; the group totals agree with the national accounts. The scenario builder takes the population's side as the truth and rescales the units' wage bills to it.

## Edge cases & failure modes
- **An industry group with workers and no unit:** rejected with a named error.
- **A unit with no persons employed in the file:** it gets nobody, unless every unit of the group is like that.
- **A unit too small for one synthetic person** (most units at 1:1000): it may get nobody; the split's rounding decides. At 1:100, 342 of the 349 units have employees.
- **No industries, no units, a unit of an unlisted industry, more than 255 industries or 65,535 units, columns of another population:** rejected with a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/employers.rs` (protected; to be written in a test-authoring session, before the Rust code), compiled only with the crate feature `employers`. "The fixtures" are the margin files and the firm-unit file committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42.

- [ ] **AC-POPE-01** `[unit]` Structure, at each of the three scales: a person has an industry and an employer exactly when employed; the employer's industry is the person's industry; that industry's group is the person's `industry_group`; employers, own-account and family workers are in micro units.
- [ ] **AC-POPE-02** `[unit]` The stage changes nothing before it: the earlier hashes are those of their own golden tests.
- [ ] **AC-POPE-03** `[unit]` At each of the three scales, in every industry group, each unit's share of the group's employees is within 3 points of its share of the group's persons employed in the firm-unit file.
- [ ] **AC-POPE-04** `[unit]` At each of the three scales the mean wage of employees rises from micro to small to large units; with `employer_wage_link` = 0 the three means are within 10% of each other.
- [ ] **AC-POPE-05** `[unit]` Same inputs and seed give identical columns (equal employer hash). A different seed changes who works where; AC-POPE-03 still holds. Without wages the stage runs and AC-POPE-03 holds.
- [ ] **AC-POPE-06** `[golden]` The employer hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPE-07** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns.
- [ ] **AC-POPE-08** `[unit]` No industries, no units, a unit of an unlisted industry, jobs of a different length, and a worker in an industry group without a unit are each rejected with a named error and produce no output.

Measured on 2026-10-11 with the Python reference on the fixtures (seed 42):

| | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|
| Largest gap of a unit's share in its group, points | 1.3 | 0.2 | 0.03 |
| Units with employees, of 349 | 299 | 342 | 348 |
| Industries with workers, of 90 | 89 | 90 | 90 |
| Mean wage in micro, small, large, public units, lei | 3,817; 4,547; 6,215; 8,650 | 3,797; 4,562; 6,225; 8,662 | 3,806; 4,574; 6,219; 8,658 |
| Employees in micro, small, large, public units, million | 1.76; 1.10; 2.62; 1.06 | 1.77; 1.10; 2.62; 1.06 | 1.77; 1.10; 2.61; 1.06 |

Hashes of the reference, for the test writer to record as golden values (the earlier hashes are unchanged):

| Scale | Employer hash |
|---|---|
| 1:1000 | `40dc87a87a0e31af` |
| 1:200 | `54fe641b38dca16e` |
| 1:100 | `37541a9150c60900` |
| 1:10 | `07df05437989c962` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. The skeleton in `crates/econ-popgen/src/employers.rs` is the definition.

```rust
// crate econ-popgen, module `employers` (no I/O), re-exported at the crate root
pub const NO_INDUSTRY: u8 = 255;
pub const NO_EMPLOYER: u16 = 65535;
pub enum UnitSize { Micro = 0, Small = 1, Large = 2, Public = 3 }
pub struct FirmUnit { pub industry: u8, pub size: UnitSize, pub domestic: bool, pub persons_employed: u64, pub wages: u64 }
pub struct FirmUnits { pub industry_group: Vec<u8>, pub units: Vec<FirmUnit> }
pub struct EmployerParams { pub rng_seed: u64, pub wage_link_ppm: u32 }                 // default 500_000
pub struct PersonEmployers { pub industry: Vec<u8>, pub employer: Vec<u16> }
pub enum EmployerError { EmptyTable, ShapeMismatch, NoMargin { industry_group: u8 } }

pub fn assign_employers(pop: &Population, jobs: &PersonJobs, wages: Option<&PersonWages>, firms: &FirmUnits, params: &EmployerParams)
    -> Result<PersonEmployers, EmployerError>;
impl PersonEmployers { pub fn state_hash(&self) -> u64; }
```

- `domestic` is true for a cohort unit controlled from Romania (the units step 2 deals over).
- Command line: `econ-cli synth-population … --jobs FILE --wages FILE --firms FILE` adds the two columns to `persons.arrow` and the employer hash to `fit_report.json`.
- Python reference: `python/reference/popgen_reference.py`, functions `assign_employers`, `employers_hash` and `employer_fit` (exists; `--firms FILE` on its command line, with `--attributes FILE --jobs FILE`). Where this text and the reference disagree on a detail of ordering or rounding, the reference is the definition until the spec is locked.
- Firm-unit fixture: `python/pipeline/fixtures/firm_units2021_ro.json`, built by `python/pipeline/normalise_firms.py`.

## Open questions
- [ ] **Bands of AC-POPE-03 and AC-POPE-04:** to be accepted by the owner.
- [ ] **Industry inside the group ignores occupation and county** (first simplification). Good enough for the start, or use the Labour Force Survey's occupation-by-activity table now?
- [ ] The scenario builder rescales each unit's wage bill to its employees' wages (Interactions). The alternative is to adjust wages so that units pay what the business statistics say; that would undo part of Stage F's fit.
