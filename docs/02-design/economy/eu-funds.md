---
id: economy/eu-funds
title: EU Funds & Absorption
status: locked
owner: horia
depends_on: [economy/eu-membership, economy/infrastructure, economy/state-capacity, economy/fiscal-policy, economy/budget-2021, economy/trade-fx, economy/industries, game/world-loop, adr/0007-money-and-ledger, adr/0017-time-base]
research: [research/government-revenue-romania-2021]
updated: 2026-10-22
---

# EU Funds & Absorption

## Purpose
EU funds are a large source of investment money for Romania, but only if the state can **absorb** them: prepare projects, co-finance, procure, build and meet milestones. Absorption depends on [state capacity](state-capacity.md). This is a core strategic layer: money is available, the challenge is spending it well.

The full layer is roadmap M18. On 2026-10-21 the owner pulled the money flows into M3: without EU grants the `World` misses about 22.6 billion lei a year of government revenue, and farmers about 9.8 billion of income. The first version below moves 2021's flows. Envelopes, projects and absorption stay for M18.

## Real-world basis
- **Cohesion policy** (ERDF, Cohesion Fund, ESF+, Just Transition Fund): multi-year envelopes, project-based, co-financing required. The "n+3" decommitment rule loses unspent money after a deadline.
- **Common Agricultural Policy (CAP):**
  - direct payments to farmers per hectare (EAGF);
  - rural development programmes (EAFRD), paid through the paying agency AFIR.
- **Recovery and Resilience Facility (PNRR in Romania):** grants and loans paid against milestones and targets (reforms and investments), with a hard end date in 2026.
- Romania has historically had low-to-medium absorption rates compared with other member states.
- **Romania, 2021** ([research note](../../01-research/notes/government-revenue-romania-2021.md)):

| Flow | ESA | Million lei | Confidence |
|---|---|---|---|
| Current grants from the EU to the government (ESF, technical assistance) | D74 from S212 | 6,550.2 | high |
| Capital grants from the EU to the government (ERDF, Cohesion Fund, EAFRD investment, first RRF spending) | D9 from S212 | 16,017.5 | high |
| Direct payments to farmers (EAGF), outside the government in ESA | D39 paid by S212 | ~9,806 | medium |
| Own resources paid to the EU (VAT- and GNI-based) | D76 | 12,105.2 | high |
| Customs duties collected for the EU (traditional own resources) | — | ~1,500 | medium |

- **The direct payments are derived from the input-output table.** Agriculture's net other subsidies on production are 12,185.1 million lei, less the government's own other subsidies (D39, 2,379.1).
- **The cash budget shows more.** The Ministry of Finance's cash budget shows 38.4 billion lei of EU receipts. It includes the direct payments, which the state pre-finances, and timing differences.
- **Net position.** Romania was a net receiver of about 4 to 5 billion euro.

## Model (full design, M18)
### Envelopes
Each programme has:
- total envelope and period;
- eligible project types (infrastructure, energy, education, health, digital, social, agriculture, business support);
- co-financing rate and deadline rule.

One data file per programme.

### Project pipeline (uses the [project system](infrastructure.md))
1. **Preparation:** the player (or ministries automatically, at a default pace) prepares projects for eligible types. Preparation takes months and administrative capacity.
2. **Approval:** probability depends on project quality (state capacity) and compliance status ([eu-membership](eu-membership.md)).
3. **Execution:** a normal project in the project system. The government pre-finances the spending and the EU reimburses it after verification, so cash flow and co-financing hit the budget.
4. **Reimbursement:** the EU pays its share after checks. Irregularities found (more likely with high corruption) lead to **financial corrections**: money withheld or clawed back.
5. **Decommitment:** unspent money past the deadline is lost.

### CAP
- Direct payments per hectare to households and firms with farm land → farm income ([households](households.md), `agri_*` industries).
- Rural development grants via the project pipeline.

### Recovery facility (milestones)
- Payments are tied to milestones: reforms (e.g. pensions, tax administration) and investments. Missing milestones cuts the payment.
- Milestones are lever conditions that the game checks ("tax administration digitalised," "coal plants closed by X").

### Absorption rate
```math
\text{absorption}_t = \frac{\text{funds reimbursed (cumulative)}}{\text{funds available (cumulative)}}
```
Driven by state capacity (admin quality, procurement speed), co-financing room in the budget, construction capacity and compliance.

