---
id: game/levers
title: Player Levers — Catalogue
status: draft
owner: horia
depends_on: [game/overview]
research: []
updated: 2026-10-10
---

# Player Levers — Catalogue

**Single list of everything the head of state can do in v1.** The spec that owns each lever defines its exact effect; this page is the menu. When a spec adds or removes a lever, update this table in the same change.

Lag = time from decision to first effect. A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)). In the tables below, written before that decision, "next tick" means **the first day of the following month** for law (taxes, benefits, budgets, regulation) and **the next day** for market operations of the central bank (policy rate, QE, monetary financing, FX intervention); "N ticks" means N months.

Changing a lever costs nothing in v1 (owner decision 2026-10-10); a cost for frequent reversals may come later.

## Taxation ([taxation](../economy/taxation.md))
| Lever | Form | Range | Lag |
|---|---|---|---|
| Personal income tax brackets | list of (threshold, marginal rate); add or remove brackets | 0–90% per bracket | next tick |
| Tax-free allowance | amount per person per year | ≥ 0 | next tick |
| Tax credits | per child, per student, low-income credit | amount | next tick |
| Capital income tax | rate on interest and dividends (or taxed as income) | 0–60% | next tick |
| Corporate profit tax | rate; optional reduced rate per industry | 0–60% | next tick |
| VAT ([vat](../economy/vat.md)) | standard rate + two reduced rates + per-good-category assignment (standard / reduced / second reduced / zero / exempt) | 0–40% | next tick |
| Payroll and social contributions | employee and employer rates; ceiling | 0–40% | next tick |
| Property tax | rate on housing and commercial property value | 0–3%/yr | next tick |
| Wealth tax | threshold + rate | 0–5%/yr | next tick |
| Excise duties | per good: fuel, tobacco and alcohol, carbon | amount per unit | next tick |
| Tariffs | see trade (locked by EU membership) | | |

## Spending and public services ([fiscal policy](../economy/fiscal-policy.md))
| Lever | Form | Lag |
|---|---|---|
| Budget per ministry | education, health, police and justice, infrastructure maintenance, research, culture, environment, administration | next tick (effects build over months or years) |
| Public sector wages | % vs private equivalent | next tick |
| Public sector headcount targets | per ministry | hiring over months |
| Government borrowing | issue bonds or bills; maturity mix | next tick |

## Social transfers ([social transfers](../economy/social-transfers.md))
| Lever | Form | Lag |
|---|---|---|
| State pension | retirement age; benefit formula (flat + earnings-related replacement rate); indexation rule | 1–3 ticks; retirement-age changes phased |
| Unemployment benefit | replacement rate, cap, duration | 1–2 ticks |
| Child benefit | amount per child, income-tested or universal | 1–2 ticks |
| Student finance | grant per student, loan per student, loan interest and repayment threshold, means test | next academic year (or 1–2 ticks for top-ups) |
| Minimum income / social assistance | guaranteed floor, withdrawal rate | 1–3 ticks |
| Housing benefit | share of rent covered for low incomes | 1–3 ticks |
| Charity drive for the poor | launch a campaign: target group, duration, government match rate | 1 tick |

## Labour and markets ([labor market](../economy/labor-market.md), [prices](../economy/prices-inflation.md))
| Lever | Form | Lag |
|---|---|---|
| Minimum wage | amount per hour or month; or indexed to median wage | next tick |
| Price caps | per good (e.g. energy, bread, rent); cap level | next tick; shortages emerge |
| Immigration policy (non-EU) | annual quota per skill level; open / selective / closed. EU citizens move freely | months |

## Industry and ownership ([state enterprises](../economy/state-enterprises.md), [fiscal policy](../economy/fiscal-policy.md))
| Lever | Form | Lag |
|---|---|---|
| Subsidies | per industry: production subsidy (% of output value or per unit), investment subsidy (% of capex), wage subsidy, or consumer subsidy on a good | next tick |
| Nationalise | take a % stake in a firm unit, a company (all its units) or all units of an industry (compensated at transaction value, or expropriated) | 1–3 ticks |
| Privatise | sell a % stake: direct sale or IPO / share sale on the stock exchange | 1–6 ticks |
| Bail out a named firm | capital injection for equity while it is under administration | 1 tick |
| SOE directives | for state-controlled firm units: profit target vs employment target vs price target | next tick |
| Research grants | per field; raises productivity growth in matching industries | years |

