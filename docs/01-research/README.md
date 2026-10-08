---
id: research/overview
title: Research — Index & Backlog
status: draft
owner: horia
depends_on: []
updated: 2026-10-08
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

### P1 — core mechanics
- [ ] Wage-setting and Phillips curve evidence (incl. flattening debate).
- [ ] Price setting & inflation expectations (markup pricing, anchoring).
- [ ] Monetary transmission lags (rule of thumb: 12–24 months to inflation peak effect).
- [ ] Fiscal multipliers by instrument and state of economy.
- [ ] Consumption functions: MPC by income group.
- [ ] Debt sustainability & sovereign risk premia; historical default triggers.
- [ ] Trade elasticities and exchange-rate pass-through.
- [ ] Laffer / tax base elasticities by tax type.

### P2 — calibration data
- [ ] Data sources catalogue: World Bank WDI, IMF WEO & IFS, OECD, Penn World Table, OECD ICIO input-output tables, ILOSTAT, BIS.
- [ ] A "typical" economy archetype set (advanced, emerging, low-income, commodity exporter) with key ratios.

### Later
- [ ] Political economy: how economic outcomes drive approval/elections.
- [ ] Economics of military spending and conflict.
