---
id: game/policy-scenarios
title: Policy-Scenario Test Suite (M4)
status: draft
owner: horia
depends_on: [game/lever-system, game/levers, adr/0008-explainability-architecture, adr/0009-calibration-and-stability, adr/0010-verification-and-testing, adr/0018-scenario-and-commands, economy/monetary-policy, economy/money-banking, economy/investment-capital, economy/prices-inflation, economy/labor-market, economy/households, economy/taxation, economy/vat, economy/fiscal-policy, economy/trade-fx, economy/social-transfers, economy/energy]
research: [research/budget-romania-2021, research/government-revenue-romania-2021, research/law-in-force-romania-2021]
updated: 2026-10-25
---

# Policy-Scenario Test Suite (M4)

## Purpose
Nine dated policy changes, each run against the same world without it, with the response the simulation must show: which way each headline indicator moves, roughly how far, and when. This is the M4 item "policy-scenario test suite" of the [roadmap](../../roadmap.md). Its exit: every scenario passes at three scales, and a rule with its sign flipped makes at least one scenario fail.

The suite checks the model's economics, which the ledger invariants cannot do: a sign error can still balance ([ADR-0010](../../03-architecture/decisions/0010-verification-and-testing.md), "wrong-but-balanced"). It does not calibrate. The bands are loose enough for an uncalibrated model at 1:1000. The tight, evidence-anchored bands are M7's targets file ([ADR-0009](../../03-architecture/decisions/0009-calibration-and-stability.md), e.g. R-1). Every later milestone adds its own scenarios in the same form (roadmap, "What every milestone from M9 on must deliver").

## Scope
**In:** the nine M4 scenarios (rate hike, income-tax cut, VAT rise, deficit spending, monetary financing, devaluation, minimum-wage rise, pension rise, oil-price shock). For each: the commands, the baseline, the measurements, the bands and the mechanics they need. Also the three-scale run, the noise floor and the mutation check.
**Not in:** new economic behaviour. A scenario that fails because a channel is missing is reported against the spec that owns the channel; the suite is not a place to add it. Not in either: student grants and power plants (the old Phase 1b list). Those come with their milestones (M14, M12).

## How a scenario is run
**World.** The Romania scenario built by the [scenario builder](scenario-builder.md) at the scale under test, with the 2021 law from `law.json` and `world.json`, seed 42. It starts on 1 December 2021 (D4).

**Shock date.** Every scenario's commands take effect on **T0 = 1 March 2022**, after three months in which the starting transients settle the same way in both runs. Commands are given on 15 February 2022, so `law` levers take effect on 1 March (the next 1st). For `market` and `world` levers, 1 March is named as the `effective` date. **Month k** is the k-th calendar month from T0: month 1 is March 2022, month 12 is February 2023.

**Run length.** Until 28 February 2025: 39 months, of which 36 come after T0.

**Baseline.** The same world, seed and scale with no command. One baseline run per scale serves all nine scenarios. Because the RNG is keyed ([ADR-0006](../../03-architecture/decisions/0006-determinism-contract.md)), the two runs are identical day by day up to T0 (AC-LEV-05), and every difference after T0 comes from the commands or from the paths diverging.

**Placebo and noise floor.** A third run per scale moves the standard VAT rate by one basis point (19.00% to 19.01%) from T0. Its effect is economically nil but splits the paths. For each check, the noise floor $\nu$ is the largest absolute monthly difference between placebo and baseline over the check's window, in the check's unit. A check passes only if its measured difference has the expected sign, lies in the band, and is at least $2\nu$ in absolute value. Checks whose band contains zero (the "returns to baseline" ones) skip the $2\nu$ rule.

**Commands files.** One per scenario, in the form of the command log ([lever system](lever-system.md), "Logging"). For example, `rate_hike.jsonl`:
```json
{"given":"2022-02-15","lever":"money.policy_rate","value":375,"effective":"2022-03-01"}
```
They are read by the same `read_commands` that `econ-cli play` uses, so `econ-cli play --commands rate_hike.jsonl --until 2025-02-28` reproduces a scenario by hand.

**Where.** `crates/econ-core/tests/acceptance/policy/`: one test file per scenario and the commands files under `commands/`. All protected; they are written by the test writer (ADR-0014).

## Measurements
Every indicator is read at each monthly close of both runs. Sources are the `World`'s monthly reports until M6's statistics registry replaces them: `MonthStats`, `TradeMonth`, `FiscalMonth`, `World::central_bank()` and `World::foreign()`.

