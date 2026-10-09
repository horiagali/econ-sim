---
id: adr/0008-explainability-architecture
title: "ADR-0008: Explainability architecture — why did this change"
status: accepted
owner: horia
depends_on: [adr/0007-money-and-ledger, adr/0005-simulation-core-architecture]
updated: 2026-10-09
---

# ADR-0008: Explainability architecture — why did this change

## Context
Pillar 4 says the player can always ask "why did this change?" and get a causal breakdown. The [population research](../../01-research/notes/population-modelling-deep-research.md) ranks this the second-biggest technical risk: it is cheap only if designed into the ledger and the behaviour rules from day one, and very costly to retrofit. AI agents drift toward opaque threshold logic unless an interface forces decomposable rules.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (section "The 'why did this change' ledger: five layers") found:
- Different questions need different machinery. "Why did deposits fall?" is answered by ledger flow codes. "Why did firms raise prices?" needs the rule's driver terms. "Why did poverty rise?" needs a distribution decomposition. "How much was my rate hike vs the energy shock?" needs counterfactual runs.
- Exact Shapley attribution over k factors needs 2^k runs, so it cannot run for every indicator every tick.
- For a log-linear rule, the per-driver contributions wᵢ·Δln xᵢ sum to Δln(output) to machine precision (checked in a sandbox).
- Victoria 3's UX team called nested tooltips the one technology they could not see the game without.

## Options considered
1. **Five layers, with a rule macro as the only way to write behaviour rules.**
   Pros: each question gets the cheapest exact mechanism; the rule format makes explanations automatic.
   Cons: every rule must fit a decomposable shape; a macro to build and maintain.
2. **Per-agent provenance (store why each record changed).**
   Pros: maximum detail.
   Cons: storage explodes (the posting log alone is 13.5 GB per 50 years at 1:100).
3. **Runtime Shapley for everything.**
   Pros: one uniform method.
   Cons: 2^k runs per question; far too slow.
4. **Interpreted rule trees (rules as data evaluated at runtime).**
   Pros: introspectable.
   Cons: 10–100× slower; harder for agents to type-check.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1.**

| Layer | Answers | Mechanism | Cost |
|---|---|---|---|
| L1 Ledger | "Why did household deposits fall?" | Every posting carries a flow code; accumulators by code × sector ([ADR-0007](0007-money-and-ledger.md)) | O(1) per posting; ~230 KB/month with county detail |
| L2 Driver terms | "Why did industry j raise prices?" | Rules are additive (Δ per term) or log-linear (wᵢ·Δln xᵢ, exact) | 10–100 KB/tick |
| L3 Distribution | "Why did poverty rise?" | Shift-share: within-group change vs composition change, plus L1 flow codes of households crossing the line | On demand |
| L4 Counterfactual | "My rate hike vs the energy shock?" | Forked runs with Shapley/Owen attribution over at most 6 lever groups | Offline / on demand |
| L5 Events | "What happened?" | Append-only typed events: defaults, clamp hits, policy changes | Small |

**The `behaviour_rule!` macro is the only way to write a behaviour rule.** A rule declares its named terms, a shape (`additive`, `log_linear` or `partial_adjustment`), its adjustment speed λ, its clamp IDs, and the provenance of each parameter (`from_data`, `fitted` or `tuned`). From that one declaration the macro generates:
- a fast `eval` with no allocation;
- an `eval_explained` that returns the contribution of each term;
- a `META` record that feeds the docs, the stability test ([ADR-0009](0009-calibration-and-stability.md)), the causal-graph view and the calibration parameter list.

A rule that cannot be decomposed needs an explicit `shape: opaque` waiver. CI counts waivers and reports the count on every PR.

**Query API.** The core exposes `explain(indicator, region, window) -> ContributionTree`. The UI shows the 3–7 largest children plus "other"; each child can be hovered for its own breakdown, and hovering any chart point shows that period's tree.

**Explanations must sum exactly.** Σ contributions = Δ indicator holds exactly (in bani) for L1 and within 1e-9 relative for L2. This is invariant I-6 and a test.

**Open semantic question (recorded, not decided).** How to aggregate log-linear contributions across agents of different sizes: LMDI weights in levels, or weighted mean log contributions. It may differ per indicator. Spike 3 settles it, and this ADR is amended with the result before it is accepted.

## Consequences
- Easier: every indicator has a "why" from the first mechanic; explanations, docs and calibration parameter lists cannot drift from the code, because the macro generates all of them.
- Easier: agents cannot quietly add opaque logic; the waiver count makes it visible.
- Harder: some behaviours (thresholds, regime switches) must be expressed as smooth terms or carry a waiver.
- Harder: L3 and L4 explanations cost compute and need UI patience (on-demand, cached).
- Revisit if the macro becomes a compile-time or ergonomics burden for agents.

## Open questions / to verify
- [ ] Log-linear aggregation across agents of different size: LMDI vs weighted mean log contributions (Spike 3).
- [ ] Which indicators get L3 shift-share decompositions, and how groups are chosen for them.
- [ ] Maximum lever groups for L4 (6 proposed) and whether results are cached per save.
- [ ] Spike 3 exit criterion: a nested tree for one indicator sums exactly; μ metadata test runs on 3 rules.
