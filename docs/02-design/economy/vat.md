---
id: economy/vat
title: Value-Added Tax (VAT)
status: draft
owner: horia
depends_on: [economy/accounting, adr/0007-money-and-ledger, adr/0014-agent-workflow-guardrails]
research: []
updated: 2026-10-10
---

# Value-Added Tax (VAT)

## Purpose
Compute the VAT on what households buy and move it from households to the government through the ledger. This spec covers the tax itself: rates, categories, rounding and the posting. It was split out of [taxation](taxation.md) (owner decision, 2026-10-10) so that it can be locked on its own; it is the first mechanic that went through the tests-first flow (Spike 9) and its code exists in `crates/econ-mech-tax`.

What VAT does to prices, demand and compliance is not decided here. See Interactions.

## Real-world basis
- VAT is charged on sales at every stage, and a registered firm deducts the VAT it paid on its inputs. The tax therefore ends up on final consumption, and a firm's purchases from other firms carry none on balance.
- A good is in one of four positions: **standard** rate, **reduced** rate, **zero** rate (taxable at 0%; the seller still deducts input VAT) or **exempt** (outside VAT; the seller cannot deduct input VAT, as with financial services, health and education).
- EU law (Directive 2006/112/EC) sets a floor of 15% for the standard rate and limits which goods may have reduced rates ([eu-membership](eu-membership.md)).
- **Romania:** a standard rate and reduced rates on food, medicines, housing and some services. The starting rates belong to the scenario data, not to this spec. *(The scale world uses 21% and 11% as stand-ins; to be verified in a research note before the scenario is built. Confidence: medium.)*
- Romania has one of the largest VAT gaps in the EU. That is modelled in [informal-economy](informal-economy.md), not here.

## State variables
None of its own. VAT collected arrives in the government's account under flow code `tax.vat`; revenue by tax is read from the ledger.

## Inputs
| Input | From |
|---|---|
| Household purchases this tick: net amount per good category and household record, with `hh_weight` | [households](households.md) |
| VAT category of each good | goods catalogue ([industries](industries.md)) |
| The VAT schedule: standard and reduced rate | player lever |

## Outputs
- A ledger transaction per tick: households → government, instrument cash, flow code `tax.vat` (3002), one leg per purchase record, multiplied by the record's weight.
- The total collected this tick.
- Warnings about EU-rule breaches in the schedule.

## Update rule
Per tick (one month). Money is `Bani`; rates are integer basis points (2100 = 21%), so the arithmetic is exact and identical on every machine ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md)).

**Rate of a good.** Standard → `vat_rate_standard`; reduced → `vat_rate_reduced`; zero → 0%; exempt → no VAT.

**VAT on one purchase** of net amount $n$ (bani) at rate $\tau$ (basis points):

```math
v = \left\lfloor \frac{n \cdot \tau}{10\,000} \right\rceil , \qquad \text{gross} = n + v
```

$\lfloor\cdot\rceil$ rounds half away from zero to whole bani.

**Collection.** For every purchase record with $v \neq 0$ and weight $w > 0$, one leg of $v \cdot w$ from households to government. All legs of a tick form one transaction, so either all of it is posted or none. A tick with no VAT posts nothing.

**From a gross budget.** A household that decides how much to spend in total, VAT included, needs the reverse: the largest net amount $n$ with $n + v(n) \le$ budget. Because of rounding, at most one ban of the budget is left unspent per household and category; it stays with the household.

**EU floor.** A standard rate below 15% is accepted and reported as a warning (`BelowEuMinimumStandardRate`). The game does not refuse it: the player may break the rule and face the procedure in [eu-membership](eu-membership.md).

> **Simplification:** VAT is charged on household consumption only. Purchases between firms carry none (as if every firm deducted its input VAT in full, in the same month). The input VAT that exempt sectors cannot deduct in reality is ignored.

> **Simplification:** the government receives the VAT in the tick of the purchase. Real firms remit it a month or a quarter later.

> **Simplification:** zero-rated and exempt goods are the same to the buyer in v1 (no VAT). They are kept apart because they differ for the seller once input VAT is modelled.

## Player levers
| Lever | Range | Lag |
|---|---|---|
| `vat_rate_standard` | 0–40% | next tick |
| `vat_rate_reduced` | 0–40% | next tick |
| VAT category of each good category (standard, reduced, zero, exempt) | — | next tick |

See the [lever catalogue](../game/levers.md).

## Tuning parameters
None. The EU floor (15%) is a fact of EU law, not a tuning value.