| Indicator | Glossary name | Read as |
|---|---|---|
| CPI | `price_level` | `MonthStats::cpi_ppm` |
| Twelve-month inflation | `inflation` | `MonthStats::inflation_ppm` |
| Expected inflation | `inflation_expectation` | `MonthStats::expected_inflation_ppm` |
| Unemployment rate | `unemployment_rate` | `unemployed / (employed + unemployed)` of `MonthStats` |
| Mean gross wage | `wage_nominal` | `MonthStats::wage_bill / employed` |
| Household consumption | `consumption` | `MonthStats::consumption` (VAT included) |
| Real GDP | `gdp_real` | `MonthStats::gdp_expenditure / price_level` |
| Exchange rate (lei per euro) | `exchange_rate` | `World::foreign().exchange_rate_ppm` at the close; **lower = stronger leu** |
| Export volume | `exports` (volume) | `TradeMonth::exports_volume` |
| Current account | — | `TradeMonth::current_account` |
| Government deficit | — | `FiscalMonth::deficit` |
| A tax or benefit | flow code | `FiscalMonth::revenue` or `spending` under the code (`tax.income`, `tax.vat`, `benefit.pension`) |
| Credibility | `cb_credibility` | `World::central_bank().credibility_ppm` |

> **Simplification (M4):** real GDP is deflated by the CPI, not by a GDP deflator. M6 replaces it with the registry's deflator. The direction of every check here is the same with either.

**Differences against the baseline.** The window is months $a$ to $b$, both included. Scenario values are $x^s$ and baseline values $x^b$.

| Kind | Indicators | Measured difference |
|---|---|---|
| **level, %** (stocks and indices at the close) | `price_level`, `exchange_rate`, `wage_nominal` | $\frac{1}{b-a+1}\sum_{k=a}^{b} 100\,(x^s_k / x^b_k - 1)$ |
| **flow, %** (monthly flows) | `consumption`, `gdp_real`, exports, a tax or benefit | $100\,\big(\sum_{k=a}^{b} x^s_k / \sum_{k=a}^{b} x^b_k - 1\big)$ |
| **points** (rates) | `unemployment_rate`, `inflation`, `inflation_expectation`, `cb_credibility` (×100) | $\frac{1}{b-a+1}\sum_{k=a}^{b} (x^s_k - x^b_k)$, in percentage points |
| **% of GDP** (flows against output) | deficit, current account | $100\,\sum_{k=a}^{b}(x^s_k - x^b_k) \,/\, \sum_{k=a}^{b} \text{GDP}^b_k$ (nominal, `gdp_expenditure`) |

A band $[lo, hi]$ is inclusive. **Checked** expectations fail the test. **Watched** ones are computed and printed with the test's output but do not fail it. They become checked when M7 calibrates, or earlier if the owner promotes them.

## Real-world basis
Evidence behind the direction, size and timing of each band. Bands are deliberately wider than the evidence, by a factor of about three on each side, and never cross zero where the sign is the point. Confidence: **high** / **medium** / **low**. "From memory" marks figures not re-checked against the source in this pass. These are educated guesses, to be confirmed when M7 writes the targets file.

