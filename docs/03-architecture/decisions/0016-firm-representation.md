---
id: adr/0016-firm-representation
title: "ADR-0016: Firm representation"
status: accepted
owner: horia
depends_on: [adr/0002-modelling-approach, adr/0003-people-representation, adr/0007-money-and-ledger, adr/0012-data-pipeline-and-licensing, economy/production, economy/industries, research/population-modelling-deep-research]
updated: 2026-10-09
---

# ADR-0016: Firm representation

## Context
The specs used to model **one aggregate firm per industry** (81 at the time), split into domestic private, foreign-owned and state-owned parts ([production](../../02-design/economy/production.md)). The [population research](../../01-research/notes/population-modelling-deep-research.md) flags this as a bigger modelling gap than population scale: CANVAS and the Austrian/BeforeIT models draw individual firms from business-demography data, and several mechanics need firms as rows:
- **Bankruptcies and entry:** an aggregate firm cannot partly go bankrupt; defaults become a smooth write-down with no visible exits.
- **Credit:** SMEs and large firms differ in credit access, rates and default rates; EU funds and state aid often target SMEs.
- **Heterogeneity:** small and large firms differ in productivity, wages, exporting, informality and bankruptcy rates, so policies (minimum wage, SME tax regimes, enforcement) land differently.
- **Real names:** the scope promises real company names for the largest firms ([scope](../../00-vision/scope.md)), shown as events (Dacia expands, a plant closes).

The owner suggested: **per industry, 3 named real companies plus a 4th generic firm representing all smaller companies.** This ADR evaluated that idea and proposed a refinement, which the owner accepted.

## Options considered
**(a) One aggregate firm per industry (previous design).**
- Pros: simplest; ~90 units (×3 ownership parts); easy to calibrate to national accounts.
- Cons: no bankruptcies or entry, no SME/large split, no named firms as real rows; credit and EU-funds mechanics for SMEs cannot work; ownership splits are bolted on.

**(b) Owner's proposal: 3 named firms + 1 generic firm per industry.**
- Pros: simple and readable (~360 units); real names everywhere; one generic firm keeps the long tail cheap.
- Cons: a fixed 3 does not fit Romania's industry structure. Concentrated industries have fewer or more than 3 real players that matter (cars: Dacia and Ford Otosan; energy: Hidroelectrica, Nuclearelectrica, OMV Petrom, Romgaz). Fragmented industries (agriculture, restaurants, construction, personal services) have no firm whose name a player would recognise, so 3 names there would be arbitrary small firms. One generic firm still mixes micro and medium firms, which differ in productivity, wages, credit and informality, and it cannot show bankruptcies except as a smooth write-down.

**(c) Variable named firms + size-class cohort firms.**
- Pros: real players where they exist and none where they don't; SME heterogeneity by size class; bankruptcies and entries become changes in a count; the same weighted-record logic as the synthetic population. About 550–700 units: cheap.
- Cons: more units to calibrate than (a) or (b); needs business-demography data by size class and NACE; within-cohort heterogeneity is still averaged.

**(d) Full firm sample like CANVAS (thousands of firm records).**
- Pros: richest heterogeneity; firm-size distributions and network effects emerge.
- Cons: much more data (firm-level microdata are not available to an individual), many more parameters to calibrate, harder to explain, and more ledger postings; the extra detail is mostly invisible to the player.

## Decision
Accepted by the owner on 2026-10-09. The owner explicitly chose the variable 0–5 named firms per industry (by threshold) plus size-class cohort firms, and delegated the open details; they are resolved below as decisions.

**Chosen: option (c).** Firms are represented as **firm units**: a small table per industry containing named firms and cohort firms.

### Named firms
A real company is a **named firm** in an industry if it meets at least one of:
- ≥ 5% of the industry's output or employment;
- among Romania's top ~20 exporters;
- a state-owned company with ≥ 1% of the industry's output.

At most **5 named firms per industry**, largest first. Concentrated industries get their real players (cars: Dacia, Ford Otosan; energy: Hidroelectrica, Nuclearelectrica, OMV Petrom, Romgaz). Fragmented industries (agriculture, restaurants, construction) get none. A named firm has `firm_count = 1`. The list, with real names, lives in the `firms` data file, never in code ([ADR-0015](0015-distribution-and-licensing.md)).

