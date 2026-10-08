---
id: glossary
title: Glossary & Variable Registry
status: draft
owner: horia
depends_on: []
updated: 2026-10-08
---

# Glossary & Variable Registry

**Single source of truth for names.** Specs and code use these symbols and `code_name`s. Add new variables here in the same change that introduces them.

Conventions: subscript `t` = tick; stocks are end-of-tick; flows are per tick; money amounts are in nominal local currency unless marked `real`.

| Symbol | code_name | Meaning | Unit | Type | Owner spec |
|---|---|---|---|---|---|
| $Y$ | `gdp_real` | Real output | real LCU / tick | flow | economy/production |
| $P$ | `price_level` | Consumer price index | index (base = 1) | state | economy/prices-inflation |
| $\pi$ | `inflation` | Price inflation, annualised | % / year | derived | economy/prices-inflation |
| $N$ | `employment` | Employed persons | persons | state | economy/labor-market |
| $L$ | `labor_force` | Labour force | persons | state | economy/demographics |
| $u$ | `unemployment_rate` | $1 - N/L$ | ratio | derived | economy/labor-market |
| $W$ | `wage_nominal` | Average nominal wage | LCU / worker / tick | state | economy/labor-market |
| $C$ | `consumption` | Household consumption | LCU / tick | flow | economy/households |
| $I$ | `investment` | Gross fixed investment | LCU / tick | flow | economy/investment-capital |
| $K$ | `capital_stock` | Capital stock | real LCU | state | economy/investment-capital |
| $G$ | `gov_spending` | Government purchases | LCU / tick | flow | economy/fiscal-policy |
| $T$ | `tax_revenue` | Total tax revenue | LCU / tick | flow | economy/fiscal-policy |
| $B$ | `gov_debt` | Government bonds outstanding | LCU | state | economy/fiscal-policy |
| $i$ | `policy_rate` | Central bank policy rate | % / year | lever | economy/monetary-policy |
| $e$ | `exchange_rate` | LCU per unit foreign currency | ratio | state | economy/trade-fx |
| $X$ | `exports` | Exports | LCU / tick | flow | economy/trade-fx |
| $M$ | `imports` | Imports | LCU / tick | flow | economy/trade-fx |

## Terms
- **SFC (stock-flow consistent):** modelling approach where every flow changes a stock and all balance sheets reconcile.
- **Tick:** one simulation step.
- **Lever:** a variable directly set by the player.