| # | Evidence | Source | Confidence |
|---|---|---|---|
| E1 | After a policy-rate rise, prices fall with a lag. The peak effect comes after about 10–20 months in post-transition economies, shorter than the 29-month average across countries. | Havránek and Rusnák (2013), "Transmission lags of monetary policy: a meta-analysis", *International Journal of Central Banking* 9(4) | high (meta-analysis), medium (CEE figure from memory) |
| E2 | In CEE, a monetary tightening lowers prices and output, with responses at least as large as in the euro area. The exchange-rate channel is strong. | Jarociński (2010), "Responses to monetary policy shocks in the east and the west of Europe", *Journal of Applied Econometrics* 25(5) | medium |
| E3 | Target for a 1-point rise: CPI trough after 8–24 months, peak effect −0.2% to −1.5%. | [ADR-0009](../../03-architecture/decisions/0009-calibration-and-stability.md), R-1 | medium (a design target from CEE evidence) |
| E4 | The BNR raised its rate from 1.25% (Sept 2021) to 7% (Jan 2023) while EUR/RON stayed within about 4.9–5.0. Under its tightly managed float, the leu moves little in response to rates. | BNR policy decisions, EUR/RON reference rates (from memory) | medium |
| E5 | Romania raised standard VAT from 19% to 24% on 1 July 2010. Twelve-month CPI inflation went from 4.4% (June) to 7.1% (July) and about 7.9% (December). It fell back to about 4.9% in July 2011, when the rise left the twelve-month base. | INS CPI releases; BNR Inflation Report, August 2010 (from memory) | medium-high |
| E6 | Standard-rate VAT changes pass to consumer prices almost fully and quickly. Reduced-rate changes pass through less. | Benedek, De Mooij, Keen and Wingender (2020), "Varieties of VAT pass through", *International Tax and Public Finance* 27 | high |
| E7 | Fiscal multipliers are small in open economies with flexible or managed exchange rates: near 0 on impact for government consumption, up to about 0.5 over a year. They are higher in a slump. | Ilzetzki, Mendoza and Végh (2013), "How big (small?) are fiscal multipliers?", *Journal of Monetary Economics* 60(2) | high (cross-country), low (for Romania) |
| E8 | Spending shocks in Romania have positive but small output effects. | Boiciuc (2015), "The effects of fiscal policy shocks in Romania: a SVAR approach", *Procedia Economics and Finance* 32 (from memory) | low |
| E9 | Low-income and liquidity-constrained households spend 0.5–0.9 of extra income, wealthy ones 0.05–0.2. | [households](../economy/households.md), "Real-world basis" | medium |
| E10 | Deficits financed by the central bank produced Romania's high inflation of the 1990s: about 256% in 1993 and 155% in 1997. Monetary financing de-anchors expectations. | INS CPI (from memory); [monetary policy](../economy/monetary-policy.md), "Real-world basis" | medium |
| E11 | Exchange-rate pass-through is high to import prices (near full within a few months) and lower to the CPI. For Romania, the CPI pass-through after a year is about 0.2–0.5. | Ca' Zorzi, Hahn and Sánchez (2007), ECB Working Paper 739; [trade & FX](../economy/trade-fx.md) and [prices](../economy/prices-inflation.md) | medium (imports), low-medium (CPI, Romania) |
| E12 | Trade price elasticities are 0.5–1.5. The trade balance can worsen before it improves (J-curve). | [trade & FX](../economy/trade-fx.md), "Real-world basis" | medium |
| E13 | Hungary roughly doubled its minimum wage in 2001. Firms passed about three quarters of the cost to consumers, and about 10% of the affected jobs were lost. | Harasztosi and Lindner (2019), "Who pays for the minimum wage?", *American Economic Review* 109(8) | high |
| E14 | Romania's minimum wage was 2,300 lei in 2021 (53% of the median), 2,550 in 2022 and 3,000 in 2023. A large share of employees earn exactly the minimum. | [labour market](../economy/labor-market.md); [law in force](../../01-research/notes/law-in-force-romania-2021.md) | high (levels), medium (share at the minimum) |
| E15 | The pension point rose 10% on 1 January 2022 (1,442 to 1,586 lei). Old-age pensions cost 111.3 billion lei in 2021, 9.4% of GDP. | [budget 2021](../../01-research/notes/budget-romania-2021.md); law in force (point value) | high |
| E16 | A 10% rise in the oil price raises euro-area consumer prices by about 0.1–0.3 points in the first year. Net oil importers lose about 0.1–0.2% of GDP. Romania imports most of its crude and is a net importer of fuels (about 1.5–2% of GDP). | ECB and IMF rules of thumb (from memory); Romanian trade statistics (from memory) | medium (rules of thumb), low (Romania's net fuel imports) |

## The scenarios
The levers' values are the shape's integers: rates in basis points, money in bani ([lever system](lever-system.md)). "Status today" was checked against `crates/econ-core/levers.toml` and `crates/` on main, 2026-10-25.

### S1 Rate hike (AC-POL-01)
**Commands.** `money.policy_rate` 175 → 375 (1.75% → 3.75%, +2 points), effective T0, held to the end.

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| The leu is stronger within three months | `exchange_rate`, level % | 1–3 | −10 to −0.1 | checked | E2, E4, trade-fx step 4 |
| Prices are lower after a year | `price_level`, level % | 12–24 | −6 to −0.2 | checked | E1, E3 |
| Unemployment is higher | `unemployment_rate`, points | 6–18 | +0.05 to +2.5 | watched (checked from M13) | E2, AC-MP-06 |
| Inflation is lower | `inflation`, points | 12–24 | −4 to −0.1 | watched | E1, E3 |
| Real GDP is lower | `gdp_real`, flow % | 6–18 | −4 to −0.05 | watched | E2 |
| The deficit is larger (dearer debt) | deficit, % of GDP | 1–24 | 0 to +1 | watched | fiscal-policy step 3 |

**Channels the model has.** Loan and deposit rates reprice at the next close (AC-BANK-05). Firms' investment falls with the real loan rate (AC-INV-08), so demand for construction and equipment falls. The leu appreciates through the interest-parity term of the exchange-rate step, and the BNR's intervention offsets 80% of the move. Import prices then fall, and so do exports. New bonds pay the higher yield.
**Missing, and why the bands allow for it.** Households do not borrow (M13), and their consumption does not react to the real rate in the first version ([households](../economy/households.md)). Higher deposit rates raise households' interest income, which pushes the other way. So the rate hike works only through firms' credit, deposits and the exchange rate. The unemployment band starts at 0.05 points for that reason.
**Status today.** **Blocked:** `money.policy_rate` is `planned`. The banks and the exchange rate already read the rate (`StartEdit::PolicyRate` exists for tests). Wiring is the registry's job (M4 group "wire every alpha lever").

### S2 Income-tax cut (AC-POL-02)
**Commands.** `tax.income.schedule` `[[0, 1000]]` → `[[0, 500]]` (flat 10% → 5%), effective T0.

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| Income-tax revenue falls, by less than the rate | `tax.income`, flow % | 1–12 | −65 to −25 | checked | mechanical, E7 |
| Household consumption rises | `consumption`, flow % | 2–12 | +0.2 to +4 | checked | E9, E7 |
| The deficit is larger | deficit, % of GDP | 1–12 | +0.1 to +1.2 | watched | mechanical |
| Prices are slightly higher | `price_level`, level % | 6–18 | 0 to +2 | watched | demand |
| Unemployment is no higher | `unemployment_rate`, points | 6–18 | −1 to +0.1 | watched | E7 |

**Channels.** Pay day computes the tax from the schedule ([taxation](../economy/taxation.md)). Disposable income rises, and households spend it through `mpc` (near 1 for the three poorest fifths). Pensions above `tax.income.pension_tax_free` are taxed by the same schedule.
**Note.** The `mpc` defaults (1.00 to 0.95) are far above the evidence (E9), so the consumption response may sit at the top of the band until M7.
**Status today.** Ready: the lever is `active` and read.

### S3 VAT rise (AC-POL-03)
**Commands.** `tax.vat.rate_standard` 1900 → 2400 (19% → 24%, the 2010 change), effective T0.

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| The price level jumps once | `price_level`, level % | 2–6 | +1 to +6 | checked | E5, E6 |
| Inflation returns to baseline after the base month | `inflation`, points | 14–24 | −1.5 to +1.5 | checked | E5, AC-PRICE-06 |
| VAT revenue rises, by less than in proportion (+26.3%) | `tax.vat`, flow % | 1–12 | +8 to +26 | checked | AC-TAX-11 |
| Real consumption falls | `consumption / price_level`, flow % | 1–12 | −4 to −0.1 | watched | E6 |

**Channels.** The consumer price is $(p_g + d_g)(1 + \tau^{VAT}_g)$ for standard-rated goods ([prices](../economy/prices-inflation.md), [VAT](../economy/vat.md)), so the price level jumps on T0. Producer prices may give a little back as demand falls. The twelve-month rate is one-off as long as expectations stay anchored (credibility 0.7) and the January wage round indexes only to expected inflation.
**Status today.** Ready (the lever is `active`; AC-LEV-15 already uses it).

### S4 Deficit spending (AC-POL-04)
**Commands.** `spend.ministry_budget[education]` 3,242,470,000,000 → 3,842,470,000,000 bani and `spend.ministry_budget[health]` 5,768,260,000,000 → 6,368,260,000,000 bani. That is +6 billion lei a year each, together about 1% of 2021 GDP (1,179.8 billion), effective T0 and permanent. Investment shares, pay and headcount are unchanged, so the extra running money buys inputs ([lever system](lever-system.md), "Ministry budgets"). Bonds finance it.

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| The deficit is larger, by less than the spending | deficit, % of GDP | 1–12 | +0.3 to +1.1 | checked | mechanical, revenue feedback |
| Real GDP is higher | `gdp_real`, flow % | 3–12 | +0.1 to +2 | checked | E7, E8, AC-FIS-06 |
| Unemployment is lower | `unemployment_rate`, points | 6–18 | −1 to −0.02 | watched | E7 |
| Prices are slightly higher | `price_level`, level % | 6–18 | 0 to +2 | watched | demand |

**Channels.** Public units get a larger running allowance and spend it day by day in their industry's input mix, imports included. Imports leak part of it. Suppliers' output, staff and pay rise. The treasury issues bonds to keep its deposits at target ([fiscal policy](../economy/fiscal-policy.md), step 3).
**Status today.** Ready: the levers are `active` (M4 group 5). The first month's inputs are fixed at the previous month end (the lever system's simplification), so the GDP window starts at month 3.