## Projects ([infrastructure](../economy/infrastructure.md), [energy](../economy/energy.md))
| Lever | Form | Lag |
|---|---|---|
| Build infrastructure | roads, rail, ports, airports, broadband, water, public housing, schools, universities, hospitals, police stations; choose region | build time 1–8 years |
| Build power plant | technology (see Energy below); size; region | 1–10 years by technology |
| Close or decommission plant | | months |
| Maintenance budget | % of required maintenance funded | decay over years |

## Energy ([energy](../economy/energy.md))
Each generation technology is its own industry (`power_nuclear`, `power_hydro`, `power_coal`, `power_gas`, `power_wind`, `power_solar`, `power_biomass`, `power_storage`), so every lever below is set **per technology**.

| Lever | Form | Lag |
|---|---|---|
| Generation subsidy | per technology: production subsidy (LCU/MWh) or investment subsidy (% of capex) | next tick; capacity after build time |
| Contracts for difference | per technology: auction volume (MW or MWh) and maximum strike price | auction months; capacity after build time |
| Technology taxes | per technology: windfall or solidarity tax on generator revenue above a threshold | next tick |
| Build plant | state project per technology, size and region; owned by a state-controlled unit | 1–15 years by technology |
| Close plant / phase-out | close a named plant or all of one technology by a date (e.g. coal phase-out) | months |
| Nationalise or privatise | per firm unit or company (e.g. a generator, the grid operator, a supplier) | 1–6 ticks |
| Electricity price caps | retail (household and non-household) and wholesale caps | next tick |
| Consumer energy subsidies | per household group or per kWh | next tick |
| Network tariff | regulated tariff of `power_grid` (via the regulator) | next tick |
| Interconnectors and grid | projects per border or region | years |

## Money and finance ([monetary policy](../economy/monetary-policy.md), [money & banking](../economy/money-banking.md))
| Lever | Form | Lag |
|---|---|---|
| Inflation target | % per year (announced; affects credibility and expectations) | credibility-dependent |
| Policy rate | % per year (disabled inside the euro: ECB rate applies) | next tick for market rates; 6–24 months to inflation |
| Reserve requirement | % of deposits | next tick |
| Bank capital requirement | % of risk-weighted assets | phased |
| Quantitative easing | central bank buys or sells government bonds per month | next tick |
| Monetary financing | central bank directly funds the deficit ("print money") | next tick; credibility hit |
| Deposit insurance | coverage limit | next tick |
| Bank bailout or nationalisation | when a bank is failing | event-driven |

## Trade and currency ([trade & FX](../economy/trade-fx.md))
| Lever | Form | Lag |
|---|---|---|
| Tariffs | **locked** while in the EU (EU common external tariff applies) | — |
| Export promotion | trade missions, export credit insurance (state aid rules apply to subsidies) | months |
| Exchange rate regime | float / managed float / peg to EUR; ERM II and euro via [euro](../economy/euro.md) | next tick |
| FX intervention | buy or sell foreign reserves per month | next tick |
| Capital controls | **not allowed** in the EU | — |

## EU ([eu-membership](../economy/eu-membership.md), [eu-funds](../economy/eu-funds.md))
| Lever | Form | Lag |
|---|---|---|
| Rule compliance | the game flags when a lever breaks EU rules (state aid, VAT floor, fiscal path); the player may proceed and face the procedure | months to years |
| Request fiscal flexibility | longer adjustment path in exchange for reforms and investment | months |
| EU funds priorities | which programmes and project types to prepare; co-financing budget | months to years |
| Recovery-plan milestones | meet or skip reform and investment milestones | per milestone |
| Schengen | leave (border checks back) / rejoin (needs external-border standards + EU decision) | months |
| External border budget | border police and equipment | months |

## Euro ([euro](../economy/euro.md))
| Lever | Form | Lag |
|---|---|---|
| Enter ERM II | choose central rate; then defend the ±15% band | 24+ months in ERM II before adoption |
| Request convergence assessment | checks all criteria | months |
| Adopt the euro | when all criteria pass; conversion on 1 January after a transition | 6–12 months |
| Leave the euro | announce exit; bank holiday and capital controls allowed during exit; new currency floats | immediate crisis, years of aftermath |

## Informal economy ([informal-economy](../economy/informal-economy.md))
| Lever | Form | Lag |
|---|---|---|
| Tax administration & labour inspection budget | amount | months |
| Digitalisation | e-invoicing, electronic receipts, cash payment limit | project / next tick |
| Penalties | fine multiplier | next tick |
| Tax amnesty | one-off | next tick |

## Foreign investment ([foreign-ownership](../economy/foreign-ownership.md))
| Lever | Form | Lag |
|---|---|---|
| Investment incentives | % of capex or tax holiday for new investment (state-aid limits) | months |
| Industrial parks / special zones | project per county | years |
| Negotiate a pending investment | offer incentives for a named project | months |

