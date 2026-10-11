---
id: society/population-wealth
title: Population Generator, Stage H — Household Wealth
status: draft
owner: horia
depends_on: [society/population-housing, society/population-wages, society/population-pensions, society/population-generator, society/population-groups, economy/accounting, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/household-wealth-romania]
updated: 2026-10-11
---

# Population Generator, Stage H — Household Wealth

## Purpose
Give every private household of the starting population its money (`hh_cash`, `hh_deposits`), the value of its home if it owns it (`hh_dwelling_value`) and its debts (`hh_mortgage`, `hh_consumer_debt`), so that the household sector holds the totals of the financial accounts. It runs after Stages E, F and G, once, when a scenario is built. [Households](../economy/households.md) spend out of this wealth, [money and banking](../economy/money-banking.md) starts with these deposits and loans on the banks' books, and [housing](../economy/housing.md) starts from these values.

## Real-world basis
From the [research note](../../01-research/notes/household-wealth-romania.md):
- **Totals, end of 2021 (Eurostat financial accounts, confidence high):** households held 57.3 billion lei in currency and 294.2 billion in deposits, and owed 187.3 billion in loans. Their dwellings were worth 428.6 billion lei at replacement cost, without land (capital stock accounts).
- **No source gives the distribution of wealth in Romania.** The country is not in the European household finance survey.
- **Hungary 2020** (Eurostat experimental statistics): the richest fifth of households by net wealth holds 71% of it, the poorest fifth under 1%; by income, the top fifth holds 42% and the bottom fifth 10%. Wealth is concentrated, and only loosely tied to income, because most of it is the family home.