### S5 Monetary financing (AC-POL-05)
**Commands.** `money.monetary_financing` 0 → 800,000,000,000 bani (8 billion lei a month, about 8% of 2021 GDP a year, roughly the 2021 deficit), effective T0. A second line sets it back to 0 on 1 March 2024 (24 months). Spending is unchanged: the central bank funds the existing deficit, and the treasury issues fewer bonds.

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| Expected inflation rises | `inflation_expectation`, points | 3–12 | +0.5 to +12 | checked | E10, monetary-policy step 5 |
| Credibility falls | `cb_credibility`, points (×100) | 6–24 | −70 to −2 | checked | AC-MP-05 |
| Prices are higher | `price_level`, level % | 12–24 | +0.5 to +25 | checked | E10, AC-PRICE-10 |
| Inflation is higher | `inflation`, points | 13–30 | +0.5 to +20 | watched | E10 |
| The leu is weaker | `exchange_rate`, level % | 6–24 | +0.1 to +30 | watched | trade-fx step 4 (lower real rate) |

**Channels.** Expected inflation rises by `monfin_pass` times the year's monetary financing over GDP. Credibility falls by $k_3$ times the same ratio ([monetary policy](../economy/monetary-policy.md), step 5). Prices follow expectations through `expectation_indexation` and wages at the January round. A lower real rate weakens the leu, which raises import prices.
**Status today.** **Blocked:** `money.monetary_financing` is `planned`. The measurement and the expectations term exist (AC-MP-05 uses a start edit). Wiring needs the central bank to credit the government's deposits each month under a flow code, and the treasury's issue rule to see the extra deposits.

