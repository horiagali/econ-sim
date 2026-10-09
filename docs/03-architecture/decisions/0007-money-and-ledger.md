---
id: adr/0007-money-and-ledger
title: "ADR-0007: Money and the accounting ledger"
status: accepted
owner: horia
depends_on: [adr/0005-simulation-core-architecture, adr/0006-determinism-contract, economy/accounting, adr/0003-people-representation]
updated: 2026-10-09
---

# ADR-0007: Money and the accounting ledger

## Context
Pillar 1 says money never appears or disappears without an accounting entry. The [accounting spec](../../02-design/economy/accounting.md) already requires double entry, reason codes and per-tick invariant checks, and leaves open "integer money vs float". Most code will be written by AI agents, and bookkeeping bugs ship even at major studios: Victoria 3's patch 1.1 fixed a trade loop that created value, tariffs paid to the wrong party and an overflow in gold reserves.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (sections "Verifying stock-flow consistency in AI-written code" and "Population scale") found:
- Production ledgers such as TigerBeetle keep one invariant (every debit has an equal and opposite credit), integer amounts, a `code` field for the "why", per-currency ledgers, atomic linked transfers and immutable records corrected only by new entries.
- Storing raw per-household postings would cost about 22.5 MB a month at 1:100, or 13.5 GB over 50 years. Aggregated postings by flow code and sector cost about 230 KB a month with county detail.
- Weighted households transact with banks, government and named firms that are not weighted. Without an explicit rule, weighted household balances and sector balances drift apart, and the drift depends on the scale.

## Options considered
**Money type**
1. **`Bani(i64)` with checked arithmetic** (1 RON = 100 bani). Exact, fast, deterministic. Rates must convert through a rounding rule.
2. **`f64` with periodic reconciliation.** Simple. Drift over decades; balances never exactly zero; invariants need tolerances.
3. **Decimal or fixed-point crates (`rust_decimal`, `fixed`).** Exact. Slower, pre-2.0 APIs, heavier for agents.

**Ledger storage**
1. **Aggregated postings** (flow code × sector, optionally × county). Small; enough for national accounts and the "why" panel.
2. **Per-household posting log.** Full detail; 13.5 GB per 50-year game.

**Scale handling**
1. **Per-unit balances, multiplied by the integer weight at posting time.** Exact at every scale.
2. **Multiply by a global constant when aggregating.** Breaks if weights vary (oversampling) and hides scale in the code.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1 in each group.**

**Money**
- Money is `Bani(i64)`. Arithmetic is checked and panics on overflow in every build, including release.
- Products of an amount and a rate use `i128` intermediates. Rates stay `f64` and convert to bani through one named rounding function.
- Splits (for example, a tax bill across members) use largest remainder with an index tiebreak, so parts always sum exactly.
- clippy `arithmetic_side_effects` and `cast_possible_truncation` are enabled in the money crates.

