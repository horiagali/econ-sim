---
id: economy/firm-units-undeclared-inventories
title: Firm Units — Undeclared Sales and Opening Inventories
status: draft
owner: horia
depends_on: [adr/0016-firm-representation, economy/industries, economy/informal-economy, economy/production, society/population-incomes, society/population-employers, adr/0006-determinism-contract, adr/0014-agent-workflow-guardrails]
research: [research/firm-inventories-undeclared-sales-romania, research/informal-economy-romania]
updated: 2026-10-14
---

# Firm Units — Undeclared Sales and Opening Inventories

## Purpose
Give every firm unit of the starting economy ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md), [industries](industries.md) "The firm units") the two attributes it still lacks:
1. **Undeclared sales:** how much of its sales it does not report for VAT and profit tax (`undeclared_sales_share`), with the undeclared wages and undeclared value added behind them. It is the firm side of the [informal economy](informal-economy.md) and must agree with the undeclared wages and incomes of [Stage I](../society/population-incomes.md).
2. **Opening inventories:** the stock of its own output it can sell from (`inventory`, the $V_f$ of [production](production.md)) and the materials and goods for resale it holds (`inventory_materials`), at the end of 2021.

It runs once, when a scenario is built, after the firm-unit file and the incomes margins exist. There is no per-tick rule here; the informal economy and production move these numbers afterwards.

## Real-world basis
From the [research note](../../01-research/notes/firm-inventories-undeclared-sales-romania.md) and the [informal economy note](../../01-research/notes/informal-economy-romania.md):
- **Measured (high):** inventories at the end of 2021 were 231.9 billion lei for non-financial corporations and 141.8 billion lei for households and their businesses (Eurostat `nama_10_nfa_bs`).
- **Anchors (medium):** undeclared work was 27.1% of private-sector value added (2019); the VAT gap was 36.7% (2021), of which hidden sales are only a part.
- **From Stage I (its margins, so the two agree):** the undeclared share of each industry group's wage bill; households' undeclared income from farm sales, self-employment and informal activity (131.9 billion lei a year).
- **Not measured anywhere found:** inventories by industry or firm size; undeclared sales by industry or firm size.

