---
id: adr/0002-modelling-approach
title: "ADR-0002: Economic modelling approach"
status: accepted
owner: horia
depends_on: [research/overview]
updated: 2026-10-09
---

# ADR-0002: Economic modelling approach

## Context
The core choice of how the economy is represented determines realism, performance, explainability and how crises emerge.

**New requirements from the v1 definition (2026-10-08):**
- People modelled in fine detail; representation decided separately in [ADR-0003](0003-people-representation.md) (weighted synthetic population).
- About 90 **industries** linked by an input-output supply chain ([industries](../../02-design/economy/industries.md)).
- Strict stock-flow and person conservation, determinism and explainability.

These fit option 3 (hybrid): SFC sector accounting as the backbone, with the synthetic population as the disaggregated household sector and industries as the disaggregated firm sector. Transitions are deterministic fractional flows, not random agents.

## Options considered
1. **Aggregate stock-flow consistent (SFC) model** — sectors as aggregates with balance sheets; behavioural equations. Very explainable, cheap to run, accounting-safe. Less heterogeneity.
2. **Agent-based model (ABM)** — many households/firms/banks. Rich emergence and distribution; harder to explain, tune and keep stable; costlier.
3. **Hybrid** — SFC backbone with disaggregated households, industries and banks for heterogeneity.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 3 (hybrid).** SFC sector accounting is the backbone; the weighted synthetic population ([ADR-0003](0003-people-representation.md)) is the disaggregated household sector; industries linked by input-output tables, represented by firm units ([ADR-0016](0016-firm-representation.md)), are the disaggregated firm sector. Transitions are deterministic fractional flows with alignment, not free-running random agents.

## Consequences
- Easier: every sector has a balance sheet, so stock-flow consistency and the ledger invariants apply everywhere ([ADR-0007](0007-money-and-ledger.md)).
- Easier: distributional detail comes from the synthetic population without a full agent-based model's tuning cost.
- Harder: behavioural equations still need calibration and stability work ([ADR-0009](0009-calibration-and-stability.md)).
- Revisit if the firm representation or the population proves too coarse for the crises the game must produce.