A company active in several industries (e.g. OMV Petrom in `crude_oil`, `refined_petroleum` and `power_gas`) appears as a separate named unit in each industry where it passes the threshold. The units share a `company` id in the data file, so events, ownership changes and nationalisation can act on the whole company.

### Cohort firms
Per industry, up to three size-class cohorts cover everything not named:
- **micro**: fewer than 10 employees
- **small**: 10–49 employees
- **medium/large**: 50 or more employees, excluding named firms

Each cohort is one representative firm carrying an integer **firm count**: the number of real firms it stands for. Where the foreign share of a cohort is material (≥ 10% of its output or employment), the cohort is split into a **domestic** and a **foreign-owned** cohort; within each split, control is 100% domestic or 100% foreign (see Ownership). Empty or negligible cohorts are dropped at scenario build; a cohort whose count falls to 0 during play stays in the table (inactive) so entry can revive it.

### What every firm unit has
Named or cohort; per real firm for cohorts:
- its own balance sheet: deposits, loans, capital, inventories, equity;
- employment by occupation;
- capacity split across regions as a **vector** (not separate units per region);
- an ownership vector (`ownership[f]`) and a controller (`controller[f]`); see Ownership;
- in v1, the price is the industry's market price (all units in an industry sell at it). Exception: generation units sell `wholesale_electricity` at the merit-order price ([energy](../../02-design/economy/energy.md)).

The industry is the sum of its units, each weighted by its firm count. Industry output, employment, capital, loans and so on are derived, never stored separately.

**Size.** About 90 industries × 6–8 units ≈ 550–700 firm units. Compute cost is negligible next to the synthetic population.

### Same representation as the synthetic population
Cohort firms are **weighted records** with `firm_count` as the weight, just as synthetic households carry `hh_weight` ([ADR-0003](0003-people-representation.md)). Ledger postings between a firm unit and a sector-level counterparty (bank, government, rest of world) are multiplied by the firm count at posting time, consistent with the weighted-posting rule in [ADR-0007](0007-money-and-ledger.md). **Firm counts are read from the scenario and never hardcoded**; no table is sized by a firm-count constant, and the number of units per industry follows from the data.

### Postings between two weighted records
When both sides are weighted records (a synthetic household paid by, or buying from, a cohort firm; a dividend from a cohort firm to households; a cohort firm buying from another cohort firm), money goes through a **per-industry clearing account** ([ADR-0007](0007-money-and-ledger.md)):
- **Wages:** each firm unit posts its total wage bill (per-firm amount × `firm_count`) to the industry's wage clearing account `wage_clearing[j]`; each household receives from it per person (per-person amount × `hh_weight`).
- **Sales:** household spending on good *g* posts to the industry's sales clearing account `sales_clearing[j]` (per-household amount × `hh_weight`); firm units receive shares by market share.
- The same pattern is used for dividends to households and for purchases between cohort firms.
- **Exactness.** The side with the finer split is computed first and defines the total (for wages and sales, the household side). The total is split across firm units by largest remainder on unit totals. A unit total that is not a multiple of `firm_count` changes the per-firm balance by the integer quotient and keeps the rest (< `firm_count` bani) in the unit's carried remainder `posting_remainder[f]`, so the unit's total balance is exact.
- **Invariant I-8:** every clearing account nets to exactly zero at the end of every tick ([ADR-0007](0007-money-and-ledger.md), [accounting](../../02-design/economy/accounting.md)).