### S6 Devaluation (AC-POL-06)
**Commands.** The leu is devalued by 10% against the euro on T0, and the managed float continues from the new level. **No registered lever can express this yet** (see Open questions). Proposed: a new `market` lever `fx.devaluation` (rate, signed, a one-off step of `exchange_rate` on its effective day). The scenario would set it to 1000 (+10%).

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| The leu is weaker | `exchange_rate`, level % | 1–3 | +5 to +11 | checked | the command |
| Prices are higher, by much less than 10% | `price_level`, level % | 6–12 | +0.5 to +6 | checked | E11, AC-PRICE-09 |
| Exports rise | exports, flow % | 7–18 | +1 to +12 | checked | E12, AC-TFX-04 |
| Import prices in lei rise | import price index (imports-weighted `import_price_ppm`), level % | 1–3 | +5 to +11 | watched | E11 |
| The current account improves after the J-curve | current account, % of GDP | 13–24 | 0 to +3 | watched | E12 |

**Channels.** Import prices in lei are the euro price times $e/e_0$, so they rise at once. Import shares and export volumes adjust monthly (Armington, `share_adjustment`). Domestic prices rise through imported inputs and the household basket. A better current account then pulls the leu back a little ($\gamma_2$ term).
**Status today.** *Wired 2026-10-25:* `fx.devaluation` is `active` ([trade & FX](../economy/trade-fx.md), amendment of 2026-10-25). *Before:* **Blocked:** there is no lever for a step of the rate. `fx.regime` and `fx.intervention` are `planned`, and an intervention amount has no rule for how far it moves $e$ ([trade & FX](../economy/trade-fx.md) is locked). Until the lever exists, AC-TFX-04's world-price edit (every euro price ×1.1) gives the same import prices and export demand and can stand in.

### S7 Minimum-wage rise (AC-POL-07)
**Commands.** `labour.minimum_wage` 230,000 → 300,000 bani (2,300 → 3,000 lei, +30%: the 2023 level, about 69% of the starting median), effective T0.

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| The mean wage rises | `wage_nominal`, level % | 1–6 | +1.5 to +12 | checked | E14, AC-LM-05 |
| Prices are higher | `price_level`, level % | 6–18 | +0.2 to +5 | checked | E13, AC-PRICE-05 |
| Employment falls a little, if at all | `employment`, level % | 6–18 | −3 to +0.5 | watched | E13, AC-LM-09 |
| Fewer households in hardship | `hh_hardship` households, level % | 3–12 | −30 to 0 | watched | income |

**Channels.** On T0 every wage below the new minimum is raised to it (AC-LM-05). Unit costs rise in proportion to each industry's wage share, and prices follow over months. Pay and job choice (M4 group 6) moves relative pay.
**Missing.** Labour demand does not substitute away from labour, and informality does not react (M15). The employment effect comes only through demand, so it is watched with no required sign.
**Status today.** Ready: the lever is `active` and read.

### S8 Pension rise (AC-POL-08)
**Commands.** `transfer.pension.point_value` 144,200 → 158,600 bani (1,442 → 1,586 lei, +10%: the January 2022 rise), effective T0.

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| State pension spending rises, by at most 10% | `benefit.pension`, flow % | 1–12 | +6 to +10.5 | checked | E15, AC-ST-01 |
| Household consumption rises | `consumption`, flow % | 1–12 | +0.3 to +3 | checked | E9, E15 |
| The deficit is larger | deficit, % of GDP | 1–12 | +0.3 to +1.2 | watched | E15 |
| Prices are slightly higher | `price_level`, level % | 6–18 | 0 to +1.5 | watched | demand |

**Channels.** A pension is points × point value, not below the minimum ([social transfers](../economy/social-transfers.md)). Pensions at the minimum stay where they are, which is why the bill rises by less than 10%. Pensioners' households spend through `mpc`.
**Status today.** Ready: the lever is `active` and read.

### S9 Oil-price shock (AC-POL-09)
**Commands.** `world.fuel_price[oil]` 32,400 → 48,600 bani a barrel (+50%, about the move of early 2022), effective T0, permanent.

| Expectation | Indicator, kind | Window | Band | | Evidence |
|---|---|---|---|---|---|
| Prices are higher | `price_level`, level % | 3–12 | +0.2 to +4 | checked | E16 |
| The current account worsens | current account, % of GDP | 1–12 | −2 to −0.1 | checked | E16 |
| Fuel is dearer | `price[refined_petroleum]`, level % | 1–3 | +5 to +50 | watched | E16, AC-EXC-09 |
| Real GDP is lower | `gdp_real`, flow % | 6–18 | −2 to 0 | watched | E16 |

