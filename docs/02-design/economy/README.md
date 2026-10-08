---
id: economy/overview
title: Economy — System Overview
status: draft
owner: horia
depends_on: [vision/pillars]
updated: 2026-10-08
---

# Economy — System Overview

## Sectors (who holds stocks and makes decisions)

| Sector | Decides | Main balance sheet items |
|---|---|---|
| Households | consume, save, supply labour, hold assets | deposits, bonds, equity, housing, loans |
| Firms (by industry) | produce, price, hire, invest, borrow | capital, inventories, loans, equity |
| Banks | lend, set loan/deposit rates | loans (A), deposits (L), reserves (A), central bank advances (L) |
| Government | tax, spend, transfer, borrow | bonds (L), deposits at central bank (A) |
| Central bank | policy rate, reserves, QE, FX intervention | bonds (A), FX reserves (A), reserves (L), cash (L) |
| Rest of world | imports, exports, capital flows | foreign assets/liabilities |

## Mechanic map

```
                ┌────────────── fiscal-policy ───────────────┐
                │  taxes, spending, transfers, debt           │
                ▼                                             ▲
 households ◄─ labor-market ─► production ─► prices-inflation ─┤
   │  ▲                         ▲    │            │           │
   ▼  │                         │    ▼            ▼           │
 consumption ──── demand ───────┘  investment   monetary-policy (central bank)
   │                                  │            │
   ▼                                  ▼            ▼
 trade-fx ◄────────────────────── money-banking (credit, rates)
```

## Mechanics (one spec each)

| Spec | Owns |
|---|---|
| [accounting](accounting.md) | Balance sheets & transaction matrix — the consistency backbone |
| [production](production.md) | Output, productivity, capacity, industries |
| [labor-market](labor-market.md) | Employment, unemployment, wages |
| [households](households.md) | Consumption, saving, income distribution |
| [prices-inflation](prices-inflation.md) | Price setting, inflation, expectations |
| [investment-capital](investment-capital.md) | Firm investment, capital stock, depreciation |
| [money-banking](money-banking.md) | Credit creation, interest rates, bank solvency |
| [monetary-policy](monetary-policy.md) | Central bank rules and tools |
| [fiscal-policy](fiscal-policy.md) | Taxes, spending, deficits, public debt |
| [trade-fx](trade-fx.md) | Exports, imports, exchange rate, capital flows |
| [demographics](demographics.md) | Population, labour force, ageing |

## Core open decisions
These need research + an ADR before specs can move past `draft`:
- [ ] **Modelling approach:** aggregate SFC model vs agent-based vs hybrid (aggregate sectors with distributional cohorts). → [research](../../01-research/README.md)
- [ ] **Tick length:** month vs quarter.
- [ ] **Number of industries** in MVP (3? 6? input-output based?).
- [ ] **Expectations:** adaptive vs anchored-adaptive vs something richer.
