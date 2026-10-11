---
id: society/population-incomes
title: Population Generator, Stage I — Declared and Undeclared Income
status: draft
owner: horia
depends_on: [society/population-wages, society/population-jobs, society/population-housing, society/population-attributes, society/population-generator, economy/informal-economy, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/informal-economy-romania]
updated: 2026-10-11
---

# Population Generator, Stage I — Declared and Undeclared Income

## Purpose
Finish the incomes of the starting population and draw the line between what is declared and what is not:
- split every wage into a declared part (`wage_declared`) and the rest;
- give employers, own-account and family workers their income (`self_employment_income`) and its declared part;
- give some people income from undeclared activity outside a job (`informal_income`);
- give rural households the value of what they grow for themselves (`hh_subsistence_income`).

It runs after Stages D, E and F, once, when a scenario is built. It is the starting state of the [informal economy](../economy/informal-economy.md): taxes are computed on the declared parts, and the difference between total and declared is the map the player sees.

## Real-world basis
From the [research note](../../01-research/notes/informal-economy-romania.md):
- **Measured (confidence high):** households' mixed income was 46.1 billion lei in the fourth quarter of 2021, 15.4 billion a month. Agriculture's value added less its wages was 44.7 billion lei in 2021.
- **Anchors (confidence medium):** undeclared work was 21.7% of private-sector labour input and 27.1% of private-sector value added in 2019; 36.7% of VAT due was not collected in 2021.
- National accounts already include the undeclared economy: the wage bill of [Stage F](population-wages.md) and mixed income are totals. This stage adds no money; it splits.

