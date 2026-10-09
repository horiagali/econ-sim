---
id: adr/0003-people-representation
title: "ADR-0003: Representing people — weighted synthetic population"
status: accepted
owner: horia
depends_on: [research/people-model-approaches, society/population-groups, research/population-modelling-deep-research, research/tech-stack-deep-research]
updated: 2026-10-09
---

# ADR-0003: Representing people — weighted synthetic population

## Context
Modelling people is the most important and most expensive part of the game. Requirements: fine-grained attributes (region and county, urban or rural, age, sex, education, occupation, industry, income, wealth, ethnicity, language, religion and religiosity, ideology, overlapping interest groups), households with families, exact progressive taxes and benefits, ~19M people (Romania), determinism, stock-flow consistency, explainability, decades simulated in minutes. See the [research note](../../01-research/notes/people-model-approaches.md).

## Options considered
1. **Cartesian pop cells** (Victoria 3 style). Deterministic and smooth, but the cell count explodes (millions) with the requested attributes, and overlapping groups can't be represented.
2. **Cells for economic dimensions + identity as within-cell distributions.** Bounded cost, but correlations between identity and economics are lost or approximated, and it adds complexity.
3. **Weighted synthetic population (microsimulation)** of ~75k households / ~190k persons at 1 : 100, every attribute a column, groups as filters, alignment for transitions.
4. **Every citizen as an agent.** Most faithful, far too slow.

## Decision
**Accepted: option 3**, with:
- **Sample scale as a parameter.** `sample_scale` defaults to 1 : 100 for development and play. It is configurable and **never hardcoded** (see "Population scale rules" below).
- Households as the economic unit (budget, balance sheet); persons as members (age, job, education, identity, opinion).
- Keyed random numbers and alignment for all transitions → deterministic and exact on aggregates ([ADR-0006](0006-determinism-contract.md)).
- Column (struct-of-arrays) storage for vectorised updates ([ADR-0005](0005-simulation-core-architecture.md)).
- Oversampling of small minorities with lower weights.
- The population is built from the IPUMS-International 2011 Romania 10% sample, reweighted to 2021 census totals (see "Generation pipeline" below).

## Research update (2026-10-08)
The [deep research](../../01-research/notes/population-modelling-deep-research.md) supports option 3 at 1:100 (the Bank of Canada's CANVAS model uses the same scale) and finds population is **not** the biggest technical risk: calibration/stability, the explainability ledger and SFC verification rank higher. Additions it recommends for this ADR: scale as a parameter (1:100 play, 1:10 validation), logit-scaling alignment with carried rounding error, an explicit ledger rule for splitting household wealth, random numbers keyed on (seed, tick, entity, purpose), integer money, saves as snapshot + seed + command log. It also flags one aggregate firm per industry as a bigger modelling gap than population scale.

## Amendment (2026-10-09): accepted, scale rules, generation pipeline
The owner accepted this ADR on 2026-10-09 and confirmed that **1 : 100 is granular enough** and is the development default. The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) adds the scale rules below. It had also replaced the IPUMS-based generation pipeline because IPUMS-International's terms forbid commercial use; the owner then confirmed the same day that this is a **personal, non-commercial project**, so the IPUMS seed is restored ([ADR-0012](0012-data-pipeline-and-licensing.md)).

### Population scale rules
- **Scale lives in one place.** `sample_scale` is set in the pipeline parameters and the scenario manifest. It is copied into every save manifest, history block and export header, so history recorded at one scale is never silently mixed with another ([ADR-0011](0011-storage-saves-history.md)).
- **Nothing hardcodes it.** No population-sized constants. No array or table is sized by population; capacities come from the loaded scenario, whose size follows from the data and `sample_scale`.
- **Every record carries an integer weight** (`hh_weight` and the person equivalent): the number of real units it represents. Code never multiplies by a global constant. Integer weights keep scaled-up money exact and allow oversampling without code changes.
- **Weighted postings.** Postings between a weighted record and a sector-level counterparty (bank, government, named firm) are multiplied by the integer weight at posting time, so ledger invariants hold exactly at any scale ([ADR-0007](0007-money-and-ledger.md)).
- **Scale-independent stores stay scale-independent.** The aggregation cube and the aggregated ledger have no per-household dimension.
- **CI runs at more than one scale.** The tiny scenario is built and run at three scales (for example 1:1000, 1:200 and 1:100); every invariant must pass and per-capita aggregates must agree within sampling error ([ADR-0010](0010-verification-and-testing.md)). This replaces the earlier "1 : 500 for tests and CI".
- **Small counts.** Demographic and labour transitions use logit-scaling alignment, with rounding error carried into the next period (as LIAM2's `errors="carry"`), so a county's 0.3 expected births a month are not rounded away.
- The group explorer always shows how many records sit behind a view.
- Expected cost: about 4–12 ms per tick at 1:100 and 40–120 ms at 1:10; 1:1 needs 10–20 GB of RAM. Scales finer than 1:10 need a memory review.

### Generation pipeline (IPUMS 2011 seed)
The base is the IPUMS-International 2011 Romania 10% census sample, used under its free non-commercial registration. The raw microdata are never published; they stay in a private DVC remote or a gitignored local folder ([ADR-0012](0012-data-pipeline-and-licensing.md)).

1. **Stratified draw** of whole households from the IPUMS sample (by county, urban/rural and household type), at the configured `sample_scale`, with oversampling of small minorities.
2. **Reweight** the drawn households with iterative proportional fitting to 2021 census county totals (county × age × sex × education × activity; INS and the Eurostat Census Hub), then integerise the weights.
3. **Impute** income, wealth and opinions from published survey tables (EU-SILC, HBS/ABF, LFS, opinion surveys).
4. **Validate** against published aggregates (SILC deciles, poverty rates by household type, county employment).
5. Write a versioned scenario with provenance tags for every input.

The seed source stays a **swappable stage**: a seed built from public aggregates only remains possible (for example if the project ever becomes commercial) without other changes.

## Consequences
- Easier: adding attributes, overlapping interest groups, exact taxes and benefits, arbitrary group views in the UI.
- Easier: changing scale is a parameter change, proven safe by the multi-scale CI test.
- Easier: the IPUMS seed gives observed joint distributions of household composition, education, occupation and housing.
- Harder: income, wealth and opinions are imputed from published tables, and the 2011 seed predates the 2021 targets; the validation suite has to show the population behaves believably.
- Harder: noise in tiny groups (UI must show sample size); probabilistic transitions need alignment.
- The tech stack must handle ~10⁸ simple operations per tick quickly; see [ADR-0004](0004-tech-stack-overview.md).
- Revisit if profiling shows the per-tick cost is too high, if noise in small groups bothers players, or if the project ever becomes commercial (then the IPUMS seed must be replaced; see [ADR-0012](0012-data-pipeline-and-licensing.md)).

## Open questions
- [ ] Register for IPUMS-International access and confirm the Romania 2011 sample's variables (county, household type, education, occupation, dwelling).
- [ ] Which 2021 census cross-tabs exist at county level.
- [ ] The ledger rule for splitting household wealth when households split or merge.
- [x] The [population-groups spec](../../02-design/society/population-groups.md) now uses integer weights and multi-scale CI.
