---
id: domain/mechanic-name
title: Mechanic Name
status: stub            # stub | researching | draft | review | locked
owner: horia
depends_on: []          # ids of other docs, e.g. [economy/money-banking]
research: []            # ids of research notes backing this spec
updated: YYYY-MM-DD
---

# Mechanic Name

## Purpose
What this mechanic does in one or two sentences, and why the game needs it.

## Real-world basis
How this works in actual economies. Link research notes. Note which school/model we follow.

## State variables
| Symbol | Name | Unit | Range | Notes |
|---|---|---|---|---|
| | | | | |

## Inputs
Variables read from other mechanics (with the mechanic that owns them).

## Outputs
Variables this mechanic writes that others read.

## Update rule
Per tick. Equations using glossary symbols.

```math
X_{t+1} = ...
```

> **Simplification:** anything we knowingly deviate from reality on, and why.

## Player levers
What the player can change, allowed ranges, and lag before effects appear.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|

## Interactions
How this mechanic feeds into and is fed by others. A small diagram is welcome.

## Edge cases & failure modes
Hyperinflation, zero bound, default, collapse, division by zero, negative stocks…

## Acceptance tests
Observable behaviours a running simulation must show. Written so they can become automated tests.
- [ ] Given …, when …, then … within N ticks.

## Open questions
- [ ] 
