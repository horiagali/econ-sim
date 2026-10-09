---
id: research/population-modelling-deep-research
title: "Population modelling — deep research: Architect around calibration, not population size"
status: draft
owner: horia
depends_on: [research/people-model-approaches]
updated: 2026-10-08
---

> Deep-research report (2026-10-08) on how to model Romania's population and whether it is the biggest technical risk. Web search was unavailable; sources were fetched directly. Benchmarks ran on a small sandbox VM and are order-of-magnitude.

# Architect around calibration, not population size

Keep ADR-0003. For a Romania-sized game, the right model is a weighted synthetic population of about **190k persons in about 75k households at 1:100**. The Bank of Canada's CANVAS macro model uses exactly this scale, and the Bank of England's monthly housing model runs at 1:2,700. Compiled code processes one person-month in about **9 nanoseconds**, so a simulated year at 1:100 takes well under a second on a laptop. A 1:1 population of 19M people is possible: the Austrian forecasting model runs about 8M agents on a 16 GB laptop. For a monthly game with rich per-person attributes, though, 1:1 costs roughly 10–20 GB of RAM, 5–15 seconds per simulated year and multi-gigabyte saves. Worse, it makes every calibration sweep about 100x slower. **Population modelling is not the biggest technical risk, and it should not decide the tech stack.** The bigger risks, in order, are:

1. Calibrating the coupled economy and keeping it stable. Research still treats this as unsolved for large models.
2. Building the "why did this change" panel into the accounting and the behaviour rules from day one.
3. Keeping a stock-flow-consistent codebase written by AI agents correct.

The population's real problems are modelling problems: sampling noise in small counties and groups (fixed with alignment), building realistic combinations of attributes from restricted survey data, and explaining changes in distributions. The stack that follows from this is a headless Rust core with columnar tables, money stored as whole bani in 64-bit integers, random numbers derived from (seed, tick, entity), a configurable scale factor, and a ledger of tagged flows checked every tick.

## One person in a hundred is enough, and central banks agree

