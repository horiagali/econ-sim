---
id: economy/monetary-policy
title: Monetary Policy & Central Bank
status: locked
owner: horia
depends_on: [economy/money-banking, economy/prices-inflation, economy/fiscal-policy, economy/opening-balance-sheet, game/world-loop, adr/0017-time-base]
research: []
updated: 2026-10-20
---

# Monetary Policy & Central Bank

## Purpose
The central bank (modelled on the National Bank of Romania, BNR) sets the policy rate and other tools that heat up or cool down the economy, affect inflation, the exchange rate and asset prices. **In v1 the player controls the central bank directly.** If Romania adopts the euro, the policy rate is set by the ECB (exogenous) and only macroprudential tools remain; see [euro](euro.md).

## Real-world basis
- Transmission: policy rate → market rates → borrowing, saving, asset prices, exchange rate → demand → output and employment → inflation. Peak effect on output after ~1 year, on inflation after ~1–2 years.
- Taylor rule as a description of independent central banks.
- Credibility matters: credible central banks disinflate at lower output cost.
- Monetary financing of deficits is inflationary when large and persistent.
- **The BNR at the end of 2021** (from memory, medium confidence): policy rate 1.75%, a corridor of ±1 point (lending facility 2.75%, deposit facility 0.75%), inflation target 2.5% ± 1 point, inflation 8.2% in December 2021 and rising. In the opening balance sheet it holds 228.5 billion lei of foreign assets (reserves, gold and SDR) against 109.2 billion of currency, 46.7 billion of bank reserves and 41.7 billion of government deposits.

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

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| The policy rate and the corridor | set by a command (lever system, M4), effective on the day given; the advisory rule is computed at each monthly close |
| Interest on reserves and advances | month end, with the banks' interest ([money-banking](money-banking.md)) |
| The central bank's profit to the government | month end |
| Credibility | monthly close, after the CPI |

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $i$ | `policy_rate` | the policy rate | % / year |
| $\pi^{target}$ | `inflation_target` | the target | % / year |
| cred | `cb_credibility` | weight of the target in expectations ([prices](prices-inflation.md)) | 0–1 |
| — | `monetary_financing` | the central bank's claim on the government (government deposits below zero, bonds bought from the government) | LCU |
| — | CB balance sheet | foreign assets, bonds, advances, reserves, currency, government deposits: the ledger's central bank column | LCU |

## Credibility update
```math
\text{cred}_{t+1} = \text{cred}_t + \lambda_c \big[k_1 \cdot \text{track record} - k_2 |\pi - \pi^{target}| - k_3 \cdot \text{monfin}/GDP - k_4 \cdot \text{target changes}\big]
```
clipped to [0, 1]. Feeds expectations ([prices-inflation](prices-inflation.md)).

## First version (roadmap M3)
What is built first, on the `World` of [game/world-loop](../game/world-loop.md).

**In:**
1. **The policy rate** is state, 1.75% at the start, with the corridor ±1 point. Until the lever system (M4) it stays where it starts; it is what [money-banking](money-banking.md) prices from.
2. **Reserves and advances, month end.** The central bank pays the deposit facility rate on reserves above the requirement and charges the lending facility rate on advances (flow code `interest.reserves`).
3. **Profit, month end.** The central bank's income (interest on advances and on bonds it holds; its foreign assets earn `foreign_yield` a year, paid by the rest of the world) less what it pays on reserves is remitted to the government the same day (flow code `transfer.cb_profit`); a loss is borne by its equity.
4. **Monetary financing.** The government's deposits below zero (M2's placeholder) and any bonds the central bank buys from the government count as `monetary_financing`. Once bonds exist ([fiscal-policy](fiscal-policy.md), M3) the government's deposits are kept at or above zero by its bond issues; what remains is a choice of the player (M4).
5. **Credibility, at the monthly close,** by the update rule above, with `track record` the share of the last twelve closes with twelve-month inflation within the target band, and `monfin/GDP` the year's increase of `monetary_financing` over GDP.
6. **The advisory rule** is computed at each close and reported (an output for the UI, M8).

> **Simplification (M3):** no QE or FX intervention (FX intervention comes with [trade-fx](trade-fx.md)); the target band is fixed; the output gap of the rule is GDP against its trend over the last 24 months.

