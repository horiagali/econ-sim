---
id: society/opinion-approval
title: Opinion, Ideology & Approval
status: draft
owner: horia
depends_on: [society/population-groups, society/interest-groups, economy/households, society/poverty-inequality, economy/labor-market, economy/prices-inflation, game/world-loop, adr/0008-explainability-architecture, adr/0017-time-base]
research: [research/approval-pocketbook]
updated: 2026-10-26
---

# Opinion, Ideology & Approval

## Purpose
Each person has political leanings, interests and a view of the head of state. **In v1 approval is just a number:** it is computed, shown and explained, but it does not feed back into behaviour or the economy. It's the politics-ready output for later.

## Real-world basis
- Economic voting: approval tracks real income growth, unemployment and inflation (inflation is very unpopular, even with rising wages).
- Pocketbook (own finances) and sociotropic (national economy) evaluation both matter.
- Ideology and group identity shape which policies people like regardless of personal gain.
- Loss aversion: losses hurt approval more than equal gains help.

## Per-person opinion state
| code_name | Meaning |
|---|---|
| `ideo_econ` | Economic axis: −1 (state-led, redistribution) … +1 (market, low taxes) |
| `ideo_social` | Social axis: −1 (progressive) … +1 (traditional) |
| `ideo_national` | National axis: −1 (pro-EU, open) … +1 (sovereigntist, closed) |
| `salience[issue]` | Weight on issues: inflation, jobs, own income, taxes, pensions, health, education, crime, environment, corruption (later), immigration, national independence |
| `interest[g]` | Interest-group intensities ([interest-groups](interest-groups.md)) |
| `approval` | 0–100 approval of the head of state |

Ideology starts from attributes (age, education, religiosity, urban/rural, ethnicity, income, region) per research, and drifts slowly with experience (long unemployment, rising wealth, emigration of family).

## Approval update (per person, per tick)
Target approval is a sum of separately stored components:
```math
A^*_i = 50 + \underbrace{\alpha_m M_i}_{\text{pocketbook}} + \underbrace{\alpha_s S}_{\text{national economy}} + \underbrace{\alpha_g \sum_g m_{i,g}\, \text{stance}_g(\text{policy})}_{\text{interest groups}} + \underbrace{\alpha_p \text{Align}_i(\text{policy}, \text{ideology})}_{\text{ideology}}
```
- $M_i$: change in the person's real disposable income (household, equivalised) and personal unemployment risk vs a slowly moving reference point; losses weighted ×2.
- $S$: national inflation, unemployment and growth, weighted by the person's salience.
- Stances and alignment are evaluated only on levers that changed recently, with decaying memory (people forget).

Smoothing: $A_{i,t+1} = A_{i,t} + \lambda_A (A^*_i - A_{i,t})$, clipped to 0–100.

Every component is stored, so the explorer can say: *"Students in Cluj: approval 58 (+8 this year): grant increase +6, lower youth unemployment +3, inflation −1."*

## Outputs
Approval per person → any group's average (weight-averaged), national approval, interest-group approval, components and their trends.

> **Simplification (v1):** approval has **no** behavioural effects (no strikes, protests, extra emigration, or tax evasion from low approval). Those come with the politics layer.

## First version (roadmap M5)
What is built first, on the `World` of [game/world-loop](../game/world-loop.md): the per-person `approval`, driven **only by the person's own finances** (the pocketbook term $\alpha_m M_i$ above, split into four parts). There are no ideology axes, no interest groups, no national-economy term and no stances on levers yet (M14). It is **display only**: nothing in the economy reads it. Research: [approval and own finances](../../01-research/notes/approval-pocketbook.md).

**Periods.** A tick is a day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)); approval is updated at each **monthly close**, after the month's statistics and after [poverty and inequality](poverty-inequality.md) has updated the households' recent real income.

**Who.** Every person of 18 or more (the voters), in private and collective households. Children have no approval and are left out of every average.

**State.**

| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $A_i$ | `approval` | Person's approval, 0–100 (as above) | points |
| $s_{i,k}$ | `approval_component[k]` | The smoothed part *k* of the person's approval: income, unemployment, inflation, hardship, bounds | points |
| $r_h$ | `hh_real_income_reference` | The household's reference real income: what it has been used to | LCU of 2021 / month |

