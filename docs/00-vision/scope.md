---
id: vision/scope
title: Scope
status: draft
owner: horia
depends_on: [vision/vision]
updated: 2026-10-08
---

# Scope

## MVP (economy only)
- One player-controlled country; rest of the world as an aggregate trading partner.
- Sectors: households, firms (a few industries), banks, government, central bank, rest of world.
- Time step: 1 tick = 1 month (to be confirmed by [ADR](../03-architecture/decisions/)).
- Player levers: tax rates, spending categories, policy rate, minimum wage, tariffs, borrowing.
- Outputs: GDP, inflation, unemployment, wages, public debt, deficit, trade balance, exchange rate, inequality.
- A causal "why did this change" panel.

## Later
- Multiple simulated countries trading with each other.
- Regions within a country.
- Politics: factions, elections, approval, legislation, stability.
- Military: budget, readiness, arms industry, conflicts and their economic costs.

## Explicitly out
- Multiplayer.
- Real-time tactical combat.