## Accounting
- **Grants.** Funds are a capital transfer (investment grants) or a current transfer (ESF, technical assistance) from the EU (rest of the world) to the government.
- **Direct payments.** They are a subsidy from the EU to the farmers directly; the government is only the paying agent.
- **Spending.** Co-financing is government spending. Corrections are reverse transfers.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| EU grants to the government; direct payments to farmers | month end, a twelfth of the year's amount, before the treasury's financing |
| The lei value of the month's payments | at the exchange rate in force before the month end's close |

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| — | `eu_grants_current` | current grants from the EU to the government, a year, at the starting exchange rate | LCU / year |
| — | `eu_grants_capital` | capital grants from the EU to the government, a year, at the starting exchange rate | LCU / year |
| — | `eu_agri_payments` | EU direct payments to farm units, a year, at the starting exchange rate | LCU / year |
| — | `eu_net_balance` | the month's EU receipts (the three lines) less own resources paid (`transfer.eu_budget`) | LCU / month (derived) |
| — | `eu_absorption_rate` | (M18) funds reimbursed over funds available | ratio (derived) |

## First version (roadmap M3)
What is built first, on the `World` of [game/world-loop](../game/world-loop.md), with the treasury of [fiscal policy](fiscal-policy.md) and the exchange rate of [trade](trade-fx.md). It acts under the 2021 law; M2's placeholder law has none of it.

**Starting state.**

| Line | Million lei a year | In euro (at 4.9215) | Paid to |
|---|---|---|---|
| `eu_grants_current` | 6,550.2 | 1,330.9 | the government |
| `eu_grants_capital` | 16,017.5 | 3,254.6 | the government |
| `eu_agri_payments` | 9,806.0 | 1,992.5 | the farm units |

- **Grants.** The two grant lines are the 2021 ESA lines (high confidence).
- **Direct payments.** They are agriculture's net subsidies in the table less D39 (medium).
- **Where they live.** All three are scenario data, proposed as a section `eu_receipts` of the budget fixture `python/pipeline/fixtures/budget2021_ro.json`:
  - the fields are `grants_current`, `grants_capital` and `agri_payments`, in lei a year, with their ESA codes and sources;
  - the budget pipeline fills it from `gov_10a_main` and the input-output table;
  - the scenario builder copies it into the scenario's `budget.json`;
  - `econ_io::budget` reads it as `Budget::eu_receipts`, an optional field, absent meaning none.

  It is outside the budget's hash (AC-BUD-11). The same fixture carries [government revenue at 2021 levels](government-revenue-2021.md)'s `revenue_held` and `benefits_held`.
- **Farm units.** The units of the catalogue industries of A01 (`agri_cereals`, `agri_oilseeds`, `agri_fruit_veg`, `agri_vineyards`, `livestock_meat`, `livestock_dairy`).
- **Payment key.** Each farm unit's share of the payments is its share of their starting output, fixed at load.

