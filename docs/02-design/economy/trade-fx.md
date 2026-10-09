---
id: economy/trade-fx
title: Trade & Exchange Rate
status: draft
owner: horia
depends_on: [economy/production, economy/prices-inflation, economy/money-banking]
research: []
updated: 2026-10-08
---

# Trade & Exchange Rate

## Purpose
Connect the country to the rest of the world: per-good exports and imports, tariffs, the exchange rate, capital flows and foreign reserves. Devaluation helps exporters and hurts importers. Interest rates move the currency.

## Real-world basis
- **Armington:** domestic and foreign varieties of a good are imperfect substitutes; import share depends on relative price.
- Export demand depends on foreign income and the relative price in foreign currency; trade elasticities ~0.5–1.5; **J-curve** (trade balance worsens before improving after depreciation).
- Exchange rates respond to interest differentials, risk and the current account (uncovered interest parity with a risk premium, plus BoP flow pressure).
- Exchange-rate pass-through to import prices is high, to CPI lower.

## Rest of world: two partner blocs (exogenous, data-driven)
- **EU** (single market; euro as currency) and **non-EU** (everyone else; USD as reference currency). Each has, per good: price, demand index; plus interest rate, inflation, growth.
- Two exchange rates: **EUR/RON** (the one that matters most) and **USD/RON**; the EUR/USD cross rate is exogenous.
- **Trade rules come from [EU membership](eu-membership.md):** no tariffs on EU trade; the EU common external tariff on non-EU imports; no capital controls. Schengen status adds or removes a border cost on EU road trade.
- **Currency regime** (float, ERM II, euro, post-exit currency) is set by [euro](euro.md). In the euro, EUR/RON no longer exists and only EUR/USD (exogenous) matters.
- Shock scenarios (oil price spike, EU recession, global recession) are scripted changes to these exogenous paths, which is allowed by pillar 6.

## Update rule
**Imports** of good $g$ (for any domestic use), with the import share split between EU and non-EU varieties by a second Armington nest:
```math
\frac{M_g}{D_g} = \frac{\alpha_g (p^w_g e (1+\tau_g))^{1-\sigma_g}}{\alpha_g (p^w_g e (1+\tau_g))^{1-\sigma_g} + (1-\alpha_g) p_g^{1-\sigma_g}}
```
with smoothing (contracts adjust over ticks).

**Exports** of good $g$ (to each bloc $b$, with its own demand, price and exchange rate):
```math
X_g = \bar X_g \cdot \Big(\frac{p_g / e \cdot (1 - s^{X}_g)}{p^w_g}\Big)^{-\epsilon_g} \cdot (\text{world demand}_g)
```
limited by capacity and supply.

**Exchange rate** (managed float, the BNR's actual regime), RON per EUR:
```math
\frac{\Delta e}{e} = -\gamma_1 \big[(i - \pi^e) - (i^* - \pi^{*e}) - \text{rp}\big] - \gamma_2 \frac{CA + \text{capital flows}}{GDP} + \text{FX intervention effect}
```
with the risk premium rp rising with debt, inflation, political instability (later) and capital controls. Under a **peg**, the CB must intervene to hold $e$; reserves deplete if the peg is overvalued → possible forced devaluation (emergent currency crisis).

**Balance of payments:** current account (goods, services incl. tourism, primary income incl. dividends of foreign-owned firms ([foreign-ownership](foreign-ownership.md)) and interest, secondary income incl. remittances ([demographics](demographics.md)) and EU current transfers) + capital account (EU capital transfers, [eu-funds](eu-funds.md)) + financial account = −Δ reserves. Booked SFC.

## Player levers
FX regime (float, managed, peg), FX intervention, export promotion. **Tariffs and capital controls are locked while Romania is in the EU** ([eu-membership](eu-membership.md)); export subsidies count as state aid.

## Outputs
Exports and imports by good, trade balance, current account, terms of trade, exchange rate (nominal and real effective), reserves, import price index.

## Worked example: devaluation
10% depreciation → export prices in foreign currency −10% → export volumes ↑ over 3–12 ticks (car makers, chemicals, tourism) → those industries hire. Import prices +10% in LCU immediately → CPI ↑ (energy, electronics, car parts) → real wages ↓ for import-heavy baskets. Short-run trade balance may worsen (J-curve), then improve.

## Acceptance tests
- [ ] A permanent 10% depreciation raises export volumes and import prices, with the trade balance improving after an initial dip.
- [ ] A 2-point rise in the domestic policy rate appreciates the currency under a float.
- [ ] A peg defended with insufficient reserves eventually breaks.
- [ ] A change in the EU common external tariff (scenario) on a good raises its domestic price and lowers non-EU imports.
- [ ] An EU recession lowers exports more than a non-EU recession of the same size (trade weights).
- [ ] BoP identity holds every tick.

## Open questions
- [ ] Name more partner blocs later (e.g. separate China, US, Turkey, Ukraine/Moldova)?
- [ ] Tourism as a services export via `hotels_tourism` and `restaurants` (proposed)?
