---
id: vision/pillars
title: Design Pillars
status: draft
owner: horia
depends_on: [vision/vision]
updated: 2026-10-08
---

# Design Pillars

Every mechanic must be defensible against these. When two conflict, earlier pillars win.

1. **Stock-flow consistency.** All flows come from somewhere and go somewhere. Every sector's balance sheet balances every tick, and sector financial balances sum to zero. The same holds for people: every person belongs to exactly one household, and people only enter or leave through births, deaths and migration. This is the backbone that stops the sim from producing nonsense.
2. **Robust and correct.** Not buggy is a headline feature. The simulation is deterministic given a seed, guarded by invariant checks every tick, and covered by acceptance tests from every spec. No NaNs, negative stocks or runaway values without a mechanism that explains them.
3. **People are the unit of consequence.** Policies act on concrete population groups. Aggregate indicators (GDP, inflation, unemployment) are *sums over* people and firms, never free-floating numbers. A policy that "helps the economy" must help or hurt someone specific, and the player can see who.
4. **Explainability.** The player can always ask "why did this change?" and get a causal breakdown. Mechanics must therefore be decomposable: no black-box lookups.
5. **Plausible over precise.** Qualitative behaviour must match reality (raising rates cools inflation with a lag; printing money to fund deficits eventually causes inflation; devaluation helps exporters). Exact elasticities can be tuned later.
6. **Systems over scripts.** Prefer a mechanism that produces a crisis to an event that declares one. Scripted events are allowed only as flavour or for exogenous shocks (pandemics, oil shocks).
7. **Meaningful trade-offs.** Every lever has a cost somewhere. If a policy is strictly dominant, the model is wrong.
8. **Extensible core.** Politics and military will plug into the same state. Mechanics expose clean outputs (unemployment, inequality, tax capacity, industrial output, approval, ideology and interest groups by person) that those systems can read.