`hh_real_income_recent` ($\tilde y_h$) is [poverty and inequality](poverty-inequality.md)'s: a three-month moving average of the household's money income at prices of day 0.

**Inputs.** The household's recent real income and its hardship flag ([households](../economy/households.md)); whether the person and the other members of the household are looking for work ([labour market](../economy/labor-market.md)); twelve-month CPI inflation at the close ([prices](../economy/prices-inflation.md)); ages ([demographics](../economy/demographics.md)).

**Update rule.** At each monthly close $t$, for each person $i$ of 18 or more in household $h$:
1. **Income** (pocketbook). The real change against the reference, losses counted $\kappa$ times:
```math
g_h = \frac{\tilde y_{h,t} - r_{h,t-1}}{r_{h,t-1}}, \qquad c^{inc}_i = \text{clip}\big(\alpha_y\, g_h \cdot (\kappa \text{ if } g_h < 0 \text{ else } 1),\ \pm c^{max}\big)
```
   $g_h$ is 0 when both incomes are 0, and $+\infty$ (so $+c^{max}$) when only the reference is 0. The reference then moves: $r_{h,t} = r_{h,t-1} + \lambda_r(\tilde y_{h,t} - r_{h,t-1})$, starting at $r_{h,0} = \tilde y_{h,0}$. A person's income component is that of their household (all adults alike).