**Channels (proposed wiring).** Fuel is an ordinary import today ([energy](../economy/energy.md) comes in M12). Proposed for M4: the oil setting scales the euro price of the imported variants of `crude_oil` and `refined_petroleum` by its ratio to the starting value. Refining, transport and households then pay more through unit cost and the basket. The excise is per unit, so the pump price rises by less than crude in percent.
**Status today.** *Wired 2026-10-25:* the oil key is `active` and scales the euro price of `crude_oil` and `refined_petroleum`; imported inputs pay oil's price on their oil share (trade & FX, amendment of 2026-10-25); gas and coal are `planned_keys`. *Before:* **Blocked:** `world.fuel_price` is `planned`, and no mechanic reads a fuel price (lever-system table: "gap"). The world-price table holds a euro price per good, so wiring the oil key to two goods is small. Gas and coal wait for M12.

### Summary
| # | Scenario | Command (T0 = 1 Mar 2022) | Headline checked expectation | Status today |
|---|---|---|---|---|
| S1 | Rate hike | `money.policy_rate` +2 points | leu stronger in months 1–3; CPI 0.2–6% lower in months 12–24; unemployment higher | wired (2026-10-25) |
| S2 | Income-tax cut | `tax.income.schedule` 10% → 5% | income-tax revenue −25 to −65%; consumption +0.2 to +4% | ready |
| S3 | VAT rise | `tax.vat.rate_standard` 19% → 24% | CPI +1 to +6% once; inflation back within 1.5 points in months 14–24 | ready |
| S4 | Deficit spending | education and health +6 bn lei a year each | deficit +0.3 to +1.1% of GDP; real GDP +0.1 to +2% | ready |
| S5 | Monetary financing | `money.monetary_financing` 8 bn lei a month, 24 months | expected inflation up; credibility down; CPI +0.5 to +25% | wired (2026-10-25) |
| S6 | Devaluation | `fx.devaluation` 1000 (leu −10%) | CPI +0.5 to +6%; exports +1 to +12% | wired (2026-10-25) |
| S7 | Minimum wage | `labour.minimum_wage` 2,300 → 3,000 lei | mean wage +1.5 to +12%; CPI +0.2 to +5% | ready |
| S8 | Pension rise | `transfer.pension.point_value` +10% | pension bill +6 to +10.5%; consumption +0.3 to +3% | ready |
| S9 | Oil shock | `world.fuel_price[oil]` +50% | CPI +0.2 to +4%; current account worse | wired (2026-10-25, oil only) |

## Three scales
The scales are those of CI's `scales` job: **1:1000, 1:100 and 1:10**. Each scale runs the baseline, the placebo and the nine scenarios: eleven runs of 39 months. At 1:1000 and 1:100 they run in the `scales` job on every PR that touches `crates/`. That is about 0.5 minutes at 1:1000 and 2–4 at 1:100 with M3's speed, measured when the suite exists. The 1:10 runs are `#[ignore]`d and run nightly, like AC-WLD-10's 1:10 run. A run at any scale must also keep invariants I-1 to I-8 and person conservation every day, as every `[sim]` test does.

Per-capita and percentage measurements make the bands scale-free. Only the noise floor $\nu$ depends on the scale, and it is measured, not assumed.

## The mutation check
The suite must catch a rule with its sign flipped. A mutant is a build in which one named term of one rule has the wrong sign. Each mutant must make its named scenario fail at 1:1000, while the unmutated build passes.

| Mutant | Rule and term flipped | Must fail | Why it is caught |
|---|---|---|---|
| MUT-1 | `econ-mech-trade::fx_step`: the interest-parity term ($-\gamma_1[\dots]$ becomes $+\gamma_1[\dots]$) | S1 (leu stronger) | a higher rate then weakens the leu |
| MUT-2 | the purchaser price ([VAT](../economy/vat.md)): $(1 + \tau^{VAT})$ becomes $(1 - \tau^{VAT})$ | S3 (price jump) | a VAT rise then lowers the price level |
| MUT-3 | the monetary-financing term of expectations (`monfin_pass` added with the wrong sign) | S5 (expectations rise) | financing then lowers expected inflation |

MUT-1 runs on every PR that touches the trade crate or the policy suite. All three run nightly. The mechanism (cargo-mutants restricted to these functions, or a test-only switch) is an open question.

## Relation to criteria owed elsewhere
Several locked specs owe M4 criteria that these runs exercise: AC-MP-06 (rate hike and inflation), AC-TFX-06 (rate hike and the leu), AC-PRICE-06 (VAT one-off), AC-TAX-11 (VAT revenue), AC-FIS-06 (debt-financed spending), AC-LM-09 (minimum wage). They keep their IDs and their own tests. The test writer may build them on this suite's runs and harness, so each scenario is run once per scale.

