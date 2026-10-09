---
id: economy/monetary-policy
title: Monetary Policy & Central Bank
status: draft
owner: horia
depends_on: [economy/money-banking, economy/prices-inflation]
research: []
updated: 2026-10-08
---

# Monetary Policy & Central Bank

## Purpose
The central bank (modelled on the National Bank of Romania, BNR) sets the policy rate and other tools that heat up or cool down the economy, affect inflation, the exchange rate and asset prices. **In v1 the player controls the central bank directly.** If Romania adopts the euro, the policy rate is set by the ECB (exogenous) and only macroprudential tools remain; see [euro](euro.md).

## Real-world basis
- Transmission: policy rate → market rates → borrowing, saving, asset prices, exchange rate → demand → output and employment → inflation. Peak effect on output after ~1 year, on inflation after ~1–2 years.
- Taylor rule as a description of independent central banks.
- Credibility matters: credible central banks disinflate at lower output cost.
- Monetary financing of deficits is inflationary when large and persistent.

## Control
- **v1: player-controlled.** The player sets every tool directly and announces an inflation target. Credibility still exists: it drifts down when the player uses the tools for short-term goals (e.g. cutting rates while inflation is above target) and up with a track record of hitting the target.
- **Advisory rule:** the UI shows what a Taylor-type rule would recommend, as guidance only:
```math
i^{rule}_t = \rho\, i_{t-1} + (1-\rho)\big[r^* + \pi^e + \phi_\pi (\pi - \pi^{target}) + \phi_y \, \text{gap}\big]
```
- **Later:** an independence option where the central bank follows this rule on its own (higher credibility, less control).

## Tools
| Tool | Effect |
|---|---|
| Policy rate | Bank funding cost → all rates (money-banking); interest-rate differential → capital flows (trade-fx) |
| Reserve requirement | Bank liquidity cost, spreads |
| QE / QT | CB buys or sells government bonds → bond yields ↓↑, bank reserves ↑↓, asset prices |
| Monetary financing | CB credits government deposits directly → money supply ↑, credibility ↓↓ |
| FX intervention | Buy or sell FX reserves (see trade-fx) |
| Lender of last resort | Advances to banks in a run |

**Zero lower bound:** policy rate floor (default 0%, configurable to slightly negative).

**Central bank profit** (interest on assets − interest on reserves) is remitted to government; losses are possible (e.g. QE at rising rates).

## State variables
`policy_rate`, `inflation_target`, `cb_credibility` (0–1), CB balance sheet (bonds, FX reserves, advances, reserves, cash, gov deposits).

## Credibility update
```math
\text{cred}_{t+1} = \text{cred}_t + \lambda_c \big[k_1 \cdot \text{track record} - k_2 |\pi - \pi^{target}| - k_3 \cdot \text{monfin}/GDP - k_4 \cdot \text{target changes}\big]
```
clipped to [0, 1]. Feeds expectations ([prices-inflation](prices-inflation.md)).

## Acceptance tests
- [ ] A 2-point rate hike lowers inflation by a meaningful amount after 12–24 ticks, with a temporary rise in unemployment.
- [ ] A rate cut raises credit growth and house prices.
- [ ] The advisory rule recommends higher rates during a demand boom with inflation above target.
- [ ] Repeated monetary financing reduces credibility and raises expectations.
- [ ] The rate cannot go below the floor.

## Open questions
- [ ] When independence arrives later: should an independent CB be able to refuse to buy bonds in a debt crisis (forcing default)?
- [ ] Euro adoption as a long-term option (gives up the policy rate and exchange rate)?
