---
id: adr/0014-agent-workflow-guardrails
title: "ADR-0014: AI-agent workflow and repo guardrails"
status: accepted
owner: horia
depends_on: [adr/0001-docs-first-ai-driven, research/tech-stack-deep-research]
updated: 2026-10-10
---

# ADR-0014: AI-agent workflow and repo guardrails

## Context
[ADR-0001](0001-docs-first-ai-driven.md) makes development docs-first and agent-driven. Agents read instruction files selectively, can be talked into weakening tests, and write tests that pass whatever the code does. The repo is private on GitHub Free, and the owner is the only human reviewer.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (section "The agent workflow puts every hard rule in a hook or a CI check") found:
- **Claude Code reads AGENTS.md only when no CLAUDE.md exists.** The documented way to share one file is a CLAUDE.md that begins with the literal line `@AGENTS.md`. Instruction files should stay under about 200 lines; Codex stops reading once instruction files reach 32 KiB combined.
- The closest AGENTS.md to an edited file wins, so area rules can live in nested files.
- Anthropic's guidance: CLAUDE.md is advisory, while hooks are deterministic. Give the agent a check it can run; use separate sessions to write tests and code, and a fresh-context reviewer.
- **GitHub Free does not enforce CODEOWNERS or rulesets on private repos** (they need Pro, Team or Enterprise). As sole owner, a code-owner review rule would also block the owner's own PRs.

## Options considered
**Instruction files**
1. **Root AGENTS.md as a short router; CLAUDE.md imports it with `@AGENTS.md`; nested AGENTS.md per area.** One source of truth; works for Claude Code and Codex.
2. **One big instruction file.** Simple; exceeds size limits and lowers adherence.
3. **CLAUDE.md as a pointer link (current state).** Claude Code does not follow it automatically.

**Test protection on GitHub Free**
1. **Four layers of hooks and CI checks on Free.** No cost; red checks are signals the owner must respect.
2. **Upgrade to GitHub Pro.** Required status checks and +1,000 minutes; costs money; CODEOWNERS still awkward for a sole owner.
3. **Make the repo public.** Free standard runners and rulesets; exposes the source, and raw restricted microdata must never sit in a public repo ([ADR-0012](0012-data-pipeline-and-licensing.md)).
4. **Rules in prose only.** Free; agents bypass them.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1 for instruction files, and option 1 (four layers on Free) for test protection.** Upgrading to Pro is the fallback.

**Instruction files**
- Root `CLAUDE.md` starts with the literal line `@AGENTS.md`, followed only by Claude-specific notes. (Applied on 2026-10-09.)
- Root `AGENTS.md` is a router under about 200 lines: where things live, the five or six `just` commands, the hard invariants, and "read `docs/02-design/...` before implementing".
- Nested AGENTS.md files under `crates/`, `python/` and `ui/` hold area rules. `.claude/rules/*.md` with path globs hold rules for some files only; skills hold occasional workflows.

**Four protection layers**
1. A **PreToolUse hook** denies agent edits to `tests/golden/**`, `crates/*/tests/acceptance/**` and `schema/**` unless the session is in test-authoring mode.
2. A CI job fails any PR that touches both rule code and its acceptance tests, unless it carries a label the owner adds by hand.
3. A CI check fails if test or assertion counts fall, or if `#[ignore]` or skip markers appear.
4. ~~Agent permissions deny `gh pr merge`; only the owner merges.~~ **Changed by the owner on 2026-10-10:** the deny rule was removed from `.claude/settings.json`. An agent may merge a pull request when the owner tells it to and CI is green; it still never merges on its own initiative, and `git push --force` stays denied.

On Free a red check is only a signal: the owner refuses to merge it.

**Tests-first two-PR flow**
1. Each spec acceptance criterion gets an ID such as `AC-VAT-03`, with a type tag.
2. A script generates ignored test stubs from the IDs.
3. A test-writer session fills the stubs; the owner merges that PR.
4. The implementation PR, written in a separate session, may not touch those tests.
5. An extension of `scripts/docs_index.py` fails CI when a `locked` spec's criterion has no live test (spec-ID traceability).
6. Any golden snapshot change needs a `CHANGELOG-sim.md` entry with the economic reason ([ADR-0010](0010-verification-and-testing.md)).
7. A fresh-context reviewer session reviews each implementation PR.

**Rule escalation.** When agents bypass a prose rule more than once, it becomes a hook or a CI check.

## Consequences
- Easier: rules hold regardless of which agent or session runs; protected tests cannot be quietly weakened.
- Easier: every acceptance criterion in a locked spec is traceable to a live test.
- Harder: two PRs per mechanic; more owner merges; hooks and CI scripts to maintain.
- Harder: on Free, enforcement depends on the owner not merging red PRs.
- Revisit if minutes run out or real enforcement is needed: upgrade to Pro, or reconsider a public repo (owner's call; restricted raw data would have to stay outside it).

## Open questions / to verify
- [x] Owner decision: GitHub Free with hooks + CI checks (2026-10-09).
- [ ] How "test-authoring mode" is signalled to the hook (environment variable, session flag or branch name).
- [ ] Spike 0 exit criterion: an agent session on Windows runs `just check` green and is blocked from editing `tests/golden/`.
- [ ] Spike 9 exit criterion: one real mechanic (e.g. VAT) goes spec → tests → implementation with no hand edits to protected paths.

## Amendment 1 (2026-10-10): a spec may be locked before its tests
Step 5 failed CI for any criterion of a `locked` spec without a live test. The first specs the owner wanted to lock (population-groups, accounting) have criteria that need a running simulation, so they could not be locked at the point where locking is useful: before tests and code are written against them.

Owner decision: a spec may be locked first. Each criterion without a live test must then be listed in the spec under a "Tests owed" heading, one line per criterion with what it waits for (`- AC-XXX-NN: reason`). `scripts/traceability.py` fails when a criterion of a locked spec has neither a live test nor such a line, when a listed criterion has a live test (the line must be removed), or when a line names a criterion the spec does not define. The debt is therefore visible in the spec and in the output of `just trace`, and cannot grow silently. Everything else in this ADR is unchanged: tests are still written in a test-authoring session, before the code.
