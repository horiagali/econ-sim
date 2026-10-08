---
id: adr/0001-docs-first-ai-driven
title: "ADR-0001: Docs-first, AI-agent-driven development"
status: accepted
owner: horia
depends_on: []
updated: 2026-10-08
---

# ADR-0001: Docs-first, AI-agent-driven development

## Context
The game will be built largely by AI coding agents. Agents work best with small, precise, well-linked context and explicit acceptance criteria. Economic simulations are easy to get subtly wrong (leaking money, unstable feedback loops).

## Options considered
1. **Code-first prototyping** — fast feedback, but agents drift, mechanics become implicit in code, hard to review.
2. **Docs-first specs with structured frontmatter** — slower start, but every mechanic is reviewable, testable, and loadable by an agent in isolation.

## Decision
Docs-first. Each mechanic is specified in its own file using a fixed template, with frontmatter (`status`, `depends_on`), glossary-controlled variable names, and acceptance tests. Code is only written against specs at `locked` status. `AGENTS.md` holds agent rules; `scripts/docs_index.py` validates and indexes docs.

## Consequences
- Easier: parallel agent work, reviews, test generation from specs, onboarding.
- Harder: upfront writing effort; specs must be kept in sync with code (enforced by CI later).