## Interactions
- **[Lever system](lever-system.md):** commands, validation, effective dates, `read_commands`, `econ-cli play`. A scenario's commands must be accepted. A rejection fails the test with the lever system's message.
- **[ADR-0008](../../03-architecture/decisions/0008-explainability-architecture.md):** a scenario is a one-group counterfactual (L4) with the baseline as the fork. The suite's runs are the same forks the "why" panel will make. AC-LEV-15 already checks that a lever's L2 term appears in a price's "why".
- **[ADR-0009](../../03-architecture/decisions/0009-calibration-and-stability.md):** M7's targets file tightens these bands (R-1 for the rate). A band here is never tighter than the target there.
- **[ADR-0010](../../03-architecture/decisions/0010-verification-and-testing.md):** the three-scale and mutation layers.

## Edge cases & failure modes
- **The baseline drifts** (the leu, firm loans and consumption before calibration, see AC-TFX-05 and AC-BANK-08). Differences against the baseline remove the drift. A scenario that only "passes" because the baseline collapsed is caught by the invariants and by the quiet-baseline tests, not here.
- **A check below its noise floor** fails at that scale. If this happens at 1:1000 for a checked expectation, the shock size or the window is revised here before the band is widened (see Open questions).
- **A command rejected** (out of range, `not_yet`): the test fails with the rejection, so a blocked scenario fails until its lever is wired. It is written behind its lever's cargo feature until then (restated 2026-10-25 by the test writer: `#[ignore]` markers fail CI's count, so the blocked tests are gated as the M3 and M4 tests were; see below).
- **Months after a lever returns** (S5 after month 24): windows end at or soon after the reversal, except the inflation window, which follows the persistence.

## Acceptance tests
Each scenario criterion means: the scenario's commands file is accepted; the scenario, the baseline and the placebo run to 28 February 2025 at the scale; and every **checked** expectation of its table holds (sign, band and noise floor, as in "Measurements"). Watched expectations are printed, not asserted. Tests go in `crates/econ-core/tests/acceptance/policy/` (protected).

- [ ] **AC-POL-01** `[sim]` Rate hike (S1): with the policy rate 2 points higher from T0, the leu is stronger in months 1–3, the price level is lower in months 12–24, within the bands of S1. (Unemployment higher in months 6–18 is watched until M13 brings household credit, then checked.)
- [ ] **AC-POL-02** `[sim]` Income-tax cut (S2): with a flat 5% income tax from T0, income-tax revenue falls by 25–65% and household consumption rises by 0.2–4% over the first year, within the bands of S2.
- [ ] **AC-POL-03** `[sim]` VAT rise (S3): with standard VAT at 24% from T0, the price level is 1–6% higher in months 2–6, twelve-month inflation is within 1.5 points of the baseline in months 14–24, and VAT revenue rises by 8–26% in the first year.
- [ ] **AC-POL-04** `[sim]` Deficit spending (S4): with education and health budgets each 6 billion lei a year higher from T0, the deficit is 0.3–1.1% of GDP larger in the first year and real GDP 0.1–2% higher in months 3–12.
- [ ] **AC-POL-05** `[sim]` Monetary financing (S5): with the central bank funding 8 billion lei a month for 24 months, expected inflation is higher in months 3–12, credibility lower in months 6–24 and the price level higher in months 12–24, within the bands of S5.
- [ ] **AC-POL-06** `[sim]` Devaluation (S6): with the leu devalued by 10% on T0, the exchange rate is 5–11% weaker in months 1–3, the price level 0.5–6% higher in months 6–12 and export volumes 1–12% higher in months 7–18.
- [ ] **AC-POL-07** `[sim]` Minimum-wage rise (S7): with the minimum wage at 3,000 lei from T0, the mean gross wage is 1.5–12% higher in months 1–6 and the price level 0.2–5% higher in months 6–18.
- [ ] **AC-POL-08** `[sim]` Pension rise (S8): with the pension point 10% higher from T0, state pension spending is 6–10.5% higher and household consumption 0.3–3% higher over the first year.
- [ ] **AC-POL-09** `[sim]` Oil shock (S9): with the world oil price 50% higher from T0, the price level is 0.2–4% higher in months 3–12 and the current account 0.1–2% of GDP lower over the first year.
- [ ] **AC-POL-10** `[sim]` Three scales: AC-POL-01 to 09 pass at 1:1000, 1:100 and 1:10 (1:10 in the nightly run). Every run (baseline, placebo, scenarios) keeps invariants I-1 to I-8 and person conservation every day. At each scale, the placebo's measured differences are below every checked band's lower bound in absolute value.
- [ ] **AC-POL-11** `[sim]` Mutation: each mutant of the mutation table (MUT-1 to 3) makes its named scenario fail at 1:1000. The unmutated build passes all nine.