**In:**
1. **The month's amounts, month end.** Each line's yearly amount becomes twelve monthly amounts in bani by [budget-2021](budget-2021.md)'s rule (remainders on the first months). Each is then multiplied by the exchange-rate index $e/e_0$ in force before the month end's close (trade's close may move the rate after the payments; restated by the owner, 2026-10-22), rounded half away from zero. The EU pays in euro, so a depreciation raises the lei value.
2. **Grants to the government, month end** (before the treasury's financing, [fiscal policy](fiscal-policy.md) step 3). The rest of the world pays the month's current grants under `transfer.eu_grants_current` and capital grants under `transfer.eu_grants_capital`. The three legs of the `World` (`pay_gov_any` from the rest of the world):
   - rest of the world → banks, `deposits`;
   - banks → central bank, `reserves`;
   - central bank → government, `gov_deposits_at_cb`.

   The rest of the world's lei deposits may go negative, as for its bond purchases.
3. **What the government does with them: nothing in particular.**
   - **No earmarking.** The spending the grants finance is already in the budget's lines, which the `World` pays: investment (48.0 billion), capital transfers (16.2), goods and services, and wages of ESF projects. ESA records EU-funded spending gross, at the government.
   - **No pass-through.** Rural development grants to farmers (EAFRD) are inside the capital transfers the treasury pays.
   - **The effect.** The grants lower the deficit and the bonds the treasury issues.
4. **Direct payments to farmers, month end.** The rest of the world pays the month's `eu_agri_payments` to the farm units under `subsidy.eu_agri`, by their key (largest remainder): `deposits`, rest of the world → firms. The units' `deposits` columns move.
   - **Accounts.** It is a subsidy in the unit's profit ([investment and capital](investment-capital.md), step 4, as other income) and in GDP by income (net taxes on production fall).
   - **Not via the government.** It does not pass through the government, as in ESA.
   - **Why it is needed.** The table's agriculture prices already assume these subsidies (its other costs are negative). Without them the farm units lose about 12 billion lei a year.
5. **Balance of payments.**
   - **Where each flow belongs.** Current grants are secondary income. Direct payments are primary income (subsidies, BPM6). Both are in the current account. Capital grants are the **capital account**.
   - **The trade month report.** It counts every non-financing flow of the rest of the world (AC-TFX-03), so all three are inside its `current_account`, and the identity closes unchanged.
   - **Proposed: keep the identity on current plus capital account** (net lending of the rest of the world):
     - the trade month report keeps the sum in `current_account`;
     - it adds `capital_account`, the month's `transfer.eu_grants_capital`, as a memo;
     - AC-TFX-03's wording becomes "current and capital account";
     - the exchange rate's flow term uses the sum, as trade's full design has it ("CA + capital flows").
6. **Report, monthly.**
   - **The fiscal report** (AC-FIS-05) shows the two grant lines as revenue.
   - **The month's statistics** show the direct payments and `eu_net_balance`: receipts less own resources paid (`transfer.eu_budget`, 12.1 billion a year).

> **Simplification (M3):** the 2021 amounts are held every year in euro: no envelopes, programme periods, co-financing, absorption, decommitment or corrections, and no RRF profile (2022–2026 grants were several times 2021's). The month's twelfth replaces the real timing (direct payments mostly October to June, cohesion payments bunched at year end). Customs duties are neither collected nor passed on: no tariffs in M3 ([trade](trade-fx.md)).

**Not in M3:**
- envelopes, the project pipeline, absorption, co-financing, corrections and decommitment (M18, with [state capacity](state-capacity.md) and [infrastructure](infrastructure.md), M11);
- direct payments per hectare to households with farms (M18; in M3 the farm units receive them all);
- RRF milestones and loans (M18);
- suspension of commitments under the fiscal rules ([eu-membership](eu-membership.md), M18).

**Flow codes** (proposed, append-only):

| Code | Name | For |
|---|---|---|
| 4105 | `transfer.eu_grants_current` | current grants from the EU to the government (D74) |
| 4106 | `transfer.eu_grants_capital` | capital grants from the EU to the government (D92) |
| 5002 | `subsidy.eu_agri` | EU direct payments to farm units (D39 paid by the EU) |

## Player levers
- **M3:** none.
- **M18:**
  - Prioritise programmes and project types; prepare projects (choose region and type).
  - Co-financing budget allocation.
  - Invest in absorption capacity (EU-funds management agencies; see [state capacity](state-capacity.md)).
  - Meet or ignore recovery-facility milestones.

## Tuning parameters
| Parameter | Default | Confidence | Effect |
|---|---|---|---|
| `eu_grants_current` | 6,550.2 million lei a year | high | government revenue |
| `eu_grants_capital` | 16,017.5 million lei a year | high | government revenue, capital account |
| `eu_agri_payments` | 9,806.0 million lei a year | medium | farm units' income |
| farm-unit key | starting output | low | which farm units receive what |

## Outputs
- **M3:** grants received, direct payments, the net EU balance, the capital account.
- **M18:** envelope vs committed vs paid by programme, absorption rate, money at risk of decommitment, corrections, contribution to GDP and investment.

## Edge cases & failure modes
- **The rest of the world's lei deposits go negative:** allowed (as for its bond purchases and imports). The central bank's intervention converts them ([trade](trade-fx.md)).
- **A farm unit with no output at load:** its key is 0 and it receives nothing.
- **An amount of 0:** nothing is posted.
- **Rounding:** each month's amount is rounded once, after the exchange-rate index. The farm units' parts add up to it exactly (largest remainder).

## Acceptance tests
IDs are stable (added 2026-10-21).
- `[sim]` tests go in `crates/econ-core/tests/acceptance/eu_funds.rs` (protected), written in a test-authoring session.
- AC-EUF-01's `[unit]` test goes with the monthly rule of `econ-io::budget`, or in the mechanic's crate if one is made.

- [ ] **AC-EUF-01** `[unit]` A year's amount becomes twelve monthly amounts that add up exactly to it at an exchange-rate index of 1. At an index $r$, each month's amount is the monthly amount times $r$, rounded half away from zero.
- [ ] **AC-EUF-02** `[sim]` At every month end on the Romania scenario with the 2021 law:
  - the government's receipts under `transfer.eu_grants_current` and `transfer.eu_grants_capital` each equal the month's amount (at the exchange rate in force before the month end's close; restated by the owner, 2026-10-22), through the three legs;
  - the rest of the world's deposits fall by the same;
  - the farm units receive exactly the month's direct payments under `subsidy.eu_agri`, and no other unit receives any;
  - every ledger check holds.
- [ ] **AC-EUF-03** `[sim]` The trade month report's `capital_account` equals the month's `transfer.eu_grants_capital`. The identity of AC-TFX-03, with the current account read as current and capital account, closes every month end to the ban.
- [ ] **AC-EUF-04** `[sim]` The first twelve months' EU receipts are each within 10% of 2021's (6.55, 16.02 and 9.81 billion lei); the difference is the exchange rate. The fiscal report shows the grants as revenue, and the net EU balance (receipts less `transfer.eu_budget`) is reported every month.
- [ ] **AC-EUF-05** `[sim]` A permanent 10% depreciation of the leu (an exchange-rate edit: $e$ 10% higher from the start, $e_0$ unchanged) raises the lei value of the next month's EU payments by 10%, within one ban per payment. *(Restated by the owner, 2026-10-22: it named the world-constants edit of AC-TFX-04, a change in world euro prices, which does not move the exchange rate and so leaves EU funds in lei unchanged.)*
- [ ] **AC-EUF-06** `[sim]` *(M18)* With low state capacity, absorption stays low and money is lost to decommitment.
- [ ] **AC-EUF-07** `[sim]` *(M18)* Raising administrative capacity raises absorption over 1–3 years.
- [ ] **AC-EUF-08** `[sim]` *(M18)* A suspended commitment (fiscal-rule breach) reduces new fund flows.
- [ ] **AC-EUF-09** `[sim]` *(M18)* EU-funded projects raise construction output and public capital like other projects, but cost the budget only the co-financing share (after reimbursement).

AC-EUF-01 to 05 were written on 2026-10-21, after the code (owner's decision of 2026-10-21), from this spec only:
- **Where.** They are in `crates/econ-core/tests/acceptance/eu_funds.rs`, behind the feature `revenue`; AC-EUF-03 and 05 also need `trade`.
- **What they read:** three test writer's additions, which the implementation fills: the month rule of the API sketch (`econ_core::world::eu_month`, months 0 to 11 as in `Budget::monthly`), each unit's receipts of the day (`World::revenue_today`) and the closed months of revenue (`World::revenue_months`, the net EU balance). AC-EUF-05 uses a fourth, added 2026-10-22: a permanent depreciation of the leu at load (`StartEdit::Depreciation`).
- **Amounts.** The yearly amounts are this spec's table, because the budget fixture has no `eu_receipts` section yet.

### Tests owed
Ready for the test session of M3's revenue once the owner locks the spec (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-EUF-06: roadmap M18
- AC-EUF-07: roadmap M18
- AC-EUF-08: roadmap M18
- AC-EUF-09: roadmap M18

## API sketch
The rule is small enough to live with the `World` (`econ_core::world::treasury`), next to the budget lines it pays:

```rust
pub struct EuReceipts { pub grants_current: Bani, pub grants_capital: Bani, pub agri: Bani }   // a year, at e0
pub fn eu_month(per_year: Bani, month: u8, rate_index_ppm: u32) -> Bani;                       // AC-EUF-01
pub fn farm_key(start_output: &[(usize /* unit */, i64)]) -> Vec<(usize, u32)>;                // ppm, sums to 10^6
```

## New glossary entries
- `eu_grants_current`
- `eu_grants_capital`
- `eu_agri_payments`
- `eu_net_balance` (eu-membership's "net EU balance")
- `capital_account` (trade-fx)

## Open questions
- [x] **Direct payments straight to the farm units** (ESA), or through the government (cash budget, both a receipt and a subsidy)? Proposed: straight. The deficit is the same, and ESA revenue stays comparable with 2021's TR. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Capital grants in the identity of AC-TFX-03.** Proposed: the report keeps current plus capital account as one figure, adds `capital_account` as a memo, and the exchange rate uses the sum. Restating AC-TFX-03's wording is the owner's call (trade is locked); the identity in code does not change. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Held in euro, constant every year?** Proposed: yes for M3. The RRF years (2022–2026, several times 2021's grants) come with M18's envelopes. A run that starts in December 2021 therefore misses the RRF's rise. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Where the amounts live.** Proposed: a section `eu_receipts` in `budget.json`, filled by the budget pipeline from `gov_10a_main`. The budget's hash (AC-BUD-11) covers spending only, so it does not change. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Farm units by output**, or by land? Proposed: by output until land exists (M18, with households' farms). **Decided (owner, 2026-10-21):** as proposed.
- [ ] Do ministries prepare projects automatically by default (proposed, to avoid micromanagement), with the player able to steer? (M18)
- [ ] Model the next EU budget period (after 2027) with an uncertain envelope? (M18)