**Not in M3:** commands that move the policy rate, the reserve requirement or the target (M4); QE and QT (M4); independence (later).

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `corridor` | ±1 point | facility rates around the policy rate |
| `foreign_yield` | 0.5% a year | what the central bank's foreign assets earn |
| `credibility_speed` $\lambda_c$ | 0.05 a month | how fast credibility moves |
| $k_1, k_2, k_3, k_4$ | 1, 10, 20, 1 | weights of the track record, the miss, monetary financing and target changes |
| `target_band` | ±1 point | what counts as hitting the target |
| $\rho, r^*, \phi_\pi, \phi_y$ | 0.8, 1%, 1.5, 0.5 | the advisory rule |

## Acceptance tests
IDs are stable (added 2026-10-20). Tests go in `crates/econ-core/tests/acceptance/` (protected).

- [ ] **AC-MP-01** `[unit]` Credibility follows the update rule, clipped to [0, 1]; with inflation inside the band for twelve months and no monetary financing it rises, and it falls with monetary financing.
- [ ] **AC-MP-02** `[unit]` The advisory rule recommends a higher rate when inflation is above target and output above trend than when both are at their normal levels.
- [ ] **AC-MP-03** `[sim]` Every month end, the central bank's profit is remitted to the government exactly, under its flow code; the central bank's net financial assets change only by a loss.
- [ ] **AC-MP-04** `[sim]` Reserves above the requirement earn the deposit facility rate, advances cost the lending facility rate, both to the ban.
- [ ] **AC-MP-05** `[sim]` Repeated monetary financing (a deficit financed by the central bank for 24 months) lowers credibility and raises expected inflation.
- [ ] **AC-MP-06** `[sim]` *(M4)* A 2-point rate hike lowers inflation by a meaningful amount after 12–24 months, with a temporary rise in unemployment.
- [ ] **AC-MP-07** `[sim]` *(M13)* A rate cut raises credit growth and house prices.
- [ ] **AC-MP-08** `[unit]` *(M4)* The rate cannot go below the floor.

AC-MP-01 and 02 (`[unit]`) were written on 2026-10-20 in the mechanic's crate, `crates/econ-mech-monetary/tests/acceptance/`, against the API skeleton, behind the crate feature `mechanic`; AC-MP-03 to 05 in `crates/econ-core/tests/acceptance/monetary.rs`, behind the crate feature `monetary`, against the central bank's read-only state (`World::central_bank`), its parameters (`WorldParams::monetary`) and a start edit (`StartEdit::CentralBankFinancing`) by which AC-MP-05 compares two worlds.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-MP-06: roadmap M4
- AC-MP-07: roadmap M13
- AC-MP-08: roadmap M4

## API sketch
```rust
// crate econ-mech-monetary (no I/O); rates and shares in millionths
pub struct CredibilityParams { pub speed_ppm: u32, pub k1_ppm: u32, pub k2_ppm: u32, pub k3_ppm: u32, pub k4_ppm: u32 }
pub fn credibility_ppm(previous: u32, months_in_band_of_12: u8, miss_ppm: u32, monfin_over_gdp_ppm: u32,
                       target_changes: u8, p: &CredibilityParams) -> u32;                        // AC-MP-01
pub struct RuleParams { pub smoothing_ppm: u32, pub neutral_ppm: i32, pub phi_pi_ppm: u32, pub phi_y_ppm: u32 }
pub fn advisory_rate_ppm(previous: i32, expected_ppm: i32, inflation_ppm: i32, target_ppm: i32,
                         gap_ppm: i32, p: &RuleParams) -> i32;                                    // AC-MP-02
```
The month-end interest and profit postings and the close live with the `World`. New flow codes, append-only: `interest.reserves`, `interest.cb_foreign`, `transfer.cb_profit`.

## Open questions
- [x] **The policy rate before levers (M3):** proposed constant at 1.75%; the alternative is the advisory rule set on autopilot until the player can move it (M4). **Decided (owner, 2026-10-20): as proposed.**
- [x] **Monetary financing once bonds exist:** proposed that the government issues bonds to keep its deposits at or above zero (fiscal policy), so monetary financing is a player's choice from M4. **Decided (owner, 2026-10-20): as proposed.**
- [ ] When independence arrives later: should an independent CB be able to refuse to buy bonds in a debt crisis (forcing default)?
- [ ] Euro adoption as a long-term option (gives up the policy rate and exchange rate)?