## Interactions
- **[households](households.md)** decide what to buy; their budget is gross of VAT.
- **[prices-inflation](prices-inflation.md)** decides how far a rate change passes through to consumer prices; a VAT rise lifts the price level once.
- **[informal-economy](informal-economy.md)** decides which sales are not declared (the VAT gap). Undeclared sales never reach this mechanic.
- **[fiscal-policy](fiscal-policy.md)** reads the revenue. **[eu-membership](eu-membership.md)** reads the warnings and owes the VAT-based part of the EU budget contribution.
- **[taxation](taxation.md)** holds every other tax and the system-level expectations about VAT (revenue rises less than the rate; CPI rises once).

## Edge cases & failure modes
- **Rate 0%, zero-rated or exempt good, weight 0:** no leg is posted.
- **Tiny purchases:** VAT rounds to whole bani per purchase record, so one ban at 21% carries no VAT. Totals are sums of rounded amounts, never a rounded total.
- **The ledger rejects the transaction** (an invariant would break): nothing is posted and the error is returned.
- **Very large amounts:** overflow panics in every build ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md)); it is never silent.
- **Rates above 100%:** computed as written. The lever's range keeps the player below 40%.

## Acceptance tests
IDs are stable; tests live in `crates/econ-mech-tax/tests/acceptance/vat.rs` (protected) and all five pass. `[unit]` = pure function, `[ledger]` = posts through the ledger. (The header of that test file still names `economy/taxation` as the spec; a test-authoring session updates it.)

- [ ] **AC-VAT-01** `[unit]` VAT on a purchase is `rate × net amount`, rounded half away from zero to whole bani; the gross price is net + VAT.
- [ ] **AC-VAT-02** `[unit]` Each good's VAT category decides its rate: standard, reduced, zero (0%) or exempt (no VAT).
- [ ] **AC-VAT-03** `[ledger]` Collecting VAT on a batch of household purchases posts exactly the sum of per-purchase VAT from households to government under flow code `tax.vat`, and all ledger invariants hold.
- [ ] **AC-VAT-04** `[unit]` A standard rate below the EU minimum of 15% is accepted but flagged as an EU-rule breach (ADR: eu-membership compliance), not rejected.
- [ ] **AC-VAT-05** `[unit]` Raising the standard rate from 19% to 21% raises the gross price of a standard-rated good by exactly the VAT difference and leaves zero-rated goods unchanged.

## API sketch
As implemented in `crates/econ-mech-tax/src/vat.rs`.

```rust
pub struct VatRate(pub u32);                          // basis points: 2100 = 21%
impl VatRate {
    pub const EU_MIN_STANDARD: VatRate = VatRate(1500);
    pub const fn percent(p: u32) -> VatRate;
    pub fn gross(self, net: Bani) -> Bani;
}
pub fn vat_on(net: Bani, rate: VatRate) -> Bani;      // rounded half away from zero
pub enum VatCategory { Standard, Reduced, Zero, Exempt }
pub enum VatWarning { BelowEuMinimumStandardRate { rate: VatRate } }
pub struct VatSchedule { pub standard: VatRate, pub reduced: VatRate }
impl VatSchedule {
    pub fn rate_for(&self, cat: VatCategory) -> Option<VatRate>;   // None for exempt
    pub fn vat(&self, net: Bani, cat: VatCategory) -> Bani;
    pub fn gross(&self, net: Bani, cat: VatCategory) -> Bani;
    pub fn warnings(&self) -> Vec<VatWarning>;
}
pub struct Purchase { pub net: Bani, pub category: VatCategory, pub weight: u32 }
pub fn collect_vat(ledger: &mut Ledger, schedule: &VatSchedule, purchases: &[Purchase])
    -> Result<Bani, LedgerError>;                     // total collected
```

The split of a gross budget into net and VAT lives in the scale world for now (`split_gross` in `crates/econ-core/src/scale_spike.rs`); it moves into this crate with the households mechanic.

## Open questions
- [ ] **Lock as it stands?** The five criteria cover the tax itself. Everything about behaviour is in other specs.
- [ ] One reduced rate, or two (Romania has had several at once)? The schedule has one today.
- [ ] VAT on new dwellings and on government purchases: in reality both bear VAT. Add with the [housing](housing.md) and fiscal mechanics, or ignore in v1?
- [ ] Input VAT of exempt sectors (banks, hospitals, schools): ignore in v1 (proposed)?
- [ ] Should breaking the rules on which goods may be reduced-rated also raise a warning, like the 15% floor?
- [ ] Starting rates for the scenario: a short research note with sources.
