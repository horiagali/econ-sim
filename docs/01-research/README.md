---
id: research/overview
title: Research — Index & Backlog
status: draft
owner: horia
depends_on: []
updated: 2026-10-09
---

# Research

Research notes live in [`notes/`](notes/), one question per file, using the [research template](../templates/research-note.md). Each note ends with "Implications for the game" linking the mechanic specs it affects.

## How to run a research task with an AI agent
1. Pick a question from the backlog below; create `notes/<slug>.md` from the template.
2. Ask the agent to fill **Short answer**, **Findings** (with sources + confidence) and **Numbers worth keeping**.
3. Verify key sources yourself. Downgrade confidence for anything unsourced.
4. Write **Implications for the game** and link it from the relevant spec's `research:` field.

## Backlog (priority order)

### P0 — blocks architecture
- [ ] **Modelling approach:** SFC aggregate vs agent-based vs hybrid. Trade-offs for a game (performance, explainability, emergent crises). Key refs: Godley & Lavoie *Monetary Economics*; agent-based macro (Eurace, Mark-0, Dosi et al. K+S model).
- [ ] **Survey of existing economic games' models** (P&R, Victoria 3, Democracy 4) — what they simulate and where players say it breaks.
- [ ] **Tick length & simulation stability:** numeric stability of monthly vs quarterly SFC models.
- [x] **People model at scale:** cells vs voter sample vs weighted microsimulation → [people-model-approaches](notes/people-model-approaches.md); deep research → [population-modelling-deep-research](notes/population-modelling-deep-research.md).
- [x] **Tech stack and the hardest technical problems** (calibration and stability, explainability, SFC verification, data licensing, determinism, population scale) → [tech-stack-deep-research](notes/tech-stack-deep-research.md); decisions in ADR-0004 to ADR-0015, accepted 2026-10-09 (see the [architecture overview](../03-architecture/README.md)).
- [ ] **Synthetic population pipeline for Romania:** this is a personal, non-commercial project, so the base is the IPUMS-International 2011 Romania 10% sample (free non-commercial registration; raw microdata kept private), reweighted with IPF to 2021 census county totals, with income, wealth and opinions imputed from published survey tables ([ADR-0003](../03-architecture/decisions/0003-people-representation.md), [ADR-0012](../03-architecture/decisions/0012-data-pipeline-and-licensing.md)). EU-SILC microdata need institutional access, so its published tables are used.
  Still to answer: which IPUMS Romania 2011 variables are available, which 2021 census tables exist at county level, imputation method for income and wealth, validation targets.
- [x] **Performance prototype:** person-month kernel benchmarked in candidate languages ([population research](notes/population-modelling-deep-research.md)); market clearing and the calibration loop are left to Spike 4 and Spike 5 in the [roadmap](../roadmap.md).

### P1 — core mechanics
- [ ] Wage-setting and Phillips curve evidence (incl. flattening debate).
- [ ] Price setting & inflation expectations (markup pricing, anchoring).
- [ ] Monetary transmission lags (rule of thumb: 12–24 months to inflation peak effect).
- [ ] Fiscal multipliers by instrument and state of economy.
- [ ] Consumption functions: MPC by income group.
- [ ] Debt sustainability & sovereign risk premia; historical default triggers.
- [ ] Trade elasticities and exchange-rate pass-through.
- [ ] Laffer / tax base elasticities by tax type.
- [ ] Engel curves and consumption baskets by income class and age (for LES parameters).
- [ ] Merit-order electricity pricing and plant-type cost table (capex, build time, capacity factor).
- [ ] Output elasticity of public capital by infrastructure type.
- [ ] Returns to education and enrolment response to student support.
- [ ] Determinants of crime (unemployment, inequality, policing, education).
- [ ] Economic voting: approval vs inflation, unemployment, real income; loss aversion.
- [ ] SOE productivity vs private.
- [ ] Migration elasticities (internal, emigration of skilled workers).

### P2 — calibration data
- [ ] Data sources catalogue: World Bank WDI, IMF WEO & IFS, OECD, Penn World Table, OECD ICIO input-output tables, ILOSTAT, BIS, Eurostat EU-SILC (income distribution), UN population data.
- [ ] Derive the 32-industry I-O coefficient table from OECD ICIO.
- [ ] **Romania calibration pack:** population by county/age/sex/ethnicity/religion (Census 2021), employment by industry and occupation (LFS), income distribution (EU-SILC), household budgets (HBS), I-O tables (INS / Eurostat FIGARO), government budget and tax rules (MF, ANAF), BNR balance sheet and rates, BoP and remittances, energy mix (ANRE, Transelectrica), regional GDP by county.
- [ ] **Romania current rules** (verify, they change often): income tax, social contributions, VAT rates, micro-enterprise regime, minimum wage, pension formula, child benefit, student grants.
- [ ] **Diaspora:** size, destinations, remittance flows, return migration.
- [ ] **EU layer:** current EDP status and adjustment path, EU budget contribution, state-aid thresholds, VAT directive limits, common external tariff by product.
- [ ] **EU funds:** envelopes 2021–2027 by programme, recovery-plan (PNRR) milestones and deadline, CAP payments per hectare, historical absorption rates, decommitment rules.
- [ ] **Informal economy:** undeclared work share by industry, envelope wages, VAT gap (EU Commission VAT gap report), enforcement effects of e-invoicing (RO e-Factura).
- [ ] **Foreign ownership:** foreign share of capital and employment by industry (INS, BNR FDI report), dividend outflows, profit-shifting estimates.
- [ ] **Housing:** dwelling stock, ownership, vacancies by county (Census 2021), price and rent indices, first-home schemes, construction costs.
- [ ] **Environment:** emissions by sector (national inventory), ETS coverage and free allocation, coal phase-out commitments, air quality by county, drought and flood history and yield impacts.
- [ ] **Euro:** current convergence criteria values for Romania, ERM II procedure and pre-conditions, effects of adoption in recent joiners (Slovakia, Baltics, Croatia), Grexit-style exit analyses.
- [ ] **Schengen:** accession date and effects on border wait times and trade costs; external border obligations.
- [ ] **Named firms and business demography** ([ADR-0016](../03-architecture/decisions/0016-firm-representation.md)): firms above the named-firm threshold per industry (≥5% of output or employment, top exporters, SOEs) with ownership, counties, employment, exports (company reports, ONRC, Ministry of Finance data, Ziarul Financiar top lists); state shares in SOEs; firm counts, employment and turnover by size class and NACE (Eurostat SBS and business demography, INS).
- [ ] **State capacity:** Worldwide Governance Indicators, corruption perception, public investment efficiency (IMF PIMA), administrative capacity measures.
- [ ] **Interest groups and ideology:** survey data (Eurobarometer, European Values Study, European Social Survey) to seed ideology and group memberships by attributes.
- [ ] **P&R 2026 sector list** for industry granularity.

### Later
- [ ] Political economy: how economic outcomes drive approval/elections.
- [ ] Economics of military spending and conflict.