### Exit and entry of cohort firms
Per tick, for each cohort unit $f$:
```math
\text{exits}_f = \text{exit\_rate}_f \cdot n_f, \qquad \text{entries}_f = \text{entry\_rate}_f \cdot n_f
```
- `firm_exit_rate[f]` (bankruptcy rate) rises with low profitability, high leverage or low interest cover, and tight credit conditions, around a base rate per industry and size class from business demography.
- `firm_entry_rate[f]` (birth rate) rises with profits, demand growth and credit availability, and falls with regulatory burden (and state capacity, [state-capacity](../../02-design/economy/state-capacity.md)).
- Counts stay integers through **alignment with a carried remainder**, the same method as population transitions ([ADR-0003](0003-people-representation.md)).
- Exiting firms are wound up as average firms of the cohort: their share of assets is applied to creditors, residual loans are written down as bank losses per the ledger default rules, equity holders lose the rest, and their workers are laid off. Entrants start with an entrant balance sheet (a fraction of the cohort's average size, financed by owner equity and loans), and the cohort's per-firm values become the count-weighted average.

> **Simplification:** exiting firms carry the cohort's average balance sheet. A loss-given-exit adjustment (exiting firms being weaker than average) can be added later as a tuning parameter.

### Failure of a named firm
Named firms never disappear silently.
1. **Insolvency** (equity negative or unable to pay due debt) puts the firm **under administration** for `administration_ticks` ticks. It keeps operating, keeps its workers and is shown as an event.
2. **Rescue** during administration: the player can bail it out (capital injection) or nationalise it ([state-enterprises](../../02-design/economy/state-enterprises.md)), or a buyer (domestic, foreign or another named firm) acquires it ([foreign-ownership](../../02-design/economy/foreign-ownership.md)).
3. **If not rescued:** its capacity and workers move into the matching size-class cohort (count +1, or split across cohorts by employment if it is larger than one cohort firm); its debts are written down per the ledger rules; and the named entry is retired with an event ("X closes; its plants are taken over by smaller firms").

### Non-market public producers
Public administration, public schools and universities, public hospitals and public social care are **government-owned firm units** in their industries (`public_administration`, `education_services`, `health_services`, `social_care`; `defence_services` once the military layer exists). They have state share 100% and are **non-market**: output is valued at cost (wages, intermediate inputs, depreciation) and bought by the government from the budget. Employment and wages therefore flow through the same firm-unit and labour-market rules as everywhere else; the public sector headcount and wage levers act on these units. Private schools, clinics and care homes are ordinary market cohorts in the same industries.

## Ownership
Accepted by the owner on 2026-10-09.

**Ownership vector.** Every firm unit has `ownership[f]`: equity shares summing to 100%:
- `state`: the government (central or local);
- `domestic_private`: Romanian households, held directly or through Pillar II pension funds;
- `foreign`: the rest of the world.

Free float is folded into `domestic_private` or `foreign` by holder. `state_share[f]` ($\omega_f$) is the `state` component. Named firms start with their real shareholder structure *(verify all)*, for example:
- OMV Petrom: OMV majority (foreign), Romanian state stake, rest listed float;
- Hidroelectrica: state majority (~80%), rest listed float;
- Nuclearelectrica, Romgaz, Transelectrica: state majority, rest listed float;
- Dacia: Renault group, 100% foreign.

Cohort units are 100% domestic private or 100% foreign by their split; municipal companies (water, heating, local transport) are cohorts or named units with a state share.

**Control.** `controller[f]` is the largest holder if it owns ≥ 50%; otherwise the largest holder of the remaining blocs (state, domestic private, foreign). The controller sets behaviour:
- **state-controlled** units follow SOE directives ([state-enterprises](../../02-design/economy/state-enterprises.md));
- **foreign-controlled** units follow the foreign-ownership rules: dividend repatriation, transfer pricing, relocation ([foreign-ownership](../../02-design/economy/foreign-ownership.md));
- **domestic-private-controlled** units follow the default profit rules ([production](../../02-design/economy/production.md), [investment-capital](../../02-design/economy/investment-capital.md)).

Minority holders do not change behaviour; they only receive their share of dividends and of sale or nationalisation proceeds.

**Dividends.** A unit's dividend is paid **pro rata to the ownership vector** through the ledger:
- state share → government revenue (SOE dividends);
- foreign share → rest of world (primary-income outflow in the current account);
- domestic private share → domestic equity holders, split between households holding equity directly (allocated by their equity holdings, through the clearing pattern above) and **Pillar II pension funds**.

**Pension funds.** Pillar II pension funds are modelled as a **financial sub-sector** that holds equity, government bonds and deposits on behalf of member households; members hold a claim on the funds (`hh_pension_assets`). Dividends and interest received by the funds raise the members' claims ([accounting](../../02-design/economy/accounting.md)).

**Changes in ownership.** Only these mechanisms change `ownership[f]`; each is a ledger transaction at the transaction value below:
- nationalisation (compensated or expropriation) and privatisation: [state-enterprises](../../02-design/economy/state-enterprises.md);
- foreign acquisitions (brownfield FDI) and greenfield investment (new foreign capacity or a new foreign unit): [foreign-ownership](../../02-design/economy/foreign-ownership.md);
- IPOs and share sales on the stock exchange: the seller offers a stake; domestic households (by wealth), pension funds and the RoW buy in proportion to their demand for equity at the transaction value, with a discount that grows with the stake's size relative to market depth;
- cohort entries and exits keep the cohort's split (100% domestic or 100% foreign).

A change that moves `controller[f]` switches the unit's behaviour rules from the next tick and is shown as an event.

**Equity valuation for transactions.**
```math
V_f = E^{book}_f \cdot m_f, \qquad m_f = \text{clamp}\Big(m^0 \cdot \frac{ROE_f}{r^B + \text{erp}},\ m^{min},\ m^{max}\Big)
```
$E^{book}_f$ is book equity (per real firm for cohorts), $ROE_f$ the trailing return on equity, $r^B$ the government bond yield and `erp` an equity risk premium. `equity_multiple[f]` ($m_f$) rises with profitability and falls with interest rates. A stake $\Delta$ costs $\Delta \cdot n_f \cdot V_f$, minus the market-depth discount for large sales.

> **Simplification:** no traded share prices or stock-market dynamics in v1; the valuation rule is used only when ownership changes, and revaluations of held equity are booked from it each tick.

### Data
The `firms` stage of the pipeline ([ADR-0012](0012-data-pipeline-and-licensing.md)) builds the firm-unit table from:
- business demography by size class and NACE (Eurostat structural business statistics and business demography, INS): counts, employment, turnover and value added per cohort, and base exit and birth rates;
- named firms from company reports, ONRC filings, Ministry of Finance financial statements and top lists (employment, turnover, exports, counties, shareholders);
- foreign shares by industry (INS, BNR FDI report) to decide the domestic/foreign split.

Cohort totals are reconciled to industry totals from the national accounts and IO tables, so the industry sums match.

## Consequences
- Easier: bankruptcies and entries are visible (counts fall or rise; named firms go under administration and close as events); SME credit, SME tax regimes and EU funds for SMEs have a target; informality can differ by size class.
- Easier: named firms appear only where a player would recognise them; the list is a data change.
- Easier: one weighted-record rule for households and firms; the multi-scale CI test covers both.
- Easier: ownership, dividends, nationalisation and FDI use one vector and one control rule, instead of separate rules in each spec.
- Easier: public services employ people through the same rules as firms.
- Harder: ~550–700 units to initialise and calibrate instead of ~90; the opening balance sheet needs business demography by size class and shareholder data for named firms.
- Harder: wages, sales and dividends between weighted records go through clearing accounts with carried remainders, and a new invariant (I-8) must be checked.
- Harder: a cohort's exiting firms are treated as average firms unless a loss-given-exit adjustment is added.
- Revisit if profiling or calibration shows too many units, or if a mechanic needs within-cohort heterogeneity (then consider more size classes, not option (d)).

## Open questions
- [x] Owner confirmation of option (c) over (b) (accepted 2026-10-09).
- [x] Threshold for named firms: ≥ 5% of output or employment, top ~20 exporters, or SOE with ≥ 1% of output; max 5 per industry.
- [ ] Compile the named-firm list and shareholder structures per industry (data task, `firms` stage).
- [x] Postings between weighted records: per-industry clearing accounts netting to zero (I-8).
- [x] Exit and entry rules for cohorts: rates from financial stress and profitability, alignment with carried remainder; exiting firms are average firms in v1.
- [x] Named-firm failure: administration for `administration_ticks`, then rescue or absorption into the matching cohort.
- [x] Non-market public producers are government-owned firm units valued at cost.
- [ ] Default for `administration_ticks` (proposed 6–12 ticks) and the entrant balance-sheet fraction.
- [ ] Whether units within an industry may later set different prices (v1: one market price).
