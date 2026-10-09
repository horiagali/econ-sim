---
id: vision/inspirations
title: Inspirations & Competitor Teardown
status: stub
owner: horia
depends_on: [vision/vision]
updated: 2026-10-08
---

# Inspirations & Competitor Teardown

For each game: what it does well, what it does badly, what we take, what we avoid.

## Power & Revolution (Eversim, Geopolitical Simulator series)
- **Does well:** breadth of levers, real-world countries, feeling of running a state.
- **Does badly (owner's view):** buggy; economic outcomes don't feel close to real life.
- **Weak spots to verify:** opaque economic model, unclear causality, UI overload. *(To research: gather player reviews and forums.)*
- **Take:** head-of-state fantasy, breadth of levers, ministry-based policy structure.
- **Avoid:** bugs and inconsistent numbers (→ [pillar 2](pillars.md)); unexplained outcomes (→ pillar 4).

## Victoria 3 (Paradox)
- **Pops** (population groups by profession, culture, religion, with wealth and political leanings), goods markets, buildings, production methods.
- **Take:** pops-like visibility of who is affected; interest groups; goods with prices driven by supply and demand; supply chains.
- **Avoid:** cell-based pops at our level of detail (combinatorial explosion; see [people-model research](../01-research/notes/people-model-approaches.md)).
- **Improve on:** finance and money (no real banking or credit), monetary policy, realistic macro feedback.

## Democracy 4 (Positech)
- Policy-to-voter-group influence graph with very readable causality. Voter groups with overlapping membership.
- **Take:** clarity of "this policy affects these groups by this much"; per-group approval.
- **Improve on:** shallow economy.

## Tropico / Workers & Resources
- City or region-level production chains; useful for supply-side intuition and for physical infrastructure and power plants.

## Non-game references
- Stock-flow consistent macro models (Godley & Lavoie).
- Agent-based macro models (e.g. Eurace, the Mark-0 family, Dosi et al. K+S).
- Input-output tables (OECD ICIO) for supply chains.
- Central bank teaching simulators (e.g. "Chair the Fed").

## Open questions
- [ ] What specifically frustrates P&R players about its economy? (forums, Steam reviews)
- [ ] Which game's UI for explaining causality is best?
- [ ] What is P&R 2026's list of economic sectors? (Owner's reference for industry granularity.)