**For the test writer:** AC-POL-01, 05, 06 and 09 can only pass once their levers are wired (S1, S5, S6, S9 "Status today"). *Restated 2026-10-25 (test writer):* they are written behind the features `lever_policy_rate`, `lever_monetary_financing`, `lever_fx_devaluation` and `lever_fuel_price` of `econ-core` instead of `#[ignore]`, and the suite as a whole behind `policy`; the PR that wires a lever adds its feature to `default`. AC-POL-10 and 11 run the scenarios whose features are on.

## API sketch
For the test writer; names may change, shapes should not. Bands in millionths of the unit, so the checks are integer.
```rust
// crates/econ-core/tests/acceptance/policy/harness.rs (test support, protected)
pub enum Kind { LevelPct, FlowPct, Points, PctOfGdp }
pub enum Ind { PriceLevel, Inflation, ExpectedInflation, Unemployment, MeanWage, Consumption, RealGdp,
               ExchangeRate, ExportVolume, CurrentAccount, Deficit, Flow(&'static str), Credibility,
               Employment, Hardship, Price(&'static str), ImportPriceIndex }
pub struct Check { pub ind: Ind, pub kind: Kind, pub months: (u32, u32), pub band_ppm: (i64, i64), pub checked: bool }
pub struct Scenario { pub id: &'static str, pub commands: &'static str /* commands/<id>.jsonl */, pub checks: &'static [Check] }
pub struct Runs { pub baseline: Vec<Monthly>, pub placebo: Vec<Monthly> }          // one per scale, cached
pub fn runs(scale: u32) -> &'static Runs;
pub fn run_scenario(scale: u32, s: &Scenario) -> Vec<Monthly>;                     // 39 monthly closes
pub fn evaluate(s: &Scenario, scen: &[Monthly], runs: &Runs) -> Vec<Outcome>;      // sign, band, 2ν, printed
```

## Open questions
- [x] **Devaluation lever.** Which command expresses S6? Proposed: a new `market` lever `fx.devaluation` (signed rate, a one-off step of `exchange_rate` on its day; the managed float continues from there), added to the registry and to [trade & FX](../economy/trade-fx.md) (locked, so the owner decides). Alternatives: (a) `fx.regime = peg` plus a new `fx.peg_rate`, which also changes the regime; pegs are M20 in trade-fx. (b) `fx.intervention` with a new rule for how far an amount moves $e$. (c) Keep AC-TFX-04's world-price edit as the test's stand-in and drop the lever. **Decided (owner, 2026-10-25):** both: the one-off `fx.devaluation` lever now (M4, with a dated amendment of trade-fx), and a peg with a chosen rate later, with the euro and ERM II (M18/M20).
- [x] **Oil wiring.** Proposed: `world.fuel_price[oil]` scales the euro price of imported `crude_oil` and `refined_petroleum` by its ratio to the start. Gas and coal keys stay `planned` until M12. Alternative: crude only, with refined products following through refining's unit cost. **Decided as proposed (owner's delegation, 2026-10-25).**
- [x] **Shock sizes.** The sizes were chosen large enough to rise above 1:1000 noise (+2 points, 10% → 5%, 19% → 24%, ~1% of GDP, 8% of GDP a year, 10%, +30%, +10%, +50%). Should they instead be the "textbook" unit shocks (1 point, 1% of GDP), with the bands scaled down and 1:1000 allowed to be inconclusive? **Decided (owner, 2026-10-25):** the large shocks.
- [x] **Inconclusive at 1:1000.** If a checked expectation falls below $2\nu$ at 1:1000 only, does the scenario fail (proposed: yes, as the roadmap's exit says "passes at three scales"), or is 1:1000 allowed to report "inconclusive" while 1:100 and 1:10 must pass? **Decided (owner, 2026-10-25):** fail.
- [x] **Unemployment in the rate hike.** Households neither borrow nor respond to the real rate, so the rise in unemployment rests on investment and exports. Keep it checked (proposed, it is AC-MP-06's "temporary rise"), or watched until M13 brings household credit? **Decided (owner, 2026-10-25):** watched until M13, then checked; the leu and the price level stay checked.
- [x] **Mutation mechanism.** Proposed: a `just policy-mutants` recipe running cargo-mutants on `fx_step`, the purchaser-price function and the expectations step, with the 1:1000 suite as the test command, requiring the three listed sign mutants to be caught. Alternative: a test-only cargo feature per mutant in the mechanic crates (simpler to run, but code that exists only to be wrong). **Decided as proposed (owner's delegation, 2026-10-25).**
- [x] **CI time.** Eleven 39-month runs at 1:100 on every PR touching `crates/`: acceptable, or only 1:1000 per PR and 1:100 nightly with 1:10? **Decided under the owner's delegation (2026-10-25):** as proposed above: 1:1000 and 1:100 on every PR touching `crates/` (2–4 minutes at 1:100), 1:10 nightly.
- [x] **Watched expectations.** Should any be checked from M4 (for example the deficit in S2 and S8, which are nearly mechanical)? **Decided none promoted in M4; revisit in M7 (owner's delegation, 2026-10-25).**
