---
id: society/households-lifecycle
title: Household Life Cycle — Leaving Home, Couples, Separation, Weighted Records
status: draft
owner: horia
depends_on: [society/population-groups, society/population-generator, economy/households, economy/accounting, game/world-loop, adr/0003-people-representation, adr/0006-determinism-contract, adr/0007-money-and-ledger, adr/0017-time-base]
research: [research/demographics-romania-2021]
updated: 2026-10-29
---

# Household Life Cycle — Leaving Home, Couples, Separation, Weighted Records

## Purpose
Households form and dissolve: young adults leave their parents, single people form couples, couples separate, a household ends when its last member dies, and children left without an adult are cared for. This spec carries out the "household formation and dissolution, simple rates in v1" row of [population-groups](population-groups.md) for roadmap M5, next to [demographics](../economy/demographics.md) (births, deaths, retirement, estates). It also defines the **operations on weighted records** that every transition of people uses, including migration (`economy/migration-remittances`, a separate spec), and the money rule for each: the ledger rule for splitting and merging household wealth left open by [ADR-0003](../../03-architecture/decisions/0003-people-representation.md) and [ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md).

## Real-world basis
From the [research note](../../01-research/notes/demographics-romania-2021.md): Romanians leave the parental home late (mean age 28.0: men 30.3, women 25.6, 2021), and 38.7% of 25–34-year-olds lived with their parents; about 130,000 marriages and 30,000 divorces a year before the pandemic, with cohabitation common (a third of births are outside marriage). Household size falls slowly as the population ages.

## The weighted-record problem
A synthetic household of weight $w$ stands for $w$ identical real households; a person's weight is the household's ([ADR-0003](../../03-architecture/decisions/0003-people-representation.md)). A transition that moves one person moves all $w$ copies of that person. Two consequences:
- **Moving out is cheap:** a person leaving a household of weight $w$ forms a new household of the same weight $w$. Person and money totals stay exact.
- **Moving in is not:** two persons from households of weights $w_a \ne w_b$ cannot simply share a household. The larger record is first **split** into two records, one of the matching weight. Splitting copies every member, so the life cycle only merges **one-person** households, where a split copies a single person.

Weights at the start are close to `sample_scale` but not equal (the generator rakes them), so splits happen but stay small.

## State variables
| Name | Meaning | Unit | Notes |
|---|---|---|---|
| `hh_key` | stable identity of the household, never reused | id (u64) | the start's index, then a counter; the entity of keyed draws about households |
| `origin_household` | `hh_key` of the household a person left when leaving home | id or none | the kinship link estates use ([demographics](../economy/demographics.md)) |
| `hh_moved_to_date` | money moved into the household by these operations since day 0, net, per household | LCU | keeps AC-HH-03's identity: balance = start + income − spending + moved |
| `hh_alive` | the record counts | bool | an ended household is a tombstone until the yearly compaction |
| `role` | head, partner, child, other | category | exists in the scenario |

`hh_key` is loaded since 2026-10-29: a column of the scenario's `tables/households.arrow` (the start's index), kept by the `World` in `World::people()` with the next free key; the other columns here wait for the life cycle.

