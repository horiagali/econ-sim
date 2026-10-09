---
id: adr/0015-distribution-and-licensing
title: "ADR-0015: Distribution and third-party licensing"
status: accepted
owner: horia
depends_on: [adr/0013-ui-stack, adr/0012-data-pipeline-and-licensing]
updated: 2026-10-09
---

# ADR-0015: Distribution and third-party licensing

## Context
This is a **personal, non-commercial project**, not a product for sale (owner decision, 2026-10-09). The ADR was first drafted for a game sold on Steam and itch.io; its scope is now reduced to dependency licence hygiene and how real company names are stored. Data licences are handled in [ADR-0012](0012-data-pipeline-and-licensing.md).

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (section "Shipping and licensing") found:
- cargo-deny can enforce an SPDX licence allow-list that denies everything not listed.
- **BeforeIT.jl's licence is ambiguous**: GitHub's sidebar says AGPL-3.0, the LICENSE file on `main` says Apache-2.0. An agent's line-by-line port from Julia to Rust risks becoming a derivative work.
- Real company names carry trademark and defamation risk when firms are shown failing, which matters mainly if the game is published.
- The research also covered Steam Direct, itch.io's butler, installers and code signing; that material is kept in the research note for later.

## Options considered
**Distribution**
1. **None for now; deferred.** The owner runs local builds. No store pages, installers or code signing.
2. **itch.io first, Steam at the vertical slice** (the earlier proposal). Only useful if the game is published.

**BeforeIT.jl**
1. **Work from the papers; check the licence before copying any code.** Reading papers is always fine.
2. **Port the code freely.** Fastest; legal risk if AGPL applies.

**Firm names**
1. **Real names in a swappable data file.** Realism now, fictional fallback in one data change.
2. **Hardcoded real names.** Realism; expensive to change.
3. **Fictional names only.** No risk; less realism.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1 in each group.**

**Distribution is deferred.** No Steam, itch.io, installers, code signing or updater for now. This is revisited only if the project changes (for example, if it is ever published or sold; see the checklist in [ADR-0012](0012-data-pipeline-and-licensing.md)).

**Dependency licence hygiene**
- cargo-deny allow-list: MIT, Apache-2.0 (also with the LLVM exception), BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0. MPL-2.0 reviewed case by case. GPL, AGPL and LGPL denied. The same allow-list should apply to npm packages in the UI bundle.
- A third-party notices file is generated, with data-source credits from the provenance tags in [ADR-0012](0012-data-pipeline-and-licensing.md).

**BeforeIT.jl.** Mechanics are specified from the Poledna and Glielmo papers. Check the BeforeIT.jl licence before copying any of its code. Each ADR or spec that borrows a mechanic records its provenance, for example "Taylor rule per Poledna et al. 2023, eq. X".

**Real firm names.** Named firms live in the `firms` data file, never in code, so they can be swapped for fictional names ([ADR-0016](0016-firm-representation.md)).

**Telemetry.** None planned.

## Consequences
- Easier: no distribution work or fees; no licence surprises in the dependency tree; real names can be removed in one data change.
- Harder: mechanics inspired by BeforeIT take longer to specify from papers than to port.
- Revisit distribution if the project is ever published or sold; revisit the BeforeIT rule once its licence is checked.

## Open questions / to verify
- [ ] Optional: check the BeforeIT.jl licence before copying any code (reading papers is fine).
- [ ] Which tool checks npm licences against the allow-list (not covered by the research).