Serious national models already run at many scales, which shows that scale is a runtime choice, not a credibility choice. The **Bank of England housing model simulates 10,000 households, each standing for about 2,700 real ones, on a one-month time step**. It is calibrated from the English Housing Survey, the Wealth and Assets Survey and loan-level data ([BoE](https://www.bankofengland.co.uk/working-paper/2016/macroprudential-policy-in-an-agent-based-model-of-the-uk-housing-market)). **CANVAS states that "each agent in the model represents 100 individuals or businesses in the Canadian economy"**. Its persons aged 15+ carry employment status, age group, sex and industry ([Bank of Canada](https://www.bankofcanada.ca/wp-content/uploads/2022/12/swp2022-51.pdf)). At the other end, the Austrian model by Poledna et al. runs at 1:1 with **about 8 million agents on a quarterly step**. The same code drops to about 8,000 agents through a single scale parameter, and the Bank of Italy's Julia port (BeforeIT.jl) runs the full-scale version on a Ryzen 5 laptop with 16 GB of RAM ([BeforeIT.jl paper](https://arxiv.org/html/2502.13267v1)). What makes these models credible is that their agents reproduce the real *joint* distribution of income, wealth, debt and housing tenure. The headcount does not matter.

The cost gap between scales is large. The estimates below extrapolate from a person-month kernel benchmarked during this research: employment transition, Romanian-style contributions and income tax in integer bani, consumption, saving, and per-county totals. They assume about 100 attributes per person and multi-phase monthly ticks on a 6–8 core laptop (local benchmark on a 2-vCPU sandbox VM). Market clearing (labour matching, goods markets) was not benchmarked. Treat the numbers as order-of-magnitude.

| Scale | Persons | State in RAM (incl. households, firms) | Multithreaded monthly tick | Simulated year | Compressed save |
|---|---|---|---|---|---|
| 1:100 | 190k | ~100–200 MB | ~4–12 ms | ~0.05–0.6 s | ~20–40 MB |
| 1:10 | 1.9M | ~1–2 GB | ~40–120 ms | ~0.5–1.5 s | ~0.2–0.4 GB |
| 1:1 | 19M | ~10–20 GB | ~0.4–1.3 s (memory-bound) | ~5–15 s | ~2–4 GB |

At 1:1, the person table alone can exceed a typical 16 GB Windows machine, and every full pass is limited by memory bandwidth. At 1:100 there is roughly two orders of magnitude of headroom for richer behaviour.

Commercial games point the same way. **Victoria 3 starts with 20–25k pop objects and ends near 100k**. Its developers found that the cost of the weekly employment update scales with the *number of pop objects*, not with total population. The worst bottleneck was a serial step that creates and destroys pops when people change jobs. Their fixes were to merge small pops more aggressively, process pop growth for only 1/120th of pops per tick, and use a deterministic sort to keep multiplayer in sync ([Victoria 3 Dev Diary #76](https://forum.paradoxplaza.com/forum/threads/victoria-3-dev-diary-76-performance.1570277/)). The proposed 190k records are the same order of magnitude as a late-game Victoria 3 save, but they tick monthly instead of every six in-game hours. A fixed sample also removes the churn that hurt Paradox. Records persist, and only births, deaths and migration add or remove them.

Make the scale a parameter, not a constant. Every aggregate already multiplies by a weight, so the same code can run at 1:100 for play and at 1:10 for validation runs, where the noise in small groups drops by about a factor of three.

## Persons and households as tables, groups as filters, politics as fractions

Paradox and microsimulation handle the combinatorial explosion in opposite ways. **Victoria 3 splits pops on only five keys (culture, religion, state, profession, workplace)**. Wealth, literacy and interest-group support live *inside* each pop as fractions, and "any given pop often has individuals supporting several different interest groups" ([Vic3 wiki: Pops](https://vic3.paradoxwikis.com/Pops); [Vic3 wiki: Interest groups](https://vic3.paradoxwikis.com/Interest_groups)). A microsimulation puts every attribute on every record and limits the record count by sampling instead. ADR-0003's "groups are filters, not objects" follows naturally from this, and it is the right choice for arbitrary filters like "Orthodox pensioners in Iași". Store the data as two columnar tables: persons and households, linked by integer IDs. Store relationships to firms and industries as index columns too, not as pointers.

Interest groups and approval have a good commercial template in the Democracy series. It simulates "thousands of virtual citizens" ([Steam: Democracy 4](https://store.steampowered.com/app/1410710/)). Each citizen has graded membership in many overlapping groups. **Group membership (slow and structural) is modelled separately from happiness (fast and policy-driven)** ([Positech 2013](https://www.positech.co.uk/cliffsblog/2013/05/05/changing-the-voters-opinions-long-term-changes/)). Opinions mix an innate value with a national mood through a one-line formula weighted by each voter's "impressionability" ([Positech 2021](https://www.positech.co.uk/cliffsblog/2021/03/03/improvements-to-the-political-compass-voters-opinions/)). At 190k records you can give every person a small vector of fractional interest-group memberships and compute approval as a weighted sum. You do not need a smaller voter sub-sample.

Split attributes into two classes, because this controls calibration effort. Heterogeneous-agent macroeconomics finds that **indirect effects through labour demand "far outweigh" direct effects**, and that realistic spending responses come from uninsurable income risk combined with holding both liquid and illiquid assets ([Kaplan, Moll & Violante, AER 2018](https://ideas.repec.org/a/aea/aecrev/v108y2018i3p697-743.html)).

The **economic engine** should therefore read only:

- employment status and industry;
- labour income and unemployment risk;
- liquid deposits versus housing and other illiquid wealth;
- debt;
- housing tenure;
- age;
- household size;
- transfer and pension receipt.

**Identity and political attributes** (religion, ethnicity, ideology, interest groups) stay on the record for filtering and politics, but they should not enter the consumption and saving rules. Every attribute that enters a behaviour rule adds a parameter someone must calibrate.

Expectations are the one soft attribute that does belong in the economics. CANVAS learns inflation expectations with simple autoregressive rules. In the Bank of England model, **switching off price expectations removes the housing boom-bust cycles altogether** ([CANVAS](https://www.bankofcanada.ca/wp-content/uploads/2022/12/swp2022-51.pdf); [BoE](https://www.bankofengland.co.uk/working-paper/2016/macroprudential-policy-in-an-agent-based-model-of-the-uk-housing-market)).

Households need one rule the literature does not supply. SimPaths models partnership formation as a probit plus a matching step, leaving home as a probit, immigration by cloning existing households, and emigration by removal ([Bronka et al. 2025](https://microsimulation.pub/articles/00318)). None of the reviewed models says how to **split a household's deposits, house and mortgage when a couple divorces or a child leaves home**. In a stock-flow-consistent game this is mandatory: write an explicit splitting rule as a ledger transaction, or the balance sheets stop adding up.

## Build from the 2011 census sample, then align every flow

The data for generating the population exist. **IPUMS-International holds 10% samples of Romanian censuses, including 2011 with 1,991,024 person records**. The samples carry county identifiers and intact household and dwelling IDs, but there is no 2021 sample yet ([IPUMS-I Romania](https://international.ipums.org/international-action/sample_details/country/ro)).

A workable pipeline:

1. Draw about 75k households from the 2011 sample, stratified by county and urban/rural.
2. Reweight them with iterative proportional fitting to the 2021 census county totals.
3. Impute income, wealth and opinions from survey sources.

Step 3 is the weak link. **EU-SILC microdata require recognised-research-entity access, top-code age at 80 and coarsen region to NUTS 1** ([Eurostat EU-SILC](https://ec.europa.eu/eurostat/web/microdata/european-union-statistics-on-income-and-living-conditions)). Imputation may therefore have to rely on published tables, which limits how realistic combinations such as "income × wealth × debt" can be.

Two things remain unverified:

- Which 2021 census tables Romania has published at county level.
- Whether IPUMS or EU-SILC terms allow shipping a *derived* synthetic population inside a game.

Check both before you commit to the pipeline.

Sampling noise is the population's most visible technical problem. At 1:100, **a county of 200k people gets only about 2,000 records**. The relative month-to-month noise of an event count is about √((1−p)/(n·p)). For an event with a 1% monthly probability, n·p = 20, which gives **about 22% noise if each person is drawn independently**. Charts for small counties would jitter visibly.

The standard fix is alignment: force simulated event counts to match targets, then choose *who* experiences the event from individual probabilities. The method matters, as the benchmark by Li and O'Donoghue shows ([JASSS 2014](https://www.jasss.org/17/1/15.html)). Its two metrics are:

- **Gap to target**: how far the simulated total misses the target.
- **Distribution distortion**: how much the method bends the relationships between variables.

| Alignment method | Gap to target | Distribution distortion | Time per 100k records |
|---|---|---|---|
| Multiplicative scaling | −0.01% to −3.29% | 0.43–1.18 | 50 ms |
| Sort by predicted probability | exact | **8.13 (worst)** | 170 ms |
| Sort by logistic difference | exact | 0.03–1.18 | 193 ms |
| Sidewalk hybrid | ≈0 | 0.03–1.15 | 5,359 ms (Stata) |

JAS-mine, the platform behind SimPaths, now **deprecates the sort-by-difference methods and calls logit scaling "the clear choice" for multi-outcome alignment**. It also provides weighted versions for weighted agents ([JAS-mine alignment](https://www.microsimulation.ac.uk/jas-mine/resources/cookbook/alignment/)).

LIAM2 adds a feature that suits a monthly game: `errors="carry"` carries each period's rounding error into the next period's target ([LIAM2 processes](https://liam2.readthedocs.io/en/stable/processes.html)). Without it, a small county's monthly target of, say, 0.3 births in the sample would be rounded away every month.

Alignment has a limit: it is a last resort that hides badly specified behaviour equations instead of fixing them ([JASSS 2014](https://www.jasss.org/17/1/15.html)). Use it for demographic and labour-market totals, and take the targets from the macro layer each tick. SimPaths, for example, aligns population by gender × age × region and aligns the employment rate ([Bronka et al. 2025](https://microsimulation.pub/articles/00318)).

The monthly tick is unusual for microsimulation. SimPaths "projects data at yearly intervals" ([Bronka et al. 2025](https://microsimulation.pub/articles/00318)). Convert annual hazards to monthly ones with p_m = 1 − (1 − p_a)^(1/12). Run naturally annual events at their own time: school-year transitions in September, and the slow processes spread across months, as Victoria 3 does with pop growth.

Partner matching is cheap at this scale. LIAM2's rank matching runs in O(n log n) ([LIAM2 processes](https://liam2.readthedocs.io/en/stable/processes.html)), and at 1:100 the monthly marriage market has only dozens of candidates per county.

## Columnar tables, integer money and replay keep the core fast and reproducible

Language choice matters less than layout. On the person-month kernel, **C, Rust and Go all ran at about 8.5–9.5 ns per person per tick on one thread, Numba at 11–12, Node typed arrays at about 20, vectorised NumPy at 75–120 and pure Python at about 1,000** (local benchmark on a 2-vCPU sandbox VM).

Storing each attribute as its own column ("struct of arrays") was **10–15x faster than storing one object per person** when a tick touches 4 of 100 attributes (local benchmark on a 2-vCPU sandbox VM). Object-per-agent frameworks in interpreted languages are 3–160x slower than compiled ones in published comparisons ([ABM Frameworks Comparison](https://github.com/JuliaDynamics/ABMFrameworksComparison)).

An entity-component-system (ECS) engine adds little here. People change attribute *values*, not their set of components, so plain columnar tables per entity type give the cache benefit without the dependency. Multithreading barely matters at 190k: Numba's parallel version was *slower* than serial at that size, because thread start-up costs more than a 2 ms tick. Factorio found that naive parallel updates also ran slower than serial, because threads kept invalidating each other's caches ([Factorio FFF-215](https://factorio.com/blog/post/fff-215)). When parallelism is needed, split persons into contiguous blocks sorted by county.

Determinism comes from three choices, and **the Rust and C kernels produced bit-identical results on one and two threads** using them (local benchmark on a 2-vCPU sandbox VM).

1. **Random numbers derived from the situation, not from a shared generator.** Each draw is a keyed function of (seed, tick, entity ID, purpose). These "counter-based" generators pass the strictest statistical test suites and give the same streams on any machine ([Salmon et al., SC11](https://www.thesalmons.org/john/random123/papers/random123sc11.pdf)). Results then do not depend on thread count or on the order agents are processed.
2. **Money as 64-bit integers in bani.** Summing the same 19M floating-point balances in different orders gave different totals (+1.26 versus +1.85 bani), while integer sums are exact. Round once per transaction and assign the rounding remainder by a fixed rule.
3. **Fixed-order reductions, plus no platform maths functions in code that changes state.** `sin`, `exp` and similar "are not defined by the IEEE standard" and differ between libraries ([Bruce Dawson](https://randomascii.wordpress.com/2013/07/16/floating-point-determinism/)). Matching also needs deterministic tie-breaks, such as sorting by (score, ID), and ordered maps instead of Rust's randomised HashMap.

Determinism also solves storage. **Keeping person-level monthly history is out of reach even at 190k**: 600 monthly snapshots at 20–40 MB each would be 12–24 GB. Instead, store:

- aggregate history, which is tens of MB for decades;
- periodic state snapshots in Arrow/Parquet with zstd compression (about 1.8 MB per 190k rows for 8 columns in testing);
- the seed and a log of player commands.

Any historical group view ("students in Cluj in 2031") can then be rebuilt by replaying from the nearest snapshot. This is the same principle as Factorio's deterministic lockstep, which sends only player inputs between machines ([Factorio wiki](https://wiki.factorio.com/Desynchronization)).

The live group explorer is cheap. **At 190k persons, every Polars filter or group-by query tested took 10 ms or less**, and 252-group cross-tabs took about 4 ms (local benchmark on a 2-vCPU sandbox VM). Live charts should still read from a fixed aggregation cube computed during the tick, roughly county × status × age band × sex × education, about 34k cells. Saving that cube monthly for 50 years is about 800 MB uncompressed, so keep it at a coarser time resolution after the first few years.

GPUs are not worth it. FLAME GPU 2 requires NVIDIA CUDA ([FLAMEGPU2](https://github.com/FLAMEGPU/FLAMEGPU2)), which excludes many laptops, and parallel float reductions on GPUs break determinism.

## Calibration, explanation and verification outrank population as risks

The research ranks the risks as follows. Each is scored by how likely it is to force major rework, multiplied by how hard it is to fix late.

| Rank | Risk | Why it ranks here | How the population design touches it |
|---|---|---|---|
| 1 | Calibration and stability | Unsolved for large models; every new subsystem and player lever can destabilise tuned dynamics | Heterogeneous behaviour adds calibration knobs; a 1:1 population makes tuning sweeps ~100x slower |
| 2 | "Why did this change" panel | Must be designed into the ledger and the behaviour rules; very costly to retrofit | Explaining a change in a distribution (e.g. the poverty rate) is harder than explaining an aggregate flow |
| 3 | SFC correctness with AI-written code | Bookkeeping bugs ship even at major studios | Household splits and imputed wealth must post through the ledger |
| 4 | Data integration and the opening balance sheet | The data exist; reconciling them is the work | Survey microdata are restricted, which limits joint distributions |
| 5 | Population at scale | Milliseconds per tick at 1:100 | Noise in small groups (solved by alignment) |
| 6 | Input-output and market solving | An 80×80 solve is about 57 µs | None |

**Calibration is the open-ended risk.** Platt's comparison of calibration methods concludes they "still need development before they can reliably calibrate large-scale models" ([Platt, arXiv:1902.05938](https://arxiv.org/abs/1902.05938)). Surrogate-model approaches exist precisely because each run is too expensive to repeat thousands of times ([Lamperti et al., arXiv:1703.10639](https://arxiv.org/abs/1703.10639)).

The most successful national model sidestepped estimation. Poledna et al. set parameters directly from national accounts, sector accounts, input-output tables and business demography, and still forecast competitively with standard benchmark models ([RePEc](https://ideas.repec.org/a/eee/eecrev/v151y2023ics0014292122001891.html)). Even so, the Bank of Italy describes initialising that model for all EU-27 countries as **"still a work in progress"** ([BeforeIT.jl](https://github.com/bancaditalia/BeforeIT.jl)).

A game has an easier target than a forecaster: plausible, stable paths and believable responses to levers. But the target moves every time a subsystem is added.

Stability belongs to the same problem. Victoria 3 bounds market prices to **25–175% of base** with a clamped formula, and shortage penalties are rate-limited at 1% per day ([Vic3 wiki: Market](https://vic3.paradoxwikis.com/Market)). It still needed a wage-rule retune in patch 1.1 ([Vic3 wiki: Patch 1.1](https://vic3.paradoxwikis.com/Patch_1.1)), and its trade system was reworked again in June 2025, nearly three years after launch ([Wikipedia](https://en.wikipedia.org/wiki/Victoria_3)).

**Explainability is cheap only if you design for it.** Exact Shapley attribution over k factors needs 2^k counterfactual runs, which is 1,024 runs for ten factors ([Wikipedia: Shapley value](https://en.wikipedia.org/wiki/Shapley_value)). That is impossible for every indicator every month. A ledger in which each change to a stock is a sum of tagged flows gives an exact accounting breakdown almost for free.

Behavioural "why" questions (why did firms stop hiring?) need a second mechanism: rules written as sums or products of named driver terms that the engine logs. AI coding agents will drift toward opaque threshold logic unless an interface enforces that pattern.

**Verification failures are predictable.** Victoria 3's patch 1.1 fixed several bookkeeping bugs ([Vic3 wiki: Patch 1.1](https://vic3.paradoxwikis.com/Patch_1.1)):

- a trade loop that created value out of nothing;
- tariffs paid to the wrong party;
- an overflow in gold reserves.

A ledger with per-tick conservation checks catches all of these on the first tick. Galán et al. add that results can also come from "accessory assumptions" such as agent update order, so tests should shuffle the order and confirm the aggregates hold ([JASSS 2009](https://www.jasss.org/12/1/1.html)).

**Data reuse has one specific trap.** Eurostat allows commercial reuse with attribution, but **data on non-EU, non-EFTA and non-candidate countries must be removed before commercial reuse** ([Eurostat copyright](https://ec.europa.eu/eurostat/about-us/policies/copyright)). That matters for the game's non-EU trade blocs. FIGARO input-output tables only reach year T−2 ([Eurostat FIGARO](https://ec.europa.eu/eurostat/web/esa-supply-use-input-tables/information-data)). Reconciling sources into one consistent opening balance sheet is a large work package that is easy to underestimate.

The owner's view that people are the most important part is right for game design. As a technical risk, though, the population mostly matters through the risks above it. One gap in the current proposal deserves more worry than population scale: **one aggregate firm per industry**. Both CANVAS and the Austrian model draw individual firms from business-demography data within each industry, and CANVAS gives each firm an investor household that receives its dividends ([CANVAS](https://www.bankofcanada.ca/wp-content/uploads/2022/12/swp2022-51.pdf); [BeforeIT.jl paper](https://arxiv.org/html/2502.13267v1)). Bankruptcies, credit allocation and the planned real-company firms file all need a firm table.

## What to write into ADR-0003 and the tech-stack ADR

| Decision | Recommendation |
|---|---|
| Representation | Persons and households as columnar tables (~190k / ~75k), linked by integer IDs. A weight column and a global scale factor, so 1:100 is used for play and 1:10 for validation. Records persist; only births, deaths and migration add or remove them |
| Attributes | The economic engine reads only employment, industry, income, income risk, liquid and illiquid wealth, debt, tenure, age, household size, transfers and expectations. Identity and politics ride along. Interest groups are fractional membership vectors; membership changes slowly and satisfaction changes fast |
| Generation | Offline pipeline: IPUMS 2011 10% sample → stratified draw → iterative proportional fitting to 2021 county totals → imputation from survey aggregates → versioned Parquet. Verify IPUMS, EU-SILC and census licence terms first |
| Transitions | Annual hazards converted to monthly. Logit-scaling alignment for multi-outcome events and resampling or sidewalk for yes/no events (weighted versions). Rounding error carried to the next period. Targets come from the macro layer each tick. An explicit ledger rule for splitting household wealth |
| Noise in the UI | Live charts read from an in-tick aggregation cube. The group explorer shows how many records sit behind each view and smooths or flags views built on too few records |
| Determinism | Random numbers keyed on (seed, tick, entity, purpose). Money as int64 bani. Fixed-order reductions. No platform maths functions in code that changes state. Deterministic tie-breaks and ordered maps |
| Core stack | A headless Rust crate with `step(commands) → TickReport`, `snapshot`, `save/load` and `query`. Python plus Polars (bindings via PyO3) for the data pipeline, calibration runs and analysis. No ECS engine, no GPU |
| Storage | Save file = Arrow/Parquet snapshot + seed + command log. Keep aggregate history; rebuild person-level history by replay; store old cube history at coarser time resolution |
| Accounting and explanation | Every money movement is a tagged ledger transaction with conservation checks every tick. The "why" panel reads the ledger. Behaviour rules are written as named driver terms that the engine logs |
| Testing | Golden seeded runs with committed hashes. A check that 1 thread and N threads give identical hashes. Random lever sequences over 100-year runs with stability bounds. Property tests on behaviour rules. A test that each explanation sums to the observed change |
| Calibration | Set parameters directly from national accounts and input-output tables. Keep few free behaviour knobs. Partial adjustment and buffer stocks; log every time a clamp binds. A headless batch runner for parameter sweeps |
| Firms | Replace one aggregate per industry with a firm table: at least the large named firms individually, the rest sampled from business demography |
| Next step | A performance prototype focused on labour and goods-market clearing and on the calibration loop, the parts the benchmarks did not cover. Population throughput needs no further testing |

## Conclusion

The research turns the scale question from a decision into a dial. 1:100, 1:10 and 1:1 all have institutional precedent, and a weight column lets the game move between them. The decisions that are hard to reverse are different ones: integer money in a tagged ledger, rules written as decomposable drivers, and randomness keyed on (seed, tick, entity). These three choices also turn the population from a performance liability into an asset. A deterministic, replayable population makes history queries, bug reproduction, golden-run regression and thousands of calibration runs cheap. That speed is what the top-ranked risk needs.

There is also a lesson in where Victoria 3 struggled. It shipped bookkeeping bugs, retuned feedback rules and reworked its economy for years. Its pop counts were manageable by comparison. For an owner building with AI agents, the leverage is in invariants and tests that agents cannot route around, not in squeezing more people into the simulation. Start the tech-stack ADR from the ledger and the calibration loop, and treat the 190k-person population as the comfortable part of the system it already is.
