---
id: vision/vision
title: Vision
status: draft
owner: horia
depends_on: []
updated: 2026-10-09
---

# Vision

## Elevator pitch
*This is a personal, non-commercial project, not a product for sale.*

You are the head of state of Romania. You set tax brackets, budgets, pensions, interest rates, subsidies, trade policy; you build power plants and railways, nationalise or privatise companies. A living economy and a living population respond. They don't respond through scripted events and opaque modifiers. They respond through mechanisms that mirror how real economies work. Households spend and save, firms buy inputs, hire and invest, banks lend, prices move along supply chains, the currency floats, debt piles up.

The population is made of concrete people, simulated as a weighted synthetic population with jobs, incomes, families, education, culture, religion, ideology and interests. Any slice of it is a group you can look at, for example *"5,000 well-educated students in Cluj"*. Every policy lands on specific people. Their finances actually change, their behaviour changes, and that feeds back into the economy and into how much they like you.

## The core promise: everything feeds into everything
Illustrative chains the simulation must produce *by mechanism*, not by script:

- **Student finance law** → students' disposable income rises → they consume more (rent, food, electronics) → demand for those industries rises → more jobs and output. Over the years, more people finish higher education → more skilled workers → higher productivity and wages, lower crime. Students' approval rises. Taxpayers who fund it may like it less.
- **New power plant** → electricity supply rises → electricity price falls → costs fall for every industry in proportion to its energy use → slight disinflation; energy-hungry industries (data centres, steel, chemicals) become more profitable → they invest and expand.
- **Car-parts price rises** (e.g. a tariff on imported parts) → unit cost of car makers rises → car prices rise → domestic car demand and car exports fall → car-plant output and hiring fall.
- **Currency devalues** → exports cheaper abroad, imports dearer at home → export industries boom, import-dependent industries and consumers suffer → imported inflation.
- **Central bank raises rates** → loans dearer, saving more attractive → consumption and investment cool → unemployment drifts up, inflation falls with a lag → borrowers lose and savers win.

## Game type
- **Sandbox.** No win or lose condition in v1. You set your own goals (growth, equality, low debt, high approval…). Scenarios with goals may come later.
- **One country in v1: Romania**, recreated from real data (counties, regions, population, industries, tax system, central bank, EU membership, diaspora). Other countries may follow later via data files.
- GDP can grow without limit. Population can grow and shrink, get richer or poorer.

## What "better than Power & Revolution" means
See [inspirations](inspirations.md). In short:
- **Not buggy.** A deterministic, tested, internally consistent simulation. No exploding numbers, no money appearing from nowhere, no nonsense states.
- **Close to real life.** Mechanisms are grounded in economics research ([research](../01-research/README.md)). Qualitative behaviour matches reality.
- **Causality you can see.** Every indicator can be drilled into to show what moved it and why.
- **Consistent accounting.** Every unit of money is someone's asset and someone else's liability. Every person is counted exactly once.
- **Emergent crises.** Inflation spirals, debt crises and recessions come out of the system, not out of dice rolls.

## Target experience
- You feel like a head of state with full executive power over the economy. In v1 there's no parliament to appease. You are also finance minister and, if you choose, controller of the central bank.
- Short-term pressure (budget, prices, approval) vs long-term health (education, productivity, infrastructure, debt sustainability).
- Lots of graphs and drill-downs. Learning the game teaches real economic intuition.

## Non-goals (v1)
- Politics as a system (parliament, parties, elections, legislation votes). Approval exists, but it is an output, not a gate.
- Military.
- Decimal-exact reproduction of Romania's statistics (we aim for the right shape and magnitudes).
- Multiplayer.
