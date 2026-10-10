---
id: economy/overview
title: Economy — System Overview
status: draft
owner: horia
depends_on: [vision/pillars, society/overview]
updated: 2026-10-10
---

# Economy — System Overview

## Sectors (who holds stocks and makes decisions)

| Sector | Represented as | Decides | Main balance sheet items |
|---|---|---|---|
| Households | the **synthetic households and persons** ([population model](../society/population-groups.md)) | consume, save, borrow, supply labour, study, migrate, send/receive remittances | deposits, bonds, equity, housing (A); mortgages, consumer and student debt (L) |
| Firms | **firm units** per **industry** (~90 industries, ~550–700 units): 0–5 named real firms plus size-class cohort firms with an integer firm count ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)); each unit has an ownership vector (state, domestic private, foreign) and a controller | produce, price, buy inputs, hire, invest, borrow, pay dividends | capital, inventories, deposits (A); loans, equity (L) |
| Banks | aggregate commercial banking sector (v1) | lend, set loan and deposit rates | loans, bonds, reserves (A); deposits, CB advances, equity (L) |
| Government | one sector (ministries are budget lines) | tax, spend, transfer, invest, borrow, own firms | deposits at CB, SOE equity, public capital (A); bonds (L) |
| Central bank | one sector (modelled on the BNR), player-controlled in v1 | policy rate, reserves, QE, FX intervention | bonds, FX reserves, advances (A); reserves, cash, gov deposits (L) |
| Rest of world | two blocs: **EU** (incl. EU institutions: contributions, funds, rules) and **non-EU**; plus the Romanian diaspora | demand for exports, supply of imports, capital flows, remittances | foreign assets and liabilities |

## Mechanic map

```
                  levers ─► taxation · social-transfers · fiscal-policy · monetary-policy · trade-fx
                                 │              │              │               │             │
                                 ▼              ▼              ▼               ▼             ▼
  demographics ─► PEOPLE (households) ◄── wages ── labor-market ◄── hiring ── production ◄── industries (I-O supply chain)
        ▲            │  consumption by good                             ▲      │  ▲          ▲
        │            ▼                                                   │      │  │          │
   education    goods demand ──────────────────────────────────────────►┘      │  └── energy ┘
   & social          │                                                          │       ▲
   outcomes          ▼                                                          ▼       │
        ▲      prices-inflation ◄── unit costs, demand pressure, import prices ─┘   infrastructure
        │            │                                                                  ▲
   opinion-     expectations ─► monetary-policy ─► money-banking (rates, credit) ─► investment-capital
   approval                                │
                                           └─► trade-fx (exchange rate, capital flows) ─► exports/imports
```
Accounting ([accounting](accounting.md)) sits under everything: every arrow that moves money is a row in the transaction matrix.

## Mechanics (one spec each)

| Spec | Owns |
|---|---|
| [accounting](accounting.md) | Balance sheets and transaction matrix: the consistency backbone |
| [industries](industries.md) | The v1 industry and good catalogue, input-output structure |
| [production](production.md) | Production functions, capacity, output decisions, supply chains |
| [energy](energy.md) | Power plants by technology, merit order, wholesale and retail electricity prices, fuels |
| [infrastructure](infrastructure.md) | Public capital, the project system, productivity effects |
| [state-enterprises](state-enterprises.md) | Nationalisation, privatisation, SOE behaviour |
| [labor-market](labor-market.md) | Hiring and matching by skill and region, wages, minimum wage |
| [households](households.md) | Household budgets: income, consumption baskets, saving, borrowing |
| [prices-inflation](prices-inflation.md) | Price setting per good, CPI, expectations |
| [investment-capital](investment-capital.md) | Firm investment, capital stock, depreciation, FDI |
| [money-banking](money-banking.md) | Credit creation, interest rates, bank solvency |
| [monetary-policy](monetary-policy.md) | Central bank rule and tools |
| [taxation](taxation.md) | Every tax: base, rates, brackets, compliance |
| [vat](vat.md) | VAT itself: rates, categories, rounding, the ledger posting |
| [social-transfers](social-transfers.md) | Pensions, benefits, student finance, charity drives |
| [fiscal-policy](fiscal-policy.md) | Budget, spending, subsidies, deficit, debt, bond market |
| [trade-fx](trade-fx.md) | Exports, imports, tariffs, exchange rate, capital flows |
| [demographics](demographics.md) | Births, deaths, migration, diaspora |
| [eu-membership](eu-membership.md) | Single market, Schengen, free movement, state aid, fiscal rules, EU budget contribution |
| [euro](euro.md) | Joining (ERM II, convergence criteria, conversion) and leaving the euro area |
| [eu-funds](eu-funds.md) | Cohesion, CAP and recovery funds; project pipeline and absorption |
| [informal-economy](informal-economy.md) | Undeclared work, envelope wages, VAT gap, enforcement |
| [foreign-ownership](foreign-ownership.md) | Foreign-owned firms, FDI decisions, profit repatriation and shifting |
| [housing](housing.md) | House prices and rents by county, construction, vacancies, mortgages |
| [environment](environment.md) | Emissions, EU ETS, air pollution, weather shocks |
| [state-capacity](state-capacity.md) | Administrative capacity, corruption, rule of law, spending efficiency |

## Core open decisions
These need research and an ADR before specs can move past `draft`:
- [x] **Modelling approach:** hybrid SFC backbone + synthetic population + I-O industries, accepted 2026-10-09. See [ADR-0002](../../03-architecture/decisions/0002-modelling-approach.md) and [ADR-0003](../../03-architecture/decisions/0003-people-representation.md).
- [x] **Firm representation:** named firms + size-class cohort firms, with ownership vectors ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md), accepted 2026-10-09).
- [x] **Tick length:** one day, with each process at its own period (owner decision 2026-10-10; ADR to follow, see D5 in the [roadmap](../roadmap.md)).
- [ ] **Industry list:** 90 proposed in [industries](industries.md) (electricity split by technology, 2026-10-09); reconcile with P&R 2026.
- [ ] **Expectations:** anchored-adaptive proposed in [prices-inflation](prices-inflation.md).
- [ ] **Market clearing:** inventory-buffer disequilibrium (proposed) vs per-tick price equilibrium.
