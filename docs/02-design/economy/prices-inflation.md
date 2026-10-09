---
id: economy/prices-inflation
title: Prices & Inflation
status: draft
owner: horia
depends_on: [economy/production, economy/labor-market, economy/trade-fx, economy/monetary-policy]
research: []
updated: 2026-10-09
---

# Prices & Inflation

## Purpose
Set the price of every good each tick, compute CPI and inflation for any group, and model inflation expectations. Prices carry shocks along supply chains (car parts → cars) and connect money, wages and demand.

## Real-world basis
- Firms price as a **markup over unit cost**, adjusting the markup with demand pressure; prices are sticky (Calvo-style: a fraction adjusts each period; food and energy fast, services slow).
- Cost pass-through is partial and gradual; exchange-rate pass-through to import prices is high, to CPI lower (10–30%).
- Expectations: anchored if the central bank is credible, otherwise adaptive (de-anchoring after persistent high inflation, 1970s).
- Inflation needs a monetary or demand mechanism to persist; one-off cost shocks fade if expectations stay anchored.

## Update rule (per good $g$)
**Target price:**
```math
p^*_g = (1 + \mu_g) \cdot uc_g \cdot (1 + \tau^{VAT}_g + \tau^{excise}_g)
```
**Markup** responds to demand pressure (inventory ratio and capacity utilisation):
```math
\mu_{g,t} = \mu^0_g + \eta_1 (cu_g - cu^*) - \eta_2 \Big(\frac{V_g}{v^* D^e_g} - 1\Big)
```
**Sticky adjustment** with expectations:
```math
p_{g,t} = p_{g,t-1}\big(1 + \pi^e_{monthly}\big)^{\chi} \cdot \Big(\frac{p^*_g}{p_{g,t-1}}\Big)^{\rho_g}
```
$\rho_g$ = per-good flexibility (energy and food high, services low).

**Electricity exceptions** ([energy](energy.md)): `wholesale_electricity` has no markup rule; its price `wholesale_electricity_price` ($p^W$) is set each tick by the merit order. The network good `power_grid` has a regulated tariff. The retail good `electricity` (price `electricity_price`, $p^E$) is wholesale + network tariff + taxes + supplier margin with sticky adjustment, and is the price that enters CPI and other industries' unit costs.

**Price caps:** $p_g \le \bar p_g$; if binding, excess demand → shortages (rationing: richer households get more unless a rationing rule is set), black-market pressure later.

**Imported goods:** landed price $= p^{world}_g \cdot e \cdot (1+\tau^{tariff}_g)$. Domestic price of tradables is a weighted mix of domestic and imported variants.

**CPI:** $P = \sum_g w_g p_g$ with weights from aggregate household baskets (re-weighted yearly, like statistics offices). Core CPI excludes food and energy. Group CPI uses household baskets ([households](households.md)).

## Expectations
```math
\pi^e_{t+1} = \text{cred} \cdot \pi^{target} + (1 - \text{cred}) \big[\pi^e_t + \lambda (\pi_t - \pi^e_t)\big]
```
`cred` (central bank credibility, 0–1) rises with a track record of hitting the target and independence; falls with monetary financing and persistent misses ([monetary-policy](monetary-policy.md)). Household expectations may differ (less educated households more backward-looking).

## Player levers
VAT, excise, tariffs, price caps (incl. retail and wholesale electricity caps), subsidies (lower unit cost), policy rate and credibility (via expectations), energy supply per technology.

## Outputs
Prices by good, CPI, core CPI, inflation by group, PPI, import price index, inflation expectations, decomposition of each price change (unit-cost terms, markup, taxes, expectations).

## Edge cases & failure modes
- **Hyperinflation:** monetary financing + de-anchored expectations → `cred` → 0 → spiral. Must be possible but require sustained bad policy.
- **Deflation:** falling prices + debt burden (Fisher debt deflation); zero lower bound in monetary policy.
- Numerical: cap monthly price change at e.g. ±50% to avoid overflow; record when hit.

## Acceptance tests
- [ ] A one-off 10% VAT rise raises CPI once (level), not permanently (rate), when expectations are anchored.
- [ ] A 20% currency depreciation raises import prices quickly and CPI by a smaller amount over 6–12 ticks.
- [ ] Persistent monetary financing at 10% of GDP per year eventually de-anchors expectations and causes accelerating inflation.
- [ ] A price cap below market price produces shortages, and shortage size rises with the gap.

## Open questions
- [ ] Do we allow negative markups (fire sales) in a deep recession?
- [ ] Rationing rule under shortages: proportional, by income, or player-chosen (queues vs ration cards)?
