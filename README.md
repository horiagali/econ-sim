# econ-sim (working title)

A country-management game in the spirit of *Power & Revolution*, built around a **realistic, internally consistent economic simulation**. You run a country; every policy lever moves real-world-shaped mechanisms — prices, wages, credit, taxes, trade, debt — and the consequences propagate through the whole system.

Economics comes first. Politics and military are planned and will plug into the same simulation core.

## Status

**Phase 0 — Design & research.** No code yet. We define mechanisms precisely on paper before anything gets built.

## How this repo works

This project is built primarily with AI coding agents. The docs are written so that an agent (or a human) can pick up any mechanic, understand exactly what it does, what it depends on and how to test it — without reading the whole repo.

| Where | What |
|---|---|
| [`AGENTS.md`](AGENTS.md) | Rules for AI agents working in this repo. Read first. |
| [`docs/INDEX.md`](docs/INDEX.md) | Auto-generated map of every doc, its status and dependencies. |
| [`docs/00-vision/`](docs/00-vision/) | What the game is, design pillars, scope, inspirations. |
| [`docs/01-research/`](docs/01-research/) | Research notes: real-world economics, models, data sources. |
| [`docs/02-design/`](docs/02-design/) | Mechanic specs — the source of truth for what gets coded. |
| [`docs/03-architecture/`](docs/03-architecture/) | Technical architecture and decision records (ADRs). |
| [`docs/glossary.md`](docs/glossary.md) | Canonical names, symbols and units for every variable. |
| [`docs/templates/`](docs/templates/) | Templates for mechanics, research notes and ADRs. |
| [`docs/roadmap.md`](docs/roadmap.md) | Phases and milestones. |

## Workflow

```
research question → research note → mechanic spec (draft) → review → spec locked → implement → test against spec
```

Every doc has YAML frontmatter (`status`, `depends_on`, …). Run `python scripts/docs_index.py` to validate frontmatter and regenerate `docs/INDEX.md`. CI checks this on every push.