> **Simplification (the owner's decision of 2026-10-11):** every rule below about *who* holds the totals is an educated guess, not a measurement. The aim is households whose wealth interacts plausibly with the rest of the simulation, not a copy of 2021. The guesses are data in the margin file, listed there with their reasons, so each can be replaced when a source turns up. Confidence: low, except where said.

The guesses:

| Guess | Value | Reason |
|---|---|---|
| Housing loans as a share of household loans | 58% | banks' books split roughly three to two |
| Market value of owned dwellings over their replacement value | 2.7 | about 6.7 million owned homes at an average near 170,000 lei |
| Mean dwelling value by locality class, smallest to largest | 0.45, 0.60, 0.85, 1.20, 1.60, 2.30 | a village house against a flat in a large city |
| Money: share of the top tenth of households | about two thirds | deposits are far more concentrated than homes; the bottom half holds about 3% |
| Homes: cheapest tenth and dearest tenth against the mean | under 0.45, over 1.9 | |
| Households with a housing loan | 6% | owners, head aged 25 to 59, more often with higher income and in towns |
| Households with a consumer loan | 25% | head aged 20 to 64, loosely tied to income |
| How closely a household's place follows its income | money 0.6, home 0.3 | so that net wealth by income fifth comes out near Hungary's |

## Scope
**This increment:** the five columns above.

**Not in this increment:** government bonds, shares and stakes in firms, investment funds, pension fund claims and life insurance (together 400 billion lei, most of it stakes in firms, which arrive with the firm table and its ownership); student debt; land and second homes; what collective records hold.

The household's income here is the wages and pensions of its members. Income of the self-employed does not exist yet ([informal economy](../economy/informal-economy.md)), so their households are placed by the draw; the stage is re-measured when that income exists.

## Inputs
- The population of Stages A and B, the housing columns of Stage E, the wages of Stage F and the pensions of Stage G. **None is changed.** Without wages or pensions every income counts as zero.
- A margin file written by the pipeline's normalise stage, with provenance and the list of guesses:

| Entry | Meaning |
|---|---|
| `totals` | `cash`, `deposits`, `housing_loans`, `consumer_loans`, `dwellings`, in lei |
| `rel_locality[locality_class]` | mean dwelling value of the class, as proportions, in millionths |
| `liquid_curve`, `dwelling_curve`, `loan_curve` | value at a rank over the mean, in millionths: lines through knots, as in Stage F |
| `mortgage`, `consumer` | `share` of private households, `age_from`, `age_to` of the head, `income_link`, `urban_bonus` (millionths) |
| `income_link` | `liquid`, `dwelling` (millionths) |
| `pattern` | Hungary's shares of net wealth by wealth fifth and by income fifth: a check, not an input of the rule |

- `rng_seed`.

## Outputs
Five columns aligned with the households table, and their hash.

| Column | Values |
|---|---|
| `hh_cash`, `hh_deposits` | bani; above zero together for every private household |
| `hh_dwelling_value` | bani; 0 unless the household owns its home |
| `hh_mortgage` | bani; 0 unless the household has a housing loan (owners only) |
| `hh_consumer_debt` | bani; 0 unless the household has a consumer loan |

Records that are not private households have zero in every column.

## Update rule
There is no per-tick rule. The stage is a pure function of its inputs and `rng_seed`, in integer arithmetic. $U$ = 1,000,000. "Spread" is the three steps of [Stage F](population-wages.md) (a place on a curve from the rank, proportions, level): here always with the curve used in full, one group, no split by sex and a floor of zero.

**Draws.** `draw(PopulationGen, tick 7, entity = index in the households table)`; its first four `u64`, in this order: dwelling, money, housing loan, consumer loan. Each becomes a number of millionths: the top 20 bits times $U$, shifted right by 20.

**Income** of a household: the sum of `wage` and `pension` of its members.

**Mixed order** of a set of households, with a link $k$ (millionths): each household's rank by (income, index) is the midpoint of its own weight in that order, in millionths; its key is $k \cdot \text{rank} + (U - k) \cdot \text{draw}$, plus a bonus if one is given. The set is taken in ascending (key, index). With $k$ = 0 the order is the draw's; with $k$ = $U$ it is income's.

### Step 1 — dwellings
The owner households (`hh_tenure` owner), in mixed order with `income_link.dwelling` and the dwelling draw. **Spread** `totals.dwellings` over them with the dwelling curve and one table of proportions: `rel_locality` by the household's `hh_locality_size`.

### Step 2 — money
All private households, in mixed order with `income_link.liquid` and the money draw. **Spread** `totals.cash + totals.deposits` over them with the liquid curve. Of a household's amount $L$, cash is $\lfloor L \cdot \text{cash} / (\text{cash} + \text{deposits}) \rfloor$ and deposits the rest.

> **Simplification:** every household holds cash and deposits in the national proportion. In reality poor households hold mostly cash.

### Step 3 — housing loans
Among owner households whose head is aged `age_from` to under `age_to`: mixed order with the rule's `income_link` and the housing-loan draw, with the bonus `urban_bonus` × $U$ for urban households. From the highest key down, households borrow until their weight reaches `share` of the weight of all private households. The borrowers, in ascending (dwelling value, index), **spread** `totals.housing_loans` with the loan curve: the largest balances go with the most valuable homes.

### Step 4 — consumer loans
The same among all private households whose head is in the rule's age range, without regard to tenure. The borrowers, from the lowest key to the highest, **spread** `totals.consumer_loans` with the loan curve.

> **Simplification:** a loan is not capped by the home's value or by income. In the reference no household owes more on its housing loan than its home is worth, and about 2% of households have negative net wealth (a consumer loan larger than their money, no home).

**Wealth hash** (for golden and differential tests): FNV-1a 64 over the household count (`u32`, little-endian), then for each household `hh_cash`, `hh_deposits`, `hh_dwelling_value`, `hh_mortgage`, `hh_consumer_debt` (`u64`, little-endian, in that order).

## Player levers
None.

## Tuning parameters
All in the margin file (see "The guesses"), plus:

| Parameter | Default | Effect |
|---|---|---|
| Scaling passes | 20 | as in Stage F; with one table the proportions settle in the first pass |

## Interactions
- **Before:** Stages A, B, E, F, G; the pipeline's fetch and normalise stages.
- **After:** [accounting](../economy/accounting.md): the household column of the opening balance sheet is the sum of these columns, and the banks' deposits and household loans must equal them; [households](../economy/households.md) (spending out of wealth, debt service); [money and banking](../economy/money-banking.md); [housing](../economy/housing.md); [taxation](../economy/taxation.md) (property and wealth taxes).
- The totals here and the totals in the balance sheets of the other sectors (roadmap M1) come from the same financial accounts and must be the same figures.

## Edge cases & failure modes
- **No private household:** every column is zero.
- **No owner household, or nobody of borrowing age:** that total is not placed; the columns stay zero. The balance-sheet builder will then refuse the scenario, since the banks' loans have no counterpart.
- **Fewer eligible households than the share asks for:** all of them borrow.
- **A total of zero:** its column is zero.
- **A proportion of zero, a curve that does not ascend from rank 0 to 1, a share above 1, an empty age range, a locality class the margins do not have, housing, wage or pension columns of another population:** rejected with a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/wealth.rs` (protected; to be written in a test-authoring session, before the Rust code), compiled only with the crate feature `wealth`. "The fixtures" are the normalised margin files committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42.

- [ ] **AC-POPH2-01** `[unit]` Structure, at each of the three scales: five values per household; a record that is not a private household has five zeros; every private household has money; `hh_dwelling_value` is above zero exactly for owners; only owners have `hh_mortgage`.
- [ ] **AC-POPH2-02** `[unit]` The stage changes nothing before it: the state, housing, wage and pension hashes are those of their own golden tests.
- [ ] **AC-POPH2-03** `[unit]` At each of the three scales each of the five weighted totals equals its target in the margin file within 0.001%.
- [ ] **AC-POPH2-04** `[unit]` At each of the three scales: the share of private households with a housing loan is within half a point of the rule's share, likewise for consumer loans; every borrower's head is in the rule's age range; the mean dwelling value rises from each locality class to the next, and the largest class's mean over the smallest's is within 0.3 of the ratio of their proportions.
- [ ] **AC-POPH2-05** `[unit]` The shape, at each of the three scales: the top tenth of households by money holds between 60% and 75% of it; the richest fifth by net wealth (money plus home less loans) holds more than 45% and the poorest fifth less than 5%; net wealth by income fifth is within 6 points of the pattern in each fifth; fewer than 6% of households have negative net wealth.
- [ ] **AC-POPH2-06** `[unit]` Same inputs and seed give identical columns (equal wealth hash). A different seed changes who holds what; AC-POPH2-03 still holds. Without wages and pensions the stage runs and AC-POPH2-03 holds.
- [ ] **AC-POPH2-07** `[golden]` The wealth hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPH2-08** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns, household for household.
- [ ] **AC-POPH2-09** `[unit]` Margins with no locality classes, a curve that does not end at rank 1, a proportion of zero, a share above 1, housing columns of a different length and wages of a different length are each rejected with a named error and produce no output.

Measured on 2026-10-11 with the Python reference on the fixtures (seed 42, all earlier stages run):

| | Pattern (Hungary) | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|---|
| Largest error of a total | | 0.00005% | 0.00006% | 0.00004% |
| Owner households | | 90.9% | 90.9% | 90.9% |
| Money per household, mean and median, lei | | 45,600 and 10,600 | 45,600 and 10,600 | 45,600 and 10,600 |
| Dwelling value per owner, lei | | 165,100 | 165,200 | 165,200 |
| Money held by the top tenth | | 67.4% | 67.4% | 67.4% |
| Net wealth by wealth fifth, % | 0.8, 4.3, 8.8, 15.7, 70.6 | 2.2, 7.9, 13.4, 22.3, 54.2 | 2.2, 8.0, 13.3, 22.0, 54.5 | 2.2, 8.0, 13.3, 22.0, 54.5 |
| Net wealth by income fifth, % | 10.4, 11.9, 15.2, 20.5, 42.0 | 11.1, 13.6, 16.4, 21.0, 37.9 | 10.6, 13.9, 16.7, 21.4, 37.4 | 10.4, 14.1, 16.6, 21.6, 37.3 |
| Households with negative net wealth | | 2.3% | 2.4% | 2.4% |

Net wealth is less concentrated here than in the pattern because this increment leaves out stakes in firms and shares (282 billion lei), which the richest hold; with them the richest fifth would be near 62%.

Hashes of the reference, for the test writer to record as golden values (the earlier hashes are unchanged):

| Scale | Wealth hash |
|---|---|
| 1:1000 | `8b7d9a0ea0d3c373` |
| 1:200 | `fa255fb46e229ff7` |
| 1:100 | `76dc19b2cbad16b5` |
| 1:10 | `52463287abf2c3a5` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. The skeleton in `crates/econ-popgen/src/wealth.rs` is the definition.

```rust
// crate econ-popgen, module `wealth` (no I/O), re-exported at the crate root
pub struct Curve { pub rank: Vec<u32>, pub value: Vec<u32> }                 // millionths
pub struct BorrowerRule { pub share_ppm: u32, pub age_from: u16, pub age_to: u16, pub income_link_ppm: u32, pub urban_bonus_ppm: u32 }
pub struct WealthMargins {
    pub cash: u64, pub deposits: u64, pub housing_loans: u64, pub consumer_loans: u64, pub dwellings: u64,   // lei
    pub rel_locality: Vec<u32>,
    pub liquid_curve: Curve, pub dwelling_curve: Curve, pub loan_curve: Curve,
    pub mortgage: BorrowerRule, pub consumer: BorrowerRule,
    pub liquid_income_link_ppm: u32, pub dwelling_income_link_ppm: u32,
}
pub struct WealthParams { pub rng_seed: u64, pub scaling_passes: u32 }       // default 20
pub struct HouseholdWealth { pub hh_cash: Vec<u64>, pub hh_deposits: Vec<u64>, pub hh_dwelling_value: Vec<u64>,
                             pub hh_mortgage: Vec<u64>, pub hh_consumer_debt: Vec<u64> }   // bani
pub enum WealthError { EmptyTable, ShapeMismatch }

pub fn assign_wealth(pop: &Population, housing: &HouseholdHousing, wages: Option<&PersonWages>,
                     pensions: Option<&PersonPensions>, margins: &WealthMargins, params: &WealthParams)
    -> Result<HouseholdWealth, WealthError>;
impl HouseholdWealth { pub fn state_hash(&self) -> u64; }
```

- "Urban" is `hh_urban` of Stage E.
- Command line: `econ-cli synth-population … --housing FILE --wealth FILE` (with `--wages` and `--pensions` if those ran) adds the five columns to `households.arrow` and the wealth hash to `fit_report.json`.
- Margin file: JSON with the entries of Inputs and `provenance` (sources, and `guesses` with every guessed value).
- Python reference: `python/reference/popgen_reference.py`, functions `assign_wealth`, `mixed_order`, `wealth_hash` and `wealth_fit` (exists; `--wealth FILE` on its command line, with `--housing FILE`, adds the columns and `summary.wealth_hash`). Where this text and the reference disagree on a detail of ordering or rounding, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/household_wealth2021_ro.json`, built by `python/pipeline/normalise_wealth.py`.

## Open questions
- [x] **Borrow Hungary's pattern and build on educated guesses:** accepted by the owner, 2026-10-11.
- [ ] **Bands of AC-POPH2-04 and AC-POPH2-05:** to be accepted by the owner.
- [ ] **Stakes in firms, shares, bonds and pension fund claims** (400 billion lei): after the firm table, since a stake needs a firm unit to be a stake in.
- [ ] **The market value of homes** (2.7 times replacement value) decides property-tax revenue and how rich owners feel. It is the guess to replace first, with prices by county, when [housing](../economy/housing.md) is built.
- [ ] Re-measure, and perhaps re-tune the two income links, when the income of the self-employed exists.
