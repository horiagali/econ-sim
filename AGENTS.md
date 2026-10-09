# AGENTS.md — instructions for AI agents

This file is the entry point for any AI agent (Claude, Codex, Cursor, …) working in this repo. `CLAUDE.md` points here.

## Project in one paragraph

A country-management sandbox focused on a realistic economic simulation. The player is head of state of **Romania** (recreated from real data) and sets policy (tax brackets, spending, transfers, subsidies, interest rates, trade, nationalisation, infrastructure and power plants, …). The simulation propagates effects through a **weighted synthetic population** of households and persons (the most important system), ~90 industries linked by supply chains, banks, government, central bank and the rest of the world. v1 is economy + people only: no politics layer, no military. Phase 0 is **design and research only — do not write game code unless a task explicitly asks for it.**

## Before you start any task

1. Read `docs/INDEX.md` to see what exists and its status.
2. Read only the docs relevant to your task plus their `depends_on` docs. Don't load the whole repo.
3. Check `docs/glossary.md` for variable names. **Never invent a new name for something that already has one.** If you need a new variable, add it to the glossary in the same change.

## Tech stack (accepted)

See [ADR-0004](docs/03-architecture/decisions/0004-tech-stack-overview.md): Rust simulation core, Tauri 2 + React/TypeScript UI, Python data and calibration tooling. Once code exists, these rules are hard:
- **Determinism contract** ([ADR-0006](docs/03-architecture/decisions/0006-determinism-contract.md)): keyed RNG only, `libm` maths only, no `HashMap`/`HashSet`, fixed-order sums.
- **Money moves only through the ledger** ([ADR-0007](docs/03-architecture/decisions/0007-money-and-ledger.md)): `Bani(i64)`, typed transfers with a flow code; never write a balance directly.
- **Behaviour rules use `behaviour_rule!`** ([ADR-0008](docs/03-architecture/decisions/0008-explainability-architecture.md)); opaque rules need a counted waiver.
- **Never hardcode the population scale** ([ADR-0003](docs/03-architecture/decisions/0003-people-representation.md)): read `sample_scale` and per-record weights from the scenario; no population-sized constants.

## Doc conventions

- **One mechanic per file.** Keep files focused; split if a spec exceeds ~400 lines.
- **Frontmatter is mandatory** (see `docs/templates/`). Fields: `id`, `title`, `status`, `owner`, `depends_on`, `updated`.
- **Status lifecycle:** `stub` → `researching` → `draft` → `review` → `locked`. Only a human moves a doc to `locked`. A `locked` spec is what code is built against; changing it needs an ADR or an explicit human request.
- **Equations** go in fenced `math` blocks or inline LaTeX, using glossary symbols. State units and the time step.
- **Every mechanic must list:** state variables, inputs, outputs, update rule, player levers, tuning parameters, edge cases, and acceptance tests (observable behaviours we can check in a running sim).
- **Separate fact from design.** Real-world evidence lives in `01-research/` with sources. Game design choices live in `02-design/` and link to the research that justifies them. Mark simplifications explicitly as `> **Simplification:** …`.
- **Open questions** go in an `## Open questions` section as a checklist, not buried in prose.
- **Cite sources** in research notes with links. Rate confidence: `high` / `medium` / `low`.

## After you change docs

1. Update `updated:` in the frontmatter.
2. Run `python scripts/docs_index.py` — it validates frontmatter, checks `depends_on` targets exist, and regenerates `docs/INDEX.md`.
3. Commit with a message like `docs(economy/labor-market): draft wage-setting rule`.

## Design principles to respect

See `docs/00-vision/pillars.md`. The short version:
- **Stock-flow consistency** — money never appears or disappears without an accounting entry.
- **Explainability** — every number the player sees can be traced to its causes.
- **Plausible over precise** — qualitatively right behaviour beats calibrated decimals.
- **Systems over scripts** — outcomes emerge from mechanisms, not hard-coded events.

## Don't

- Don't write implementation code during Phase 0 (prototype snippets inside research notes are fine).
- Don't edit `locked` specs without being asked.
- Don't create new top-level folders without an ADR.