> **Simplification (the owner's decision of 2026-10-11):** no source gives, for Romania, who is paid outside a contract, how mixed income divides between farming, declared business and informal activity, or who earns the informal part. Every such rule below is an educated guess, held as data in the margin file with its reason. The aim is an informal economy of the right size, in the right places, that reacts plausibly; not a copy of 2021. Confidence: low.

The guesses:

| Guess | Value |
|---|---|
| Households' part of agriculture's surplus | 80% (so farming is 22% of mixed income) |
| Of farm income, grown and eaten at home | 60% |
| Rural households that grow food for themselves | 60%; those with a farm worker first, then those with a retired member |
| Income of non-farm self-employed, share of mixed income | 22% (about 1.2 times the mean wage each) |
| Declared part of self-employment income | 60% outside farming, 20% in farming |
| An employer against an own-account worker against a family worker | 2 : 1 : 0.5 |
| Undeclared share of the wage bill: agriculture, industry, construction, trade-transport-hospitality, information, finance, real estate, professional and administrative, public sector, other services | 40%, 8%, 25%, 18%, 3%, 2%, 15%, 8%, 0%, 20% |
| Of undeclared wages, earned with no contract at all | 35%; the rest in envelopes |
| An envelope job declares | the minimum wage, or half the true wage if that is more |
| People aged 15 to 74 with informal income outside a job | 14% |
| Who they are, relative propensities | unemployed 0.6; own-account 0.5; family worker 0.4; employer and other inactive 0.3; retired 0.2; student 0.15; employees by industry group from 0.30 (agriculture) to 0.01 (public sector) |

## Scope
**This increment:** the five columns above. `formality` of a job follows from them: formal if `wage_declared` equals `wage`, informal if it is zero, partly declared otherwise.

**Not in this increment:** undeclared sales of firms and the VAT gap (they are properties of firm units and arrive with the firm table); property income (interest, dividends, rents); benefits other than pensions (computed from the law in force).

Family workers are unpaid by definition; the income given to them here is their share of the family business's income.

## Inputs
- The population of Stages A and B, the attributes of Stage C, the jobs of Stage D, the housing columns of Stage E and the wages of Stage F. **None is changed.**
- A margin file written by the pipeline's normalise stage, with provenance and the list of guesses:

| Entry | Meaning |
|---|---|
| `mixed_income`, `farm_income`, `farm_in_kind`, `self_employment_income`, `informal_income` | lei per month; the last three parts of the first sum to it |
| `minimum_wage`, `quantile_curve` | from the earnings file of Stage F |
| `undeclared_wage_share[industry_group]`, `fully_informal_share`, `envelope_declared_share`, `low_wage_link` | the wage split (millionths) |
| `rel_kind[3]`, `self_employment_education_link`, `self_employment_declared`, `farm_declared` | the self-employed |
| `subsistence_household_share`, `subsistence_bonus`, `subsistence_dispersion` | farming for own use |
| `informal_recipient_share`, `informal_propensity` | informal income outside a job |
| `anchors` | the measured sizes of the undeclared economy: a check, not an input of the rule |

- `rng_seed`.

## Outputs
Four columns aligned with the persons table, one with the households table, and their hash. All in bani per month.

| Column | Values |
|---|---|
| `wage_declared` | between 0 and `wage`; if strictly between, at least the minimum wage |
| `self_employment_income` | above zero exactly for employers, own-account and family workers |
| `self_employment_declared` | at most `self_employment_income` |
| `informal_income` | above zero only for persons aged 15 to 74 |
| `hh_subsistence_income` | above zero only for rural private households |

## Update rule
There is no per-tick rule. The stage is a pure function of its inputs and `rng_seed`, in integer arithmetic. $U$ = 1,000,000. "Spread" is the three steps of [Stage F](population-wages.md), with no split by sex and a floor of zero. "Mixed order" is that of [Stage H](population-wealth.md). A person's weight is the weight of their household.

**Draws.** For persons, `draw(PopulationGen, tick 8, entity = index in the persons table)`, its first four `u64` in this order: wage split, self-employment place, informal pick, informal place. For households, `draw(PopulationGen, tick 9, entity = index in the households table)`, its first two: subsistence pick, subsistence place. Each becomes millionths as in Stage H.

**Take from the top** of an ordered set, up to a weight: members from the highest key down, until their weight reaches the target.

### Step A — the declared part of wages
For each industry group, with $B$ the wage bill of its employees (Σ weight × `wage`) and the target $T = \lfloor B \cdot \text{undeclared\_wage\_share} / U \rfloor$. If $T$ is zero every wage of the group is declared.
1. **No contract.** Employees in mixed order of (`wage`, wage-split draw) with `low_wage_link`: low wages first. From the lowest key up, `wage_declared` = 0, until the wages so far reach $\lfloor T \cdot \text{fully\_informal\_share} / U \rfloor$.
2. **Envelopes.** The remaining employees in ascending (wage-split draw, index). For each, until the undeclared wages so far reach $T$: `wage_declared` = max(minimum wage, $\lfloor \text{wage} \cdot \text{envelope\_declared\_share} / U \rfloor$); if that is not below the wage, the job has no contract instead (`wage_declared` = 0).

The group overshoots $T$ by at most one person.

### Steps B and C — the self-employed
Twice: for the employers, own-account and family workers outside agriculture with `self_employment_income` as the amount and `self_employment_declared` as the declared share; and for those in agriculture with `farm_income − farm_in_kind` and `farm_declared`. Members in mixed order of (`edu_level`, self-employment draw) with `self_employment_education_link`. **Spread** the amount with the earnings curve and one table of proportions, `rel_kind` by status in employment. The declared part is $\lfloor \text{amount} \cdot \text{share} / U \rfloor$.

### Step D — farming for own use
Among rural private households (`hh_urban` false): key = subsistence pick draw, plus `subsistence_bonus.farm_worker` if a member works in agriculture, else plus `subsistence_bonus.retired` if a member is retired. **Take from the top** up to `subsistence_household_share` of the rural households' weight. Those, in ascending (subsistence place draw, index), **spread** `farm_in_kind` with the earnings curve compressed by `subsistence_dispersion`.

### Step E — informal income outside a job
Among persons aged 15 to 74: propensity by what the person does (an employee's by industry group; else by status in employment; else by activity; children none); key = propensity × informal pick draw. **Take from the top** up to `informal_recipient_share` of that age group's weight, leaving out anyone whose key is zero. Those, in ascending (informal place draw, index), **spread** `informal_income` with the earnings curve.

> **Simplification:** informal income does not depend on county or locality, and a recipient's amount is unrelated to their education or other income.

**Income hash** (for golden and differential tests): FNV-1a 64 over the person count (`u32`, little-endian), then for each person `wage_declared`, `self_employment_income`, `self_employment_declared`, `informal_income` (`u64`, little-endian, in that order), then the household count (`u32`) and each household's `hh_subsistence_income` (`u64`).

## Player levers
None. The tools against the informal economy are in [informal-economy](../economy/informal-economy.md); they move these columns after the start.

## Tuning parameters
All in the margin file (see "The guesses"), plus the scaling passes of the spread (20).

## Interactions
- **Before:** Stages A to F; the pipeline's fetch and normalise stages.
- **After:** [informal economy](../economy/informal-economy.md) (the starting state of `formality` and of every undeclared amount); [taxation](../economy/taxation.md) and [social transfers](../economy/social-transfers.md) (bases are the declared parts; pension rights build on declared wages); [households](../economy/households.md) (all income, and food not bought); the [wealth stage](population-wealth.md), which is re-measured with these incomes; the firm table (each unit's `undeclared_wage_share` is what its employees' wages say).
- The model's undeclared share of private labour income comes out near 36%, above the measured 21.7% of labour *input*: the national accounts book most informal production as household income, and an hour of it is counted once in labour input however much it earns. Undeclared income is about 175 billion lei a year, against 27.1% of private value added (about 245 billion): the rest is undeclared profit of firms, which arrives with the firm table.

## Edge cases & failure modes
- **An undeclared share of zero** (the public sector): every wage of the group is declared.
- **A group where every wage is at or below the minimum wage:** nobody can have an envelope; undeclared jobs there have no contract.
- **No rural household, nobody self-employed, nobody eligible for informal income:** that amount is not placed. The scenario builder will then report that households receive less than the mixed income of the national accounts.
- **Fewer people with a propensity above zero than the recipient share asks for:** all of them receive.
- **A table whose length does not match its dimensions, a share above 1, a proportion of zero, farm income in kind above farm income, a curve that does not ascend from rank 0 to 1, columns of another population:** rejected with a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/incomes.rs` (protected; to be written in a test-authoring session, before the Rust code), compiled only with the crate feature `incomes`. "The fixtures" are the normalised margin files committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42.

- [ ] **AC-POPI-01** `[unit]` Structure, at each of the three scales: the bounds of the Outputs table hold for every person and household; nobody who is not an employee has a declared wage.
- [ ] **AC-POPI-02** `[unit]` The stage changes nothing before it: the earlier hashes are those of their own golden tests.
- [ ] **AC-POPI-03** `[unit]` At each of the three scales, weighted self-employment income plus informal income plus subsistence income equals `mixed_income` within 0.001%.
- [ ] **AC-POPI-04** `[unit]` At each of the three scales the undeclared share of each industry group's wage bill is within 3 points of its target, and exactly zero where the target is zero.
- [ ] **AC-POPI-05** `[unit]` The shape, at each of the three scales: more than 65% of employees are fully declared; more are partly declared than have no contract; the share of persons aged 15 to 74 with informal income is within half a point of its target, and of rural households growing food within one point; the undeclared share of private labour income is between 20% and 45%.
- [ ] **AC-POPI-06** `[unit]` Same inputs and seed give identical columns (equal income hash). A different seed changes who gets what; AC-POPI-03 still holds. With every undeclared wage share at zero, `wage_declared` equals `wage` for everyone.
- [ ] **AC-POPI-07** `[golden]` The income hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPI-08** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns.
- [ ] **AC-POPI-09** `[unit]` Margins with no industry groups, a table of the wrong length, a share above 1 and wages of a different length are each rejected with a named error and produce no output.

Measured on 2026-10-11 with the Python reference on the fixtures (seed 42):

| | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|
| Error of the mixed-income total | 0.0001% | 0.0001% | 0.0001% |
| Employees fully declared, partly declared, with no contract | 76.9%, 14.3%, 8.8% | 77.1%, 14.3%, 8.7% | 77.0%, 14.3%, 8.7% |
| Undeclared share of all wages | 9.8% | 9.7% | 9.7% |
| Largest gap to a group's undeclared wage share, points | 2.0 | 0.3 | 0.0 |
| Mean income of a self-employed person, lei | 4,149 | 4,150 | 4,154 |
| Mean informal income of a recipient, lei | 4,206 | 4,206 | 4,207 |
| Mean subsistence income of a growing household, lei | 925 | 930 | 924 |
| Undeclared share of private labour income | 35.7% | 35.7% | 35.7% |
| Undeclared income, billion lei a year | 175.5 | 175.3 | 175.2 |

The largest gap at 1:1000 is in real estate (19 synthetic employees: one person is five points of the wage bill).

Hashes of the reference, for the test writer to record as golden values (the earlier hashes are unchanged):

| Scale | Income hash |
|---|---|
| 1:1000 | `40c861af66b29b98` |
| 1:200 | `ea9941848b82147d` |
| 1:100 | `9b350177389121b3` |
| 1:10 | `da77f8b39da6b61d` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. The skeleton in `crates/econ-popgen/src/incomes.rs` is the definition: `IncomeMargins` (the entries of Inputs, with `Curve` of Stage H and `InformalPropensity`), `IncomeParams { rng_seed, scaling_passes }`, `OtherIncomes` (the five columns, `u64` bani), `IncomeError { EmptyTable, ShapeMismatch }` and

```rust
pub fn assign_other_incomes(pop: &Population, attrs: &PersonAttributes, jobs: &PersonJobs, housing: &HouseholdHousing,
                            wages: &PersonWages, margins: &IncomeMargins, params: &IncomeParams)
    -> Result<OtherIncomes, IncomeError>;
impl OtherIncomes { pub fn state_hash(&self) -> u64; }
```

- Command line: `econ-cli synth-population … --wages FILE --housing FILE --incomes FILE` adds the four columns to `persons.arrow`, `hh_subsistence_income` to `households.arrow` and the income hash to `fit_report.json`.
- Python reference: `python/reference/popgen_reference.py`, functions `assign_other_incomes`, `take_from_top`, `incomes_hash` and `income_fit` (exists; `--incomes FILE` on its command line, with `--wages FILE --housing FILE`). Where this text and the reference disagree on a detail of ordering or rounding, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/other_incomes2021_ro.json`, built by `python/pipeline/normalise_incomes.py`.

## Open questions
- [x] **Build on educated guesses:** accepted by the owner, 2026-10-11.
- [ ] **Bands of AC-POPI-04 and AC-POPI-05:** to be accepted by the owner.
- [ ] **The size of informal income** (8.5 billion lei a month, more than half of mixed income, to 14% of adults at about 4,200 lei each). It is what the national accounts leave after farming and declared business. If it looks too rich for the people who get it, the lever is the recipient share: at 25% the mean would be about 2,400 lei.
- [ ] **Almost a quarter of employees are not fully declared** (14% envelopes, 9% no contract). Surveys in which people admit to envelope wages give lower figures; the labour-input estimate implies more. Keep?
- [ ] Re-measure the wealth stage with these incomes, and decide whether informal income should count when households are placed by income.
