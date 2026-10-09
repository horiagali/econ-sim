---
id: adr/0013-ui-stack
title: "ADR-0013: UI stack"
status: accepted
owner: horia
depends_on: [adr/0004-tech-stack-overview, adr/0008-explainability-architecture]
updated: 2026-10-09
---

# ADR-0013: UI stack

## Context
The hard UI work in this game is business-app work: dozens of time-series charts, a group explorer with filters and cross-tabs, about 150 lever forms, a 42-county map and nested "why did this change" breakdowns. Most of it will be written by AI agents. The data volume is small: 50 charts × 5 series × 600 months is 150k points.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (section "The UI is Tauri 2 with React, uPlot and ECharts") found:
- JavaScript (66%), TypeScript (43.6%) and React (44.7%) dwarf GDScript (3.3%) in Stack Overflow's 2025 survey, so agents have far more training data and ready-made components for a web UI.
- Tauri 2 has been stable since October 2024, uses WebView2 on Windows, and its host process is Rust, so the core links in-process as a normal dependency. Default JSON command returns are slow for large data; `tauri::ipc::Response` returns binary and `Channel` streams ordered updates.
- uPlot draws 166,650 points in 25 ms from a cold start; Plotly took 310 ms. ECharts handles maps, heatmaps, sankey, waterfall and treemap views.
- egui is quick for developer tools, but its customisation is "not yet as powerful as CSS" and releases break APIs.
- Godot, Unity and Bevy make every chart, table and form custom work; Bevy shipped four breaking releases between April 2025 and June 2026.
- No commercial Tauri game appears in the awesome-tauri list; Electron has a proven Steam path (steamworks.js overlay helper).
- geoBoundaries is CC BY 4.0 with commercial use allowed. GADM forbids redistribution and commercial use without permission. Eurostat GISCO terms could not be fetched.

## Options considered
1. **Tauri 2 + React/TypeScript, uPlot + ECharts, core in-process.**
   Pros: richest component ecosystem; small installer (WebView2); no FFI or separate server.
   Cons: Steam overlay over WebView2 unverified; two languages.
2. **Electron + React.**
   Pros: proven on Steam.
   Cons: ~100+ MB runtime; the core needs a Node binding or a sidecar process.
3. **Game engine (Godot, Unity, Bevy).**
   Pros: game feel, built-in rendering.
   Cons: charts, tables and forms all custom; fewer agent-friendly components; Bevy API churn.
4. **egui/ImGui for the player UI.**
   Pros: pure Rust, fast to build.
   Cons: limited styling, breaking releases; poor fit for a polished player UI.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1 for the player UI, plus an egui developer inspector.**

**Player UI**
- Tauri 2 desktop shell with a React/TypeScript front end. The simulation crate is an ordinary dependency of the Tauri host.
- Chart data travel as binary arrays through `tauri::ipc::Response`; per-tick updates stream over a `Channel`.
- **uPlot** for every time series. **ECharts** for the county choropleth and the waterfall and treemap views of the "why" panel. No Plotly.
- All calls to the host go through a thin `api.ts` adapter, so a move to Electron stays cheap.
- The "why" panel follows Victoria 3's nested tooltips: it renders `ContributionTree`s from `explain()` ([ADR-0008](0008-explainability-architecture.md)).
- The group explorer always shows how many synthetic records sit behind a view, and flags views built on too few records ([ADR-0003](0003-people-representation.md)).

**Developer inspector.** A separate, developer-only egui/eframe app links the core directly and shows the ledger, invariants, clamp counters and calibration state. It is never shipped to players.

**Map data.** geoBoundaries ADM1 for Romania (CC BY 4.0) is preferred, pre-projected in the data pipeline, with "geoBoundaries" in the credits. Because this is a personal, non-commercial project, **GADM may be used if more convenient** (its terms allow non-commercial use), as may Eurostat GISCO NUTS boundaries; the choice is recorded in the provenance tags ([ADR-0012](0012-data-pipeline-and-licensing.md)). GADM would have to be replaced if the project ever became commercial.

## Consequences
- Easier: agents build charts, tables and forms from standard libraries; binary IPC keeps chart updates fast.
- Easier: the developer inspector can be built early, before the player UI, to debug the core.
- Tauri on Steam is unproven (Steam overlay over WebView2); irrelevant while distribution is deferred ([ADR-0015](0015-distribution-and-licensing.md)).
- Harder: front-end types must stay in sync with the core; the glossary codegen emits TypeScript types ([ADR-0012](0012-data-pipeline-and-licensing.md)).
- Revisit if WebView2 fails (or, if the game is ever published, the Steam overlay); then move to Electron behind the same `api.ts` adapter.

## Open questions / to verify
- [ ] That geoBoundaries' Romania ADM1 layer is the CC BY "gbOpen" release.
- [ ] Spike 8 exit criterion: IPC round trip for 1–5 MB measured on WebView2; cold start and memory acceptable on the owner's laptop.
