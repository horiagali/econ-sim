---
id: research/people-model-approaches
title: How to model people at fine grain — approaches and compromises
status: draft
owner: horia
depends_on: [research/overview]
updated: 2026-10-08
---

# How to model people at fine grain — approaches and compromises

## Question
The owner wants people modelled in fine detail: region, age, sex, education, job, income, wealth, ethnicity/culture, language, religion, ideology, and overlapping interest groups (environmentalists, anti-immigration, car lovers, …), for a Romania-sized country (~19 million people). What representation gives that richness at acceptable compute cost, while keeping the simulation deterministic, stock-flow consistent and explainable?

## Short answer
- **Cartesian "pop cells" (Victoria 3 style) don't scale to this many attributes.** Each attribute multiplies the cell count. With the requested attributes it reaches millions of cells, and overlapping interest groups can't be cells at all.
- **Individual simulation of all 19M people is too heavy** for a game that should run decades in minutes.
- **Recommended: a weighted synthetic population (microsimulation).** About 75k synthetic households with ~190k persons, each standing for ~100 real people. Every person carries *all* attributes as columns, so adding an attribute costs one column instead of multiplying everything. This is how real tax-benefit and pension models work (EUROMOD, dynamic microsimulation).
- **"Groups" become queries, not storage.** "Well-educated students in Cluj" is a filter over persons; its size is the sum of weights. Interest groups are per-person membership intensities (Democracy-style), so they overlap freely.
- **Main costs:** sampling noise in very small groups, and calibration work (building the synthetic population from census and survey data). Both have standard mitigations: alignment, a configurable sample scale, and raking to census marginals.

## Findings

### 1. Cell / pop approach (Victoria 3, our previous draft)
- Victoria 3 splits pops by profession, culture, religion and location, and assigns them to interest groups by profession and other traits. *(medium: from game documentation and wikis; verify)*
- Pros: deterministic, smooth aggregates, simple SFC bookkeeping, cheap when dimensions are few.
- Cons: combinatorial explosion. Example with the requested attributes: 42 counties × 2 urban/rural × 8 age × 2 sex × 6 education × 12 occupation/activity × 4 ethnicity × 6 religion × 5 income class ≈ **11.6 million** potential cells. Most are empty or tiny; the non-empty ones still number in the hundreds of thousands, and many hold fractional people. Overlapping memberships (one person is both a farmer and a car lover and religious) need either more dimensions or within-cell distributions that lose the correlations.
- Victoria 3 has had widely reported late-game performance problems tied to pop counts. *(low–medium; verify)*

### 2. Voter-sample approach (Democracy series)
- Democracy simulates a sample of individual voters, each with a membership strength in many overlapping groups (motorists, environmentalists, religious, retired…). Policies affect groups; voter happiness is a membership-weighted sum. *(medium: developer blog posts; verify)*
- Pros: overlapping groups for free; very readable "this policy affects these groups".
- Cons: the economy is not driven by those voters' budgets. The voters are opinion carriers, not economic agents.

### 3. Weighted microsimulation (policy models)
- **Static microsimulation** (e.g. EUROMOD, the EU tax-benefit model) runs tax and benefit rules on survey households (EU-SILC), each with a weight, to compute exact distributional effects. *(high)*
- **Dynamic microsimulation** (e.g. pension models, the LIAM2 framework) ages a sample over time with transition probabilities: births, deaths, education, employment, retirement, migration. It uses **alignment** to make simulated totals match target rates exactly while choosing *which* individuals transition by their individual probabilities. *(high)*
- Pros: unlimited attributes with realistic correlations (taken from real data), exact tax and benefit rules per person, natural households (families, dependants, pooled income), groups as arbitrary filters.
- Cons: per-tick cost is linear in sample size; small-group statistics are noisy; transitions are probabilistic, which needs a seeded RNG for determinism.

### 4. Distribution grids (heterogeneous-agent macro, HANK)
- Households represented as a distribution over a grid of states (income × wealth), evolved with transition matrices. *(high)*
- Pros: smooth, deterministic, economically rigorous.
- Cons: only practical for 2–3 continuous states; doesn't carry identity attributes.

### 5. Full agent-based (every citizen)
- Some city builders simulate every citizen (e.g. Workers & Resources). *(medium)* That is feasible for thousands, not for 19 million with rich monthly behaviour.

## Compute estimate for the recommended option
| Item | Estimate |
|---|---|
| Real population | ~19 million (verify: INS 2021 census / latest estimate) |
| Sample scale | 1 : 100 (configurable 1 : 50 … 1 : 500) |
| Synthetic persons | ~190k (~75k households) |
| Attributes | ~40 numeric (incomes, balances, health, approval…) + ~40 categorical (1 byte) + ~25 interest-group intensities |
| Memory | ≈ 190k × (40 × 8 + 40 + 25 × 4) B ≈ **85 MB** |
| Per-tick work | a few hundred operations per person ≈ 10⁸ ops ≈ **well under 1 second** in compiled, vectorised (struct-of-arrays) code |

So a monthly tick costs well under a second, and a simulated decade takes about a minute or less at max speed. A coarser scale (1 : 500) suits fast testing.

## Noise and its mitigations
- A group of 5,000 real people at 1 : 100 is ~50 synthetic persons: shares are noisy (±7 points for a 50% rate).
- **Alignment:** for each transition, compute the expected count per stratum from rates, then select exactly that many persons ranked by individual probability (+ seeded noise). National and regional totals become exact; individual paths stay realistic.
- **Show sample size** in the group explorer and grey out estimates from too few synthetic persons.
- **Larger samples where it matters:** oversample small minorities (e.g. ethnic minorities, rare occupations) with lower weights, a standard survey technique.
- Determinism: seeded RNG per subsystem; common random numbers allow clean what-if comparisons.

## Implications for the game
- [population-groups](../../02-design/society/population-groups.md) is rewritten around a synthetic population of weighted households and persons. "Groups" become views.
- New [identity](../../02-design/society/identity.md) (ethnicity, language, religion, religiosity) and [interest-groups](../../02-design/society/interest-groups.md) specs.
- [Taxation](../../02-design/economy/taxation.md) becomes exact per person; no within-cell distribution approximation needed.
- [Households](../../02-design/economy/households.md) is the per-household budget model.
- Building the starting population is a **data pipeline**: synthetic population from the Romanian census, household budget and income surveys via iterative proportional fitting (raking). This is a P0/P2 research and data task.
- Decision recorded in [ADR-0003](../../03-architecture/decisions/0003-people-representation.md).

## Sources
- EUROMOD — EU tax-benefit microsimulation model (JRC). https://euromod-web.jrc.ec.europa.eu/
- LIAM2 — dynamic microsimulation framework (alignment). https://liam2.plan.be/
- Li & O'Donoghue (2013), "A survey of dynamic microsimulation models", *International Journal of Microsimulation*. https://microsimulation.pub/
- Kaplan, Moll & Violante (2018), "Monetary Policy According to HANK", *AER*.
- Victoria 3 wiki (pops, interest groups). https://vic3.paradoxwikis.com/
- Positech Games developer blog (Democracy voter simulation). https://www.positech.co.uk/cliffsblog/