2. **Unemployment.** $c^{un}_i = -u_{self}$ if the person is looking for work, less $u_{hh}$ for each other member of the household looking for work (at most two counted).
3. **Inflation felt.** $c^{\pi}_i = -\alpha_\pi \max(\pi_{12} - \pi_0,\ 0)$, with $\pi_{12}$ the twelve-month inflation of the close in percentage points (annualised in the first year, as in the month's statistics). Deflation gives nothing.
4. **Hardship.** $c^{hard}_i = -h$ if the household was in hardship at its last decision.
5. **Smoothing,** each part separately: $s_{i,k,t} = s_{i,k,t-1} + \lambda_A (c_{i,k} - s_{i,k,t-1})$; at month 0 (or when a person turns 18 or arrives) $s_{i,k} = c_{i,k}$.
6. **Approval:** $A_i = \text{clip}(50 + \sum_k s_{i,k},\ 0,\ 100)$, and the **bounds** part $s_{i,bounds} = A_i - (50 + \sum_k s_{i,k})$, so that $A_i = 50 + $ the sum of all five parts exactly.

All in integers: points in thousandths (millipoints), ratios in millionths, rounding half away from zero. There is no random draw.

**Aggregates.** At each close, the weighted mean of $A_i$ and of each part over the persons of 18 or more (weight: the household's), nationally and by built-in group: activity, income fifth, development region, county, age band (18–24, 25–34, …, 65 or more), `edu_level`, sex. A group's change from one close to the next is the sum of the changes of its five parts' means exactly (the parts are linear in $A_i$; persons who entered or left the group are a sixth part, "composition"). This is the "why" of [ADR-0008](../../03-architecture/decisions/0008-explainability-architecture.md): *"Pensioners: approval 41 (−6 since June): income −4.1, inflation −2.0, hardship +0.1"*.

> **Simplification:** the income part compares household totals, not income per equivalised adult, so a birth or a member leaving with no income does not read as a loss or a gain; a member leaving with their income does.

> **Simplification:** everyone feels the CPI: every household has the same basket ([households](../economy/households.md), M2), so felt inflation does not differ by group. Pensioners and the poor lose more in an inflation burst only through their real incomes (pensions are indexed once a year).

> **Simplification:** the scale is 50 for a person whose finances have not changed, with no job loss, no hardship and inflation at most $\pi_0$. It is not calibrated to Romanian polls (trust in government was about 20–30% in 2021); the level means nothing on its own, its changes and its differences between groups do.

**Not in M5:** ideology, interest groups, stances on levers, the national-economy term $S$, salience, personal price indices, memory of specific levers, any effect of approval (strikes, protests, emigration, evasion).

**Player levers.** None directly; every lever that moves incomes, jobs or prices moves approval.

**Edge cases.**
- **A household with no income for months:** its reference falls towards zero at $\lambda_r$, so a long-lasting zero is slowly accepted (the income part returns to 0) while hardship and unemployment stay.
- **The first months:** the reference starts at month 0's income, so the income part starts at 0; approval at month 0 is 50 less unemployment and hardship (and inflation above $\pi_0$, none at day 0).
- **A person who turns 18, arrives or returns** (demographics): starts unsmoothed at the targets; the household's reference is kept, or set to its recent income if the household is new.
- **All persons of a group are under 18:** the group has no approval (none), not 0.
- **Extreme inflation** (hyperinflation): the inflation part is not capped, so approval is held at 0 by the bounds part, which the "why" shows.

## Tuning parameters
`alpha_*`, `lambda_A`, loss-aversion factor, memory decay, ideology drift rates, stance tables (data).

First version (M5). All are educated guesses (confidence low), to be tuned against polls in M14; their order of size follows the [research note](../../01-research/notes/approval-pocketbook.md).

| Parameter | Symbol | Default | Range | Effect |
|---|---|---|---|---|
| `approval_income_weight` | $\alpha_y$ ($\alpha_m$ above) | 60 points per unit of relative change | 0–200 | a 10% real gain is +6, a 10% loss −12 |
| `approval_loss_aversion` | $\kappa$ | 2 | 1–4 | losses against gains |
| `approval_income_cap` | $c^{max}$ | 25 points | 5–50 | the largest income part |
| `approval_reference_speed` | $\lambda_r$ | 1/24 a month | 1/60–1/3 | how fast people get used to a new income (about two years) |
| `approval_unemployed_self` | $u_{self}$ | 10 points | 0–30 | being out of work |
| `approval_unemployed_household` | $u_{hh}$ | 4 points per member, at most two | 0–15 | someone else at home out of work |
| `approval_inflation_weight` | $\alpha_\pi$ | 1 point per percentage point | 0–5 | inflation disliked in itself |
| `approval_inflation_neutral` | $\pi_0$ | 3% a year | 0–10% | inflation nobody notices (the central bank's target band: 2.5% ± 1) |
| `approval_hardship` | $h$ | 8 points | 0–30 | could not pay for the subsistence basket |
| `approval_smoothing` | $\lambda_A$ | 0.5 a month | 0.1–1 | how fast approval follows its target |

## Acceptance tests
IDs are stable. The M5 criteria go in `crates/econ-core/tests/acceptance/approval.rs` (protected; written by the test-writer subagent before the code), with a crate feature `approval`. "1:1000" is the Romania fixture scenario at that scale, seed 42, under `Law::law_2021()`; "the baseline" is the same run with no lever changes.

**First version (M5):**
- [ ] **AC-APP-01** `[unit]` The parts for given inputs are the formulas of steps 1 to 4 in millipoints: a 5% real loss against the reference gives exactly $\kappa$ times the magnitude of a 5% gain (unless capped); the cap holds; zero incomes give 0; the reference moves by step 1's rule.
- [ ] **AC-APP-02** `[unit]` Smoothing and bounds: each part follows step 5; $A_i$ stays within 0 and 100; $A_i$ = 50 + the five parts exactly, for any inputs including extreme inflation and a capped income.
- [ ] **AC-APP-03** `[sim]` Isolation: at 1:1000, the `World`'s state hash after 24 months is the same with approval computed and not computed, and with every approval parameter at its lowest and highest value.
- [ ] **AC-APP-04** `[sim]` At 1:1000, raising `transfer.pension.point_value` by 10% from month 3 raises the mean approval of the retired within 3 months against the baseline, by more than that of the employed.
- [ ] **AC-APP-05** `[sim]` At 1:1000, a 30% `fx.devaluation` on month 3 lowers the mean approval of every activity group of at least 50 synthetic adults within 6 months against the baseline, and lowers it more among the retired than among the employed.
- [ ] **AC-APP-06** `[sim]` At 1:1000, every person who loses a job in a month and is still looking for work at the close has approval at least $\lambda_A u_{self}$ lower than at the close before, other things (income, hardship, inflation parts) aside: the unemployment part alone falls by that much.
- [ ] **AC-APP-07** `[sim]` Explanation: at every close of a 24-month run at 1:1000, for the nation and for each built-in group, the change of mean approval equals the sum of the changes of its parts' means plus the composition part, exactly in millipoints.
- [ ] **AC-APP-08** `[unit]` Who counts: persons under 18 have no approval and are not in any mean; a group with no adult has none.

**Later versions (M14):**
- [ ] A sharp rise in inflation lowers approval in almost all groups, most among pensioners and low-income households.
- [ ] Raising the top income tax rate lowers approval among high earners and business owners and raises it among people with left economic ideology, even those who don't benefit materially.
- [ ] A pension increase raises pensioners' approval within 1–3 ticks (first version: AC-APP-04, in months).
- [ ] Approval reacts more to a 5% real income loss than to a 5% gain (first version: AC-APP-01).
- [ ] Changing approval parameters never changes any economic variable (v1 isolation test; first version: AC-APP-03).

## API sketch
First version (M5). Names may change before lock; shapes should not. There is no code yet (tests come first, ADR-0014).

```rust
// crate econ-core, module `approval` (pure functions), feature `approval`
pub struct ApprovalParams {
    pub income_weight_milli: i32, pub loss_aversion_ppm: u32, pub income_cap_milli: i32,
    pub reference_speed_ppm: u32, pub unemployed_self_milli: i32, pub unemployed_household_milli: i32,
    pub inflation_weight_milli: i32 /* per percentage point */, pub inflation_neutral_ppm: i32,
    pub hardship_milli: i32, pub smoothing_ppm: u32,
}
pub enum Part { Income = 0, Unemployment = 1, Inflation = 2, Hardship = 3, Bounds = 4 }
pub struct PersonInputs { pub recent: Bani, pub reference: Bani, pub seeking: bool,
                          pub others_seeking: u8, pub inflation_ppm: i32, pub hardship: bool }
pub fn targets(i: &PersonInputs, p: &ApprovalParams) -> [i32; 4];                     // AC-APP-01
pub fn step(prev: Option<[i32; 4]>, targets: [i32; 4], p: &ApprovalParams) -> ([i32; 5], i32 /*A_i*/); // AC-APP-02
pub fn next_reference(reference: Bani, recent: Bani, p: &ApprovalParams) -> Bani;

// in the World
pub struct GroupApproval { pub adults: u64, pub mean_milli: i32, pub parts_milli: [i32; 5] }
pub struct ApprovalStats { pub month: Month, pub national: GroupApproval,
                           pub by_activity: Vec<GroupApproval>, pub by_fifth: [GroupApproval; 5],
                           pub by_region: Vec<GroupApproval>, pub by_county: Vec<GroupApproval>,
                           pub by_age: Vec<GroupApproval>, pub by_edu: [GroupApproval; 5], pub by_sex: [GroupApproval; 2] }
impl World {
    pub fn approval(&self, person: u32) -> Option<(i32, [i32; 5])>;   // A_i and its parts, millipoints
    pub fn approval_stats(&self) -> &[ApprovalStats];                 // history, one per close
}
```

## Open questions
**First version (M5):**
- [ ] **Household total or per equivalised adult** for the income part? *Proposed:* household total (a birth is not a loss), as in the simplification.
- [ ] **Inflation felt by basket.** With one basket for all, felt inflation is the CPI. Give the poorest fifths a basket heavier in food and energy (their own price index, `hh_price_index`) now, or with the households' baskets by income (M7 or later)? *Proposed:* later, with the baskets; it moves behaviour too, which is not this spec's to change.
- [ ] **Where approval starts.** 50 less the unemployment and hardship parts gives a national mean near 47–48 at the start. Rescale so that the start equals a poll (about 30)? *Proposed:* no, keep 50 as "nothing changed"; a poll-matching offset is M14's, with the national term.
- [ ] **The voters.** Adults of 18 or more resident in Romania; the diaspora votes too (about a million voters abroad in recent elections), but it has no persons in the `World`. *Proposed:* residents only.

**Later versions:**
- [ ] Are three ideology axes enough, or do we want a fourth (e.g. authoritarian vs liberal-democratic)?