> **Simplification (the owner's rule of 2026-10-11, educated guesses):** how the totals are spread over units is guessed, listed in the margin file with a reason and a confidence. The totals and the anchor are hit exactly.

The guesses:

| Guess | Value | Confidence |
|---|---|---|
| Days of a year's output held as inventories, by table industry | agriculture 120; manufacturing 25 to 120 (cars 35, machinery 90, ships and aircraft 120); trade 120 to 150 (of its margin output); construction 60; real estate 30; energy 15; transport, finance, public services 0 to 5 | low |
| Share of inventories that is the unit's own output (finished goods and work in progress) | about half in manufacturing, 0.75 in agriculture, 0.8 to 0.9 in construction and real estate, 0.05 in trade (goods for resale), 0 to 0.4 in services | low |
| Share of operating surplus earned on sales | 1, except real estate 0.2 (the rest is imputed rent of owner-occupiers) | medium |
| Tilt of undeclared wages by class (micro, small, large, foreign) | 1.15, 0.95, 0.9, 0.8: fitted to the synthetic population, see Interactions | medium |
| Tilt of informal income by class | 1, 0.5, 0.2, 0 | low |
| Tilt of undeclared profit by class | 1.5, 1.5, 0.5, 0.15 | low |
| Undeclared profit weight of an industry group | its undeclared wage share plus 0.02 | low |
| Most a unit can hide | 90% of the value added it can hide | low |

## Scope
**This increment:** seven columns per firm unit (Outputs). **Not in it:** named firms (M9: they will take their rows from the cohorts); the inventories of government (33.1 billion lei; they stay with the government, not its public units) and of financial corporations (0.5); the part of farm output eaten at home (`hh_subsistence_income`; it is not a sale); detection and change over time ([informal economy](informal-economy.md)).

## Inputs
- **The firm-unit file** (`fixtures/firm_units2021_ro.json`): `industries` (id, industry group) and `units` (industry, kind, size, controller, output, value added, wages, in lei a year). Not changed.
- **A margin file** written by the pipeline (`fixtures/firm_undeclared_inventories2021_ro.json`, `normalise_firm_undeclared_inventories.py`), with provenance:

| Entry | Meaning |
|---|---|
| `industries[]` | per catalogue industry, in the firm file's order: `inventory_days`, `finished_share`, `household_purchases` (lei a year; a check only) |
| `undeclared_wage_share[g]` | Stage I's, per industry group (millionths) |
| `household_business_propensity[g]` | Stage I's `informal_propensity.employee_by_industry_group` |
| `undeclared_farm_income`, `undeclared_self_employment_income`, `informal_income` | lei a year, from Stage I's margins: 12 × (farm income − in kind) × (1 − `farm_declared`); 12 × self-employment income × (1 − `self_employment_declared`); 12 × informal income |
| `undeclared_value_added_share` | the anchor, 271,000 |
| `surplus_from_sales[g]`, `wage_tilt[4]`, `informal_tilt[4]`, `profit_tilt[4]`, `profit_group_floor`, `max_undeclared_share` | the guesses (millionths) |
| `inventories.corporations`, `inventories.household_business` | lei, measured |
| `anchors` | the measured anchors: a check, not an input of the rule |

The file also carries every unit's seven columns as the Python reference computes them, their totals and hash: a frozen copy for readers of the scenario, not an input.

## Outputs
Seven columns aligned with the firm file's `units`, and their hash.

| Column | Unit | Values |
|---|---|---|
| `undeclared_wages` | lei a year | 0 to `wages` |
| `undeclared_value_added` | lei a year | `undeclared_wages` plus the household part plus undeclared profit; at most `max_undeclared_share` of hideable value added |
| `undeclared_sales` | lei a year | at most `output` |
| `undeclared_wage_share` | millionths | ⌊`undeclared_wages` · U / `wages`⌋ (glossary `undeclared_wage_share[f]`) |
| `undeclared_sales_share` | millionths | ⌊`undeclared_sales` · U / `output`⌋ (glossary `undeclared_sales_share[f]`) |
| `inventory` | lei (= real units at starting prices) | the unit's own output in stock: production's `inventory[f]`, for all its firms together |
| `inventory_materials` | lei | materials, supplies and goods bought for resale |

Public units (`kind` public) have every column zero.

## Update rule
No per-tick rule. A pure function of the two files, in integer arithmetic, with no randomness. $U$ = 1,000,000. Units are taken in the file's order, index $f$. For unit $f$ of industry group $g$:
- `cohort` = kind is cohort; class $c$ = foreign if controller is foreign, else its size (micro, small, large);
- $VA_f$ = max(value added, 0); full surplus $S^0_f$ = max(value added − wages, 0); surplus on sales $S_f = \lfloor S^0_f \cdot \text{surplus\_from\_sales}_g / U \rfloor$;
- hideable $H_f$ = max($VA_f − S^0_f + S_f$, 0); room $R_f = \lfloor H_f \cdot \text{max\_undeclared\_share} / U \rfloor$ for cohort units, 0 otherwise.

**Apportion** is largest remainder on integer weights, ties to the lower index, as in [the firm balance sheets](industries.md) (a total of zero gives zeros; a positive total over weights summing to zero is the error `NoRoom`). **Apportion with caps:** apportion over the members with a positive weight and a positive cap; every member whose part would pass its cap is set to its cap and dropped; apportion what is left over the rest; repeat until nothing passes. If members run out with something left: `NoRoom`. Weights are products of a lei amount and up to two millionths; they need 128-bit integers.

### Step 1 — undeclared wages
For each industry group, over its cohort units: target $\lfloor \sum \text{wages}_f \cdot \text{undeclared\_wage\_share}_g / U \rfloor$, apportioned with caps; weight wages$_f$ × `wage_tilt`$_c$, cap min(wages$_f$, $R_f$).

### Step 2 — undeclared income of households
Three apportions with caps, in this order, each capped at $R_f$ less what the unit already has:
1. `undeclared_farm_income` over the domestic micro units of agriculture (group 0), weight $S_f$;
2. `undeclared_self_employment_income` over the domestic micro units, weight $S_f$ × `household_business_propensity`$_g$;
3. `informal_income` over the domestic cohort units, weight $S_f$ × `informal_tilt`$_c$.

### Step 3 — undeclared profit
Target $T = \lfloor \sum_{\text{cohort}} VA_f \cdot \text{undeclared\_value\_added\_share} / U \rfloor$. The rest $T$ − Steps 1 and 2 (if negative: `AnchorsInconsistent`) is apportioned with caps over the cohort units, weight $S_f$ × (`undeclared_wage_share`$_g$ + `profit_group_floor`) × `profit_tilt`$_c$, cap $R_f$ less what the unit has.

### Step 4 — undeclared sales
`undeclared_value_added` = the sum of the three steps. A unit hides the sales that carry it: `undeclared_sales` = min(output, ⌊`undeclared_value_added` × output / $VA_f$⌋), 0 if $VA_f$ = 0. The two shares as in Outputs (0 when the base is 0).

> **Simplification:** a unit that hides a sale hides the inputs bought for it too, so hidden sales carry the unit's average value added. The VAT lost on a hidden sale is then between the VAT on its value added and on the whole sale.

### Step 5 — inventories
Key $k_f$ = output$_f$ × `inventory_days` of its industry, for cohort units (0 otherwise). Apportion `inventories.corporations` over all units by $k_f$, and `inventories.household_business` over the domestic micro units by $k_f$; the unit's total is the sum. `inventory` = ⌊total × `finished_share` / U⌋; `inventory_materials` = total − `inventory`.

> **Simplification:** a trade unit's stock is goods bought for resale, held as `inventory_materials`; in the goods market households buy from the industry that makes a good, so trade stock is not a buffer there in M2. Seasonality is ignored: the stock of 31 December stands for every month.

**Hash** (golden and differential tests): FNV-1a 64 over the unit count (`u32`, little-endian), then for each unit the seven columns as `u64` little-endian, in the order of the Outputs table.

## Player levers
None here. The tools against undeclared sales are in [informal economy](informal-economy.md).

## Tuning parameters
The guesses above, all in the margin file. Production's `inventory_target_ratio` of a unit should start at `inventory` ÷ a month of output, so that the unit starts at its target (see Interactions).

## Interactions
- **Before:** the firm-unit file, the input-output table and the incomes margins of the pipeline.
- **After:** [production](production.md) (`inventory` is $V_f$ at the start); [informal economy](informal-economy.md) (the starting `undeclared_sales_share` and `undeclared_wage_share`; VAT and profit tax are due on declared sales only); [VAT](vat.md) and [taxation](taxation.md) (bases); the scenario builder (inventories on the firm units' balance sheets, next to the [firm balance sheets](industries.md)).
- **Stage I and J:** the group totals of undeclared wages are Stage I's targets by construction. Which units carry them follows from where Stage J puts low-paid employees. Measured on 2026-10-13 with the Python references (seed 42), the undeclared share of each class's wage bill:

| Class | Firm table (this rule) | Population 1:1000 | Population 1:100 |
|---|---|---|---|
| micro | 18.1% | 17.9% | 18.1% |
| small | 15.1% | 14.2% | 14.6% |
| large | 13.0% | 13.6% | 12.6% |
| foreign-controlled | 8.4% | 8.8% | 8.9% |

- Stage I's undeclared income of households (farm sales, self-employment, informal: 131.9 billion lei a year) sits in domestic units by construction; Step 3 adds 75.8 billion lei of undeclared profit of firms, the "rest" that Stage I's text announces.

Results on the fixtures (2026-10-13):

| | Value |
|---|---|
| Undeclared value added | 250.1 billion lei, 27.1% of private value added (target hit to the leu) |
| of which wages, household income, profit | 42.3, 131.9, 75.8 billion lei |
| Undeclared sales | 552.4 billion lei, 27.3% of private sales |
| Undeclared sales share: micro, small, large, foreign-controlled | 59.1%, 46.6%, 18.8%, 4.6% |
| By industry group: agriculture, industry, construction, trade-transport-hospitality, other services | 72.6%, 13.7%, 54.3%, 32.9%, 46.8% |
| VAT gap from hidden sales (share of households' purchases bought undeclared) | 23.1%, against 36.7% measured |
| Cohort units hiding 90% of their value added or more | 17 of 344 |
| Inventories | 373.6 billion lei (231.9 + 141.8, exact): own output 149.3, materials and goods for resale 224.3 |
| Days of output held: agriculture, industry, construction, trade, real estate, services | 189, 43, 86, 115, 63, 0 to 14 |

## Edge cases & failure modes
- **An industry group with no undeclared wage share** (public services): no undeclared wages; a little undeclared profit through the floor.
- **A unit with no value added, or a loss:** it hides nothing.
- **An industry with zero days** (finance, public administration): no inventories.
- **The anchor below Steps 1 and 2:** `AnchorsInconsistent` (for example an anchor of 15%).
- **More to place than the room:** `NoRoom` (for example `max_undeclared_share` at 50%).
- **Industries of the margin file not those of the firm file, a unit of an unlisted industry, a table of the wrong length, a share above one:** a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in the crate of the API sketch, in `tests/acceptance/` (protected), written in a test-authoring session before the Rust code. "The fixtures" are the committed firm-unit file and margin file.

- [ ] **AC-FUI-01** `[unit]` Structure: for every unit the bounds of the Outputs table hold; public units have every column zero; only domestic units carry household income; `inventory` + `inventory_materials` is zero for units of industries with zero days.
- [ ] **AC-FUI-02** `[unit]` Totals hit exactly: undeclared value added equals the anchor's target; each group's undeclared wages equal ⌊group wage bill × Stage I's share / U⌋; the household part equals the three amounts of the margins; inventories equal `corporations` + `household_business`, and the household part lies in domestic micro units only.
- [ ] **AC-FUI-03** `[unit]` Consistency with Stage I: the undeclared share of each industry group's wage bill equals Stage I's `undeclared_wage_share` within one millionth (rounding), and undeclared value added is at least undeclared wages plus households' undeclared income.
- [ ] **AC-FUI-04** `[unit]` Consistency with the population, at 1:1000 and 1:100, seed 42: for micro, small, large and foreign-controlled units, the undeclared share of the class's wage bill in Stages I and J (employees' `wage` − `wage_declared`, weighted, by their employer's class) is within 2 points of the firm table's.
- [ ] **AC-FUI-05** `[unit]` Anchors and shape on the fixtures: the undeclared share of private value added is 27.1% to the leu; the VAT gap from hidden sales is between 15% and 36.7%; the undeclared sales share falls from micro to small to large to foreign-controlled.
- [ ] **AC-FUI-06** `[unit]` Inventories shape on the fixtures: days of output held are highest in agriculture and trade and below 15 in information, finance, professional services and public services; units of motor, wholesale and retail trade hold less than 10% of their stock as own output.
- [ ] **AC-FUI-07** `[unit]` Purity: the same inputs give the same hash, and the input files are unchanged. With every undeclared share, amount and the anchor at zero, every undeclared column is zero and the inventories are unchanged. An anchor of 15% gives `AnchorsInconsistent`; `max_undeclared_share` of 50% gives `NoRoom`.
- [ ] **AC-FUI-08** `[golden]` The hash of the fixtures equals the committed golden value.
- [ ] **AC-FUI-09** `[diff]` The Rust function and the Python reference produce identical columns on the fixtures and on three changed margin files: the anchor at 20%, every tilt at 1, `max_undeclared_share` at 75%.
- [ ] **AC-FUI-10** `[unit]` Empty units, industries that differ from the firm file's, a unit of an unlisted industry, a tilt table of the wrong length and a share above one are each rejected with a named error and produce no output.

Hashes of the reference, for the test writer to record as golden values:

| Margins | Hash |
|---|---|
| the fixtures | `d0ecc4f61fa5a6c3` |
| anchor at 20% | `8700aab0d0fd555c` |
| every tilt at 1 | `e96cf180693cc915` |
| `max_undeclared_share` at 75% | `325189f2c0e2f679` |

## API sketch
For the test writer and the implementer; names may change before lock, shapes should not. Proposed home: a module `firm_opening` of `econ-popgen` (no I/O), behind a crate feature `firm_opening`, because AC-FUI-04 needs Stages I and J and the crate already reads the firm-unit file. The module extends `FirmUnits` of [Stage J](../society/population-employers.md) with what it needs.

```rust
pub struct FirmUnitAmounts { pub output: u64, pub value_added: i64, pub wages: u64 }          // lei a year, one per unit
pub struct IndustryMargins { pub inventory_days: u32, pub finished_share_ppm: u32, pub household_purchases: u64 }
pub struct FirmOpeningMargins {
    pub industries: Vec<IndustryMargins>,              // in the firm file's industry order
    pub undeclared_wage_share_ppm: Vec<u32>,            // per industry group
    pub household_business_propensity_ppm: Vec<u32>,    // per industry group
    pub surplus_from_sales_ppm: Vec<u32>,               // per industry group
    pub undeclared_farm_income: u64, pub undeclared_self_employment_income: u64, pub informal_income: u64,
    pub undeclared_value_added_share_ppm: u32,
    pub wage_tilt_ppm: [u32; 4], pub informal_tilt_ppm: [u32; 4], pub profit_tilt_ppm: [u32; 4],   // micro, small, large, foreign
    pub profit_group_floor_ppm: u32, pub max_undeclared_share_ppm: u32,
    pub inventories_corporations: u64, pub inventories_household_business: u64,
}
pub struct FirmOpening {                                // one entry per unit
    pub undeclared_wages: Vec<u64>, pub undeclared_value_added: Vec<u64>, pub undeclared_sales: Vec<u64>,
    pub undeclared_wage_share_ppm: Vec<u32>, pub undeclared_sales_share_ppm: Vec<u32>,
    pub inventory: Vec<u64>, pub inventory_materials: Vec<u64>,
}
pub enum FirmOpeningError { EmptyTable, ShapeMismatch, ShareAboveOne, AnchorsInconsistent, NoRoom }
pub fn firm_opening(firms: &FirmUnits, amounts: &[FirmUnitAmounts], m: &FirmOpeningMargins)
    -> Result<FirmOpening, FirmOpeningError>;
impl FirmOpening { pub fn state_hash(&self) -> u64; }
```

- Python reference: `python/reference/firm_reference.py`, functions `undeclared_and_inventories`, `columns_hash`, `fit` and `population_wage_shares`; `python python/reference/firm_reference.py FIRMS MARGINS [--units]` prints the totals, the checks and the hash as JSON. Where this text and the reference disagree on ordering or rounding, the reference is the definition until the spec is locked.
- The interim scenario builder (`build_scenario.py`) runs it and checks the totals, the committed columns and AC-FUI-04's comparison.

## Open questions
- [x] **Decided (owner, 2026-10-14): move part of the informal income onto large domestic units.** **Micro and small firms hide a lot** (59% and 47% of their sales; agriculture 73%, construction 54%). It follows from the anchor (27.1% of private value added) and from Stage I placing 132 billion lei of undeclared household income in domestic units, whose value added is small. Accept, lower the anchor (the 2019 figure measures labour, not sales), or move part of informal income to large domestic units?
- [x] **Band accepted (owner, 2026-10-14).** **The VAT gap from hidden sales is 23%** against 36.7% measured. Is a band of 15% to 36.7% right for AC-FUI-05, the rest being fraud, insolvency and VAT declared but not paid?
- [x] **Decided (owner, 2026-10-14): spread them by farm output only.** **Households' inventories** (141.8 billion lei) put a third of a year's output or more in domestic micro units (agriculture 189 days on average). Spread them by farm output only, or keep them on all household businesses?
- [x] **Kept as the trade units' goods (owner, 2026-10-14).** **Trade's stock:** keep goods for resale as trade units' materials (proposed), or count them as stock of the industries that make the goods, so that the goods-market buffer of [production](production.md) includes goods in shops?
- [x] **Accepted and written into production (owner, 2026-10-14).** Production (locked) starts `inventory` at `inventory_target_ratio` × expected demand with defaults of a month (a week for services). It should start at this spec's `inventory`, with each unit's target ratio set to match (a change to production's "First version, starting state" that needs the owner).
- [ ] Home of the Rust function: `econ-popgen` (proposed) or a scenario-builder crate of ADR-0018.
