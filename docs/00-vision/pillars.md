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

1. **Stock-flow consistency.** All flows come from somewhere and go somewhere. Every sector's balance sheet balances every tick. Sector financial balances sum to zero. This is the backbone that stops the sim from producing nonsense.
2. **Explainability.** The player can always ask "why did this change?" and get a causal breakdown. Mechanics must therefore be decomposable — no black-box lookups.
3. **Plausible over precise.** Qualitative behaviour must match reality (raising rates cools inflation with a lag; printing money to fund deficits eventually causes inflation). Exact elasticities can be tuned later.
4. **Systems over scripts.** Prefer a mechanism that produces a crisis to an event that declares one. Scripted events are allowed only as flavour or for exogenous shocks (pandemics, oil shocks).
5. **Meaningful trade-offs.** Every lever has a cost somewhere. If a policy is strictly dominant, the model is wrong.
6. **Extensible core.** Politics and military will plug into the same state. Economic mechanics must expose clean outputs (unemployment, inequality, tax capacity, industrial output) that those systems can read.
