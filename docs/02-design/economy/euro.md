---
id: economy/euro
title: Euro — Joining and Leaving the Euro Area
status: draft
owner: horia
depends_on: [economy/eu-membership, economy/monetary-policy, economy/trade-fx, economy/money-banking, economy/fiscal-policy]
research: []
updated: 2026-10-08
---

# Euro — Joining and Leaving the Euro Area

## Purpose
Romania uses the leu but is legally committed to adopting the euro. The player can steer toward adoption (meet the convergence criteria, enter ERM II, convert) and, once in, can also **leave** the euro area. These are the biggest monetary decisions in the game: joining trades away the policy rate and exchange rate for lower risk premia and trade integration; leaving brings back monetary control at the cost of a currency crisis risk.

## Real-world basis
*(Verify current criteria and procedures.)*
- **Convergence (Maastricht) criteria:** inflation ≤ 1.5 pp above the average of the three best-performing EU members; deficit ≤ 3% of GDP (no excessive deficit procedure); debt ≤ 60% of GDP or falling satisfactorily; long-term interest rate ≤ 2 pp above the three best performers; at least 2 years in **ERM II** without severe tensions (±15% band around a central rate); legal compatibility (central bank independence).
- Adoption is decided by the EU Council on the basis of Commission and ECB reports; the conversion rate is fixed irrevocably.
- Inside the euro: the ECB sets one policy rate for the whole area; national central banks keep macroprudential tools; no national exchange rate; lower currency risk premia; cross-border lending and trade costs fall.
- **Leaving** has no legal procedure in the treaties. Real-world analysis (e.g. Greek "Grexit" scenarios) points to bank runs, capital controls, redenomination disputes over debt, and a sharp devaluation of the new currency.

## Stages and state
| Stage | Monetary tools | Exchange rate |
|---|---|---|
| `leu_float` (start) | Full BNR control ([monetary-policy](monetary-policy.md)) | Managed float vs EUR |
| `erm2` | Full control, but the exchange rate must stay within the band | Central rate ± 15%; BNR intervenes; ECB support at the band edges |
| `euro` | ECB rate is exogenous (euro-area scenario); BNR keeps macroprudential tools only | Fixed: 1 EUR everywhere |
| `exited` | New national currency, full control | New currency floats (usually sharp depreciation) |

State variables: `currency_stage`, `erm2_central_rate`, `erm2_months`, `convergence[criterion]` (pass/fail with values), `conversion_rate`.

## Joining
1. **Apply for ERM II** (lever): requires a pre-condition package (e.g. banking union participation, reform commitments). Fixes the central rate (player chooses within a plausible range; an overvalued rate hurts exports later).
2. **Hold the band** for 24 months: speculative pressure rises if fundamentals (inflation, deficit, current account, credibility) are weak; reserves are used to defend.
3. **Convergence report:** every 2 years (or on request), all criteria are checked. The UI shows the live criteria dashboard.
4. **Council decision:** if all pass, adoption at the next 1 January after a transition period (≈ 6–12 months).
5. **Conversion:** all leu stocks (deposits, loans, bonds, wages, prices) are converted at the fixed rate in the accounting system. One-off small price rounding effect on services (empirically small; parameter).

**Effects after joining:**
- The policy rate becomes the ECB rate (exogenous scenario), so domestic overheating can't be fought with rates; fiscal and macroprudential tools matter more.
- Currency risk premium on bonds and loans disappears; the sovereign risk premium remains ([fiscal-policy](fiscal-policy.md)).
- Trade with the euro area: no conversion costs or exchange-rate risk → small permanent boost to EU trade and FDI ([trade-fx](trade-fx.md), [foreign-ownership](foreign-ownership.md)).
- Seigniorage shared via the Eurosystem; BNR profits change.
- No devaluation option: competitiveness adjusts only through wages and prices (internal devaluation).

## Leaving
1. **Announce exit** (lever). From the announcement: deposit flight from banks (to cash and abroad), capital outflows, bond yields spike.
2. **Bank holiday and capital controls** (temporarily allowed in the game during exit; EU relations cost) to stop the run.
3. **Redenomination:** domestic-law contracts (most deposits, loans, government bonds under Romanian law) convert to the new currency at 1:1; foreign-law debt stays in EUR → balance-sheet losses for anyone with EUR liabilities as the new currency falls.
4. **New currency floats:** devaluation size depends on credibility, current account, debt and reserves.
5. **Aftermath:** monetary control returns; inflation rises (import prices); exporters gain; EU relationship and investor confidence suffer for years (state capacity / institutions index).

## Player levers
Apply to ERM II and choose the central rate; defend the band (reserves, rates); request a convergence assessment; adopt the euro (when eligible); announce exit from the euro; capital controls during exit.

## Outputs
Convergence dashboard (each criterion: value, threshold, pass/fail, trend), ERM II band chart, stage timeline, estimated gains and losses (rates, trade, competitiveness), exit crisis indicators.

## Acceptance tests
- [ ] The player cannot adopt the euro while any convergence criterion fails.
- [ ] In ERM II, sustained weak fundamentals create pressure on the band that costs reserves to defend.
- [ ] After adoption, the domestic policy-rate lever is disabled and the rate follows the ECB scenario; all leu stocks convert exactly at the conversion rate (accounting invariants hold).
- [ ] Announcing exit triggers deposit outflows and a depreciation of the new currency; agents with foreign-law EUR debt suffer losses.

## Open questions
- [ ] How should the ECB rate scenario respond to Romania itself (proposed: not at all — Romania is small in the euro area)?
- [ ] Should the player be allowed to skip ERM II (unrealistic) in a "sandbox rules" mode?