Per transition and stratum: the alignment carry ([demographics](../economy/demographics.md#alignment)).

## Inputs
Persons (`age`, `sex`, `role`, `activity`, `employer`, `alive`), households (`hh_weight`, `hh_county`, `hh_collective`, tenure, dwelling value, every balance column); the deaths of the month ([demographics](../economy/demographics.md)); the tuning parameters below.

## Outputs
New, merged, split and ended household records; changed roles; money moved between households (`hh_moved_to_date`); statistics: households (weighted), average household size, persons living alone, young adults living with parents, persons living in a couple, leavers, unions and separations a year.

## Periods
Monthly, in the People phase at month end ([demographics](../economy/demographics.md#periods)): after a death, at once (new head, orphans, estate); then, at step 6 of the phase, leaving home, unions and separations, in that order (a person who leaves home can form a couple the same month). Compaction yearly, with the persons'.

## Record operations
Balances (`hh_cash`, `hh_deposits`, `hh_mortgage`, `hh_consumer_debt`, `hh_dwelling_value` and any balance column added later) are **per real household**. Every operation below keeps Σ weight × balance of each column and the weighted persons by every attribute exact, except where it says otherwise.

| Operation | Used by | What it does | Money |
|---|---|---|---|
| **Add person** | births, immigration | a new person record (new `person_key`) in an existing household | none |
| **Remove person** | deaths, emigration | the person's record becomes a tombstone | none (the household keeps its money) |
| **Move out** | leaving home, separation | person $p$ of household $h$ (weight $w$) goes to a new household $n$ of weight $w$, same county, region, locality size; $p$ becomes its head | per household, an amount $a$ of cash and of deposits moves from $h$ to $n$; debts and the dwelling stay with $h$ |
| **Split** | unions (unequal weights), migration of part of a record | record $h$ of weight $w$ becomes $h$ with $w - k$ and a copy $h'$ with $k$: every member copied (new `person_key`s, same attributes, same employer: the unit's weighted staff does not change) | every balance copied per household, so the totals are unchanged |
| **Merge** | unions | two one-person records of the **same** weight become one: the second's person moves in, the second record ends | balances added per household |
| **End** | the last member died or left | the record becomes a tombstone | only after the estate is settled ([demographics](../economy/demographics.md)) or its money moved |
| **Make collective** | orphans | each remaining member becomes a one-member collective record of the same weight | money and dwelling split as below |

Between records of **different** weights money moves only after a split, or through an estate, whose remainder goes to the government ([demographics](../economy/demographics.md)). Moves between households are not ledger postings: the ledger holds the household sector's totals, which do not change ([world loop](../game/world-loop.md)); they are recorded in `hh_moved_to_date` on both sides.

**A new household** (move out, split, make collective) gets: a new `hh_key`; `hh_stagger_day` by `draw(Schedule, day, hh_key, 0)` in 1 to 28 ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)); no decision yet (`last_decision` none, so it decides on its stagger day); income and spending to date 0; `hh_moved_to_date` the money it received; tenure `tenant` and no dwelling (move out), or a copy of the original's (split). Its income fifth and hardship are set at the next monthly close; its claims to benefits follow [social transfers](../economy/social-transfers.md) (minimum income's take-up is drawn per `hh_key`). Equivalised sizes of every household touched are recomputed ([households](../economy/households.md)).

## Update rule (first version, roadmap M5)

### 1. After a death
- **New head.** If the head died: the partner, if any; else the oldest live member aged 18 or more (ties by `person_key`). Partners who lost their partner keep their record and become head.
- **Orphans.** A private household with live members but none aged 18 or more is **made collective**: each child becomes a one-member collective record of the household's weight (a children's home). Cash and deposits are split equally per child (floor; the remainder to the first child by `person_key`), the dwelling goes to the first child; debts were settled with the estate rule of [demographics](../economy/demographics.md) as if the household had ended.

> **Simplification:** orphans are not taken in by relatives; grandparents raising grandchildren exist only where the starting population put them together.

### 2. Leaving home
**Eligible:** live persons aged 18 to 34 in a private household, with role child or other, not in a couple, in a household whose head or partner is at least 15 years older (the parent proxy: the starting population has no parent links). **Strata:** sex × age band (18–19, 20–24, 25–29, 30–34). **Target:** `leave_home_rate[sex, band]` a year ÷ 12 (below; low confidence, tuned so the mean leaving age is about 25.6 for women and 30.3 for men and 38.7% of 25–34-year-olds live with their parents). **Ranking:** the rate × `leave_home_job_ratio` (2) for the employed, × 0.5 for the unemployed: people with a job leave first.

| `leave_home_rate` | 18–19 | 20–24 | 25–29 | 30–34 |
|---|---|---|---|---|
| women | 3% | 12% | 15% | 12% |
| men | 2% | 5% | 10% | 10% |

**The leaver moves out** with $a$ = `leave_home_share` (0.25) × the household's per-household cash (and, separately, deposits) ÷ the number of its members aged 18 or more, rounded down. `origin_household` = the parents' `hh_key`.

> **Simplification:** a leaver always rents, alone; buying a home, moving into a family-owned dwelling and leaving as a couple are not distinguished. Housing costs do not slow leaving home until the housing market (M13).

### 3. Unions
**Eligible women:** live, aged 18 to 49, alone in a private one-person household. **Eligible men:** live, aged 18 to 59, alone in a private one-person household. **Strata:** the woman's age band (18–24, 25–29, 30–34, 35–39, 40–49). **Target:** `union_rate[band]` a year ÷ 12 over eligible women (15%, 20%, 15%, 10%, 5%; low confidence, tuned so the share of 30–49-year-olds living with a partner stays near its start).

**Matching.** Selected women, in ranking order, each take the unmatched eligible man of the same county whose age is closest to hers + `union_age_gap` (2 years), within 3 years younger to 10 years older than her; ties by the smaller `draw(Demography, day, person_key, union)`. A woman with no such man forms no union; her weight is not counted as selected, so the carry keeps it.

