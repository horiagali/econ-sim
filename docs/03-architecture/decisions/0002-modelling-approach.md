---
id: adr/0002-modelling-approach
title: "ADR-0002: Economic modelling approach"
status: proposed
owner: horia
depends_on: [research/overview]
updated: 2026-10-08
---

# ADR-0002: Economic modelling approach

## Context
The core choice of how the economy is represented determines realism, performance, explainability and how crises emerge. Pending P0 research.

## Options considered
1. **Aggregate stock-flow consistent (SFC) model** — sectors as aggregates with balance sheets; behavioural equations. Very explainable, cheap to run, accounting-safe. Less heterogeneity.
2. **Agent-based model (ABM)** — many households/firms/banks. Rich emergence and distribution; harder to explain, tune and keep stable; costlier.
3. **Hybrid** — SFC backbone with a small number of cohorts (income groups, industries, banks) for heterogeneity.

## Decision
_Pending research._ Leaning: **Hybrid (3)**.

## Consequences
_TBD_