## Housing ([housing](../economy/housing.md))
| Lever | Form | Lag |
|---|---|---|
| First-home guarantee scheme | state guarantee share, price cap, budget | next tick |
| Reduced VAT on new homes | rate, price cap | next tick |
| Rent control | max annual rent increase | next tick |
| Permit and zoning liberalisation | level | years |
| Public and social housing | projects per county | years |

## Environment ([environment](../economy/environment.md))
| Lever | Form | Lag |
|---|---|---|
| Coal phase-out schedule | close plants by date | months |
| Non-ETS carbon tax | per tonne on fuels | next tick |
| Vehicle emissions tax | per car by emissions | next tick |
| Green subsidies | heat pumps, insulation, rooftop solar, electric cars | next tick |
| Irrigation, flood defence, afforestation | projects | years |
| Logging limits | annual cap | next tick |

## State capacity ([state-capacity](../economy/state-capacity.md))
| Lever | Form | Lag |
|---|---|---|
| Civil service pay & merit reform | pay ratio, reform on/off | years |
| Anti-corruption & justice funding | amount | years |
| Procurement transparency | open-contracting reform | months to years |
| E-government digitalisation | project | years |

## World settings (v1 stand-ins for the outside world)
The outside world is not simulated in v1 and does not move by itself (owner decision 2026-10-10): each value below is constant until the player moves its slider. They are sandbox controls, not policy: the head of state does not set the oil price. A world that moves by itself comes after v1.

| Setting | Form | Used by |
|---|---|---|
| Foreign prices | growth per year of import prices, EU and non-EU | [trade & FX](../economy/trade-fx.md), [prices](../economy/prices-inflation.md) |
| Oil, gas and coal prices | level per fuel | [energy](../economy/energy.md), [prices](../economy/prices-inflation.md) |
| Foreign demand | growth per year of demand for Romanian exports, EU and non-EU | [trade & FX](../economy/trade-fx.md) |
| Foreign interest rates | ECB rate, risk-free rate abroad | [money & banking](../economy/money-banking.md), [trade & FX](../economy/trade-fx.md) |
| Wages abroad | level relative to Romania's at the start | [demographics](../economy/demographics.md) (emigration) |
| EU carbon price | per tonne | [environment](../economy/environment.md), [energy](../economy/energy.md) |
| Weather | normal by default; trigger a drought, flood or heatwave year | [environment](../economy/environment.md) |

## When each lever arrives
Milestones are those of the [roadmap](../../roadmap.md). `alpha` is M4, the first playable core. Everything in this catalogue is in v1.

| Section | alpha (M4) | Later milestone |
|---|---|---|
| Taxation | brackets, allowance, credits, capital income tax, corporate tax (one rate), VAT, contributions, property tax, wealth tax, excise on fuel, tobacco and alcohol | reduced corporate rate per industry (M9); carbon excise (M19) |
| Spending and public services | all four | — |
| Social transfers | state pension, unemployment benefit, child benefit, minimum income | housing benefit (M13); student finance, charity drive (M14) |
| Labour and markets | minimum wage | price caps (M9); immigration policy (M14) |
| Industry and ownership | — | all (M9) |
| Projects | — | all (M11); power plants (M12) |
| Energy | — | all (M12) |
| Money and finance | inflation target, policy rate, reserve requirement, QE, monetary financing | bank capital requirement, deposit insurance, bank bailout (M10) |
| Trade and currency | exchange rate regime (float, managed float, peg), FX intervention | export promotion (M18); ERM II and euro (M20) |
| EU | — | all (M18) |
| Euro | — | all (M20) |
| Informal economy | — | all (M15) |
| Foreign investment | — | all (M17) |
| Housing | — | all (M13) |
| Environment | — | all (M19) |
| State capacity | — | all (M16) |
| World settings | foreign prices, fuel prices, foreign demand, foreign interest rates, wages abroad (M3, with the mechanics that read them) | EU carbon price, weather (M19) |

## Not in v1 (for reference)
Central bank independence, laws needing parliament, military budget, foreign diplomacy, sanctions, statistics-office funding and polls (see [information](information.md)); an outside world that moves by itself; a cost for changing levers.

## Open questions
- [ ] Are any levers missing that you specifically want (e.g. working-hours law, retirement savings mandates, a CO₂ cap)? Rent control is in Housing.
- [x] Should levers have a direct "admin cost" or "reform cost" to stop the player flipping them every month? Not in v1 (owner, 2026-10-10).