**Merge.** With weights $w_f$ and $w_m$: if they differ, the record with the larger weight is split so that one part has the smaller weight; then the two equal records merge. The merged household is the record of whoever owns a dwelling (the woman's if both or neither do); the other person moves in as partner. Balances add per household; tenure is `owner` if either owned.

### 4. Separations
**Eligible:** private households with a head and a partner of the other sex, the woman aged under 60. **One national stratum. Target:** `separation_rate` (0.9% a year ÷ 12; about 35,000 a year, pre-COVID divorces plus cohabiting separations; low confidence). **The man moves out** with $a$ = `separation_share` (0.5) of the per-household cash and of the deposits; debts, the dwelling and any children stay in the original record, whose woman becomes head. He keeps no survivor right to her pension.

> **Simplification:** children always stay with their mother; debts stay with the dwelling; there is no remarriage rule beyond the union rule (a separated man is again eligible as soon as he lives alone).

## Player levers
None in the first version. Housing (M13), child benefits and student finance (M14) are the natural levers on leaving home and unions.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `leave_home_rate[sex, band]` | table above | 0–40% a year | when young adults leave home |
| `leave_home_job_ratio` | 2 | 1–5 | how much a job speeds leaving |
| `leave_home_share` | 0.25 | 0–1 | share of an adult's part of the parents' money a leaver takes |
| `union_rate[band]` | 15, 20, 15, 10, 5% | 0–40% a year | how fast single people form couples |
| `union_age_gap` | 2 years | 0–5 | the man's preferred age over the woman's |
| `separation_rate` | 0.9% a year | 0–3% | separations of couples |
| `separation_share` | 0.5 | 0–1 | share of the money the leaving partner takes |

All low confidence; calibrated in M7 against the shares in the research note.

## Interactions
- **[Demographics](../economy/demographics.md):** deaths trigger the rules of step 1 and estates; births add persons; alignment and the People phase are defined there.
- **[Households](../economy/households.md):** new households decide on their stagger day; equivalised sizes change, so spending per person changes (two singles who form a couple need less together).
- **[Social transfers](../economy/social-transfers.md):** the minimum income depends on household size and income, so moves change it; collective records receive no child allowance or minimum income.
- **Migration** (`economy/migration-remittances`): uses add, remove, move out and split.
- **Housing (M13):** will add tenure changes, buying and renting, and the effect of housing costs on leaving home.

## Edge cases & failure modes
- **A leaver whose household's money is negative or zero:** takes nothing.
- **A split when the larger weight is 1:** impossible (weights are at least 1 and differ, so the larger is at least 2).
- **No eligible man in a county:** no union there this month; the carry grows and is spent when one appears (the carry is capped at the stratum's eligible weight so it cannot grow without bound).
- **A household whose head is under 18 after a death but which has an adult "other" member:** that adult becomes head.
- **Firm units' member lists:** a split adds the copies to the employer's list; the unit's weighted staff is unchanged because the weights add up.
- **Record growth:** move out and splits add records; deaths, merges and compaction remove them. The ratio of records to real persons must not drift far (AC-HHL-09).

## Acceptance tests
IDs are stable. `[unit]` tests in `crates/econ-mech-demography` (the operations as pure functions on small tables), `[sim]` on the `World`, written by the test-writer ([ADR-0014](../../03-architecture/decisions/0014-agent-workflow-guardrails.md)).

- [ ] **AC-HHL-01** `[unit]` Split: a record of weight $w$ split into $w-k$ and $k$ has every member copied with new keys and identical attributes; Σ weight × balance of every column and the weighted persons by age, sex and activity are unchanged exactly; an employer's weighted staff is unchanged.
- [ ] **AC-HHL-02** `[unit]` Merge: two one-person records of equal weight become one record with both persons and summed balances, totals unchanged; with unequal weights the larger is split first and the leftover keeps its person and balances; records change by −1 household (equal) or 0 households and +1 person (unequal).
- [ ] **AC-HHL-03** `[unit]` Move out: the new record has the old one's weight, county and locality, tenure tenant, no dwelling and no debt; the moved amount is the rule's, rounded down; Σ weight × balance is unchanged; `origin_household` is the old record's key.
- [ ] **AC-HHL-04** `[unit]` Make collective: a household with no member aged 18 or more becomes one collective record per member, of the same weight, with the money split exactly (the remainder to the first child).
- [ ] **AC-HHL-05** `[sim]` Conservation, every month of 20 years at 1:1000: the life cycle's operations change neither the weighted persons nor Σ weight × any balance column (only ledger flows and estates do); every live private household has exactly one head; no live private household lacks a member aged 18 or more after the month's deaths; AC-HH-03's identity holds with `hh_moved_to_date`.
- [ ] **AC-HHL-06** `[sim]` Leaving home, years 1 to 10 at 1:100: the share of persons aged 25–34 living in a household with a head or partner at least 15 years older stays between 30% and 48%; the mean age of leavers is 23 to 28 for women and 27 to 33 for men.
- [ ] **AC-HHL-07** `[sim]` Couples and sizes, at 1:100: after 20 years the share of persons aged 30–49 living with a partner is within 8 points of its start value; the average size of private households stays between its start value − 0.4 and + 0.1.
- [ ] **AC-HHL-08** `[sim]` Separations: each year's weighted separations are within 10% of the target; in each, the man's new household holds `separation_share` of the original per-household cash and deposits (rounded down) and the children stay with the woman.
- [ ] **AC-HHL-09** `[sim]` Records do not explode: in the 50-year exit run of [demographics](../economy/demographics.md) at 1:1000, after each yearly compaction, synthetic persons per real person and synthetic households per real household stay within 25% of their start values.
- [ ] **AC-HHL-10** `[sim]` A household formed in a month has a stagger day from 1 to 28 and makes its first spending decision within the following 28 days; two runs with the same seed form the same households (state hashes equal).

The tests were written on 2026-10-29 by the test writer (ADR-0014), before the code: AC-HHL-01 to 04 in `crates/econ-mech-demography/tests/acceptance/lifecycle.rs` behind that crate's feature `lifecycle` (on `records`: `move_out`, `split`, `merge_singles`, `unite`, `make_collective`, `leave_home_amount`); AC-HHL-05 to 10 in `crates/econ-core/tests/acceptance/households_lifecycle.rs` behind econ-core's feature `lifecycle` (off by default; release-only).

## API sketch
**A proposal.** The operations are pure functions over the column tables, in `crates/econ-mech-demography` (shared with [demographics](../economy/demographics.md)); the World's People phase calls them.

```rust
pub struct Tables<'a> { pub households: &'a mut HouseholdColumns, pub persons: &'a mut PersonColumns, /* the people state */ }
pub struct HhKey(pub u64);
pub struct PersonKey(pub u64);

pub fn add_person(t: &mut Tables, hh: u32, attrs: PersonAttrs) -> PersonKey;
pub fn remove_person(t: &mut Tables, p: u32);                                     // tombstone
pub fn move_out(t: &mut Tables, p: u32, cash: Bani, deposits: Bani, day: Day) -> u32;   // new household index; AC-HHL-03
pub fn split(t: &mut Tables, hh: u32, k: u32, day: Day) -> u32;                   // the new record of weight k; AC-HHL-01
pub fn merge_singles(t: &mut Tables, keep: u32, other: u32) -> Result<(), LifeCycleError>;   // equal weights; AC-HHL-02
pub fn make_collective(t: &mut Tables, hh: u32, day: Day) -> Vec<u32>;            // AC-HHL-04
pub fn compact(t: &mut Tables) -> Remap;                                          // yearly; Remap applied to every stored index

pub enum LifeCycleError { WeightsDiffer, NotOnePerson(u32), Collective(u32), Tombstone(u32) }
pub struct LifeCycleReport { pub leavers: u64, pub unions: u64, pub separations: u64, pub orphans: u64,
                             pub splits: u32, pub households_formed: u64, pub households_ended: u64 }   // weighted, the month
```

Random purposes on `Stream::Demography`: `leave_home`, `union`, `separation`; stagger days on `Stream::Schedule`.

## Open questions
- [x] **Only one-person households merge.** Real couples often form straight from two parental homes. *Proposed: accept; both partners pass through a one-person household, possibly in the same month (leaving home runs before unions). It keeps splits to one person.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Leaving home and unions do not respond to the economy** beyond who leaves first (people with jobs). *Proposed: accept for M5; housing costs and income join with M13 and M14.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Orphans become collective records.** *Proposed: accept; the number is tiny (children whose every adult household member dies).* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Old people moving in with their children or into care homes.** *Proposed: not in the first version; revisit with M14 (health and care).* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Same-sex couples.** Romania does not recognise them legally and the census does not count them. *Proposed: not modelled in the first version.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **The rates are guesses** (leaving home, unions, separations). *Proposed: accept as low-confidence defaults, tuned in M7 to the shares of AC-HHL-06 and 07.* Decided as proposed (owner's delegation, 2026-10-26).