**Ledger**
- Behaviour code can move money only through `econ-ledger`'s typed `Transfer { debit, credit, code, amount }` with `amount > 0`. Writing a balance directly is impossible from outside the ledger crate. Postings are `#[must_use]`.
- **Flow codes** (the accounting spec's reason codes, such as `tax.income`) are `u16` values in ranges per module, generated from the glossary. They are append-only: never renumbered or reused, so old saves stay readable. About 150 codes are expected.
- A transaction with several legs is linked and applies completely or not at all. Example: a wage is gross wage, contributions, income-tax withholding and net pay in one linked transaction.
- RON and EUR are separate ledgers joined through an FX account.
- Any residual when reconciling the opening balance sheet goes to a named `9xxx` discrepancy account, never into a silent plug.
- Postings are immutable; corrections are new postings.
- Storage is aggregated: tag accumulators by flow code × sector (× county where needed). Raw postings are kept only in a debug buffer of 1–3 ticks and rebuilt by replay when needed.

**Weighted postings (scale independence)**
- Every synthetic household and person record carries an integer weight (`hh_weight`): the number of real units it represents. Weights come from the scenario, never from a constant.
- Record-level balances mean "per represented unit".
- Every posting between a weighted record and a sector-level counterparty (bank, government, named firm) is multiplied by the record's integer weight at posting time, in `i128`, then checked back into `Bani`.
- Result: the weighted sum of household deposits equals bank deposit liabilities exactly, and every invariant holds at any `sample_scale`.

**Postings between two weighted records** (added 2026-10-09 by owner decision, with [ADR-0016](0016-firm-representation.md))
- Cohort firm units carry an integer `firm_count` and use the same weighted-posting rule with sector-level counterparties.
- When both sides are weighted (a synthetic household paid by, or buying from, a cohort firm; dividends from cohort firms to households; purchases between cohort firms), money goes through a **per-industry clearing account**: `wage_clearing[j]` for wages, `sales_clearing[j]` for sales of good *j* (and the same pattern for dividends).
- Wages: firm units post their total wage bill (per-firm amount × `firm_count`) to `wage_clearing[j]`; households receive from it per person (per-person amount × `hh_weight`).
- Sales: household spending posts to `sales_clearing[j]` (per-household amount × `hh_weight`); firm units receive shares by market share.
- The household side defines the total; it is split across firm units by largest remainder on unit totals. A unit total that is not a multiple of `firm_count` changes the per-firm balance by the integer quotient and keeps the rest in the unit's carried remainder `posting_remainder[f]` (< `firm_count` bani), so totals stay exact.
- Every clearing account must net to exactly zero at the end of every tick (I-8).

**Invariants** (every tick in debug and test builds; on save and every N ticks in release)
1. I-1: every posting is balanced.
2. I-2: account-type constraints hold (for example, no negative deposits without an overdraft instrument).
3. I-3: transaction-flow matrix rows and columns sum to zero, and Δstock = Σ flows + revaluations.
4. I-4: independent balance assertions match, computed by a different code path (for example, bank deposit liabilities = sum of weighted agent deposits).
5. I-5: postings are immutable and replay byte for byte.
6. I-6: explanation sums equal the observed change ([ADR-0008](0008-explainability-architecture.md)).
7. I-7: GDP by production, expenditure and income agree exactly in bani.
8. I-8: every clearing account (`wage_clearing[j]`, `sales_clearing[j]` and the dividend clearing account) nets to exactly zero at the end of the tick.

Because money is integer, these checks are exact. The `1e-6 × GDP` tolerance in the accounting spec becomes zero once this ADR is accepted.

## Consequences
- Easier: leaks are caught on the first tick they happen; the "why" panel's L1 layer and the national accounts come straight from flow-code sums.
- Easier: the same code runs at 1:1000, 1:100 or 1:10, and small groups can be oversampled with lower weights without code changes.
- Harder: every mechanic must name a flow code and post through the ledger. A new code is a glossary change.
- Harder: wrong-but-balanced bugs (a tax booked to the wrong sector, a sign error that still balances) are not caught by invariants; [ADR-0010](0010-verification-and-testing.md) adds a differential reference model for those.
- The accounting spec's open question "integer money vs float" is answered by this ADR; the spec should be updated when this ADR is accepted.
- Revisit if the number of codes or the sector sparsity differs a lot from the ~150-code estimate.

## Open questions / to verify
- [ ] The explicit ledger rule for splitting household wealth when households split or merge (from the population research).
- [ ] Where government deposits sit (central bank only, or also commercial banks); open in the [accounting spec](../../02-design/economy/accounting.md).
- [x] Cohort firm units carry an integer `firm_count` and use the same weighted-posting rule; postings between two weighted records go through per-industry clearing accounts checked by I-8 ([ADR-0016](0016-firm-representation.md), accepted 2026-10-09).
- [ ] Spike 2 exit criterion: core "SIM mode" matches the Python reference model exactly in bani for 200 ticks.
- [x] The [population-groups spec](../../02-design/society/population-groups.md) now lists `hh_weight` as an integer.
