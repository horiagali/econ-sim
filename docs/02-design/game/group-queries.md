---
id: game/group-queries
title: Group Queries — Any Filter over Persons
status: draft
owner: horia
depends_on: [game/information, game/statistics-registry, society/population-groups, society/poverty-inequality, society/opinion-approval, adr/0003-people-representation, adr/0011-storage-saves-history]
research: []
updated: 2026-11-01
---

# Group Queries — Any Filter over Persons

## Purpose
Roadmap M6, second slice (owner, 2026-11-01). The [group explorer](information.md) (view 3) builds any group by filtering persons on their attributes, or breaks a group down by an attribute, and shows what the group earns, pays, receives and thinks. This spec is the query behind it, on the `World`, read-only. M8 draws it.

## The filter
A **filter** is a conjunction of conditions, each on one attribute; a condition lists the values it accepts (or a range, for age). An empty filter is every resident.

| Attribute | Values | Source |
|---|---|---|
| `county`, `region` | the scenario's county and development-region indices | the household's (kept by the `World` from the scenario; moves change it) |
| `urban` | urban or rural | the household's locality type at load (moves keep it) |
| `age` | a range of whole years `[from, to)` | the person's age |
| `sex` | F, M | — |
| `edu` | the five `edu_level`s | — |
| `activity` | the seven activities ([population attributes](../society/population-attributes.md)) | the refreshed activity of each close |
| `occupation` | ISCO major groups 0 to 9, or none | — |
| `industry` | the industry of the person's employer unit, or none | — |
| `fifth` | the household's income fifth (`hh_fifth`) | — |
| `class` | the household's standard-of-living class ([poverty](../society/poverty-inequality.md), step 6) | feature `poverty` |
| `enrolled` | the eight values of `enrolled` | — |

Only **residents** are in a group (persons alive, in records of positive weight, not abroad). A person is counted with its household's weight. A household quantity (money, income) is shared equally among the household's resident members, so a group's money adds up across a partition exactly when the household belongs to one part (a whole household), and to the ban within each household otherwise (the remainder of the equal split goes to the member listed first).

## The summary
For a filter, at the latest close:

| Field | Meaning |
|---|---|
| `persons`, `households` | weighted persons; weighted households with at least one member in the group |
| `records` | the synthetic sample behind it: person rows and household rows ([ADR-0003](../../03-architecture/decisions/0003-people-representation.md)); a view on fewer than `min_records` (default 30) person rows is flagged |
| `employed`, `unemployed`, `unemployment_rate` | by the refreshed activity |
| `wages_gross`, `wages_net` | the month's pay slips of the group's persons, weighted (lei) |
| `pensions`, `unemployment_benefits` | the month's pay slips, weighted |
| `household_transfers` | child allowance and minimum income of the month, shared per member |
| `taxes_paid` | the person's income tax, pension and health contributions of the month, and the household's self-employment tax shared per member |
| `income_per_adult` | the mean equivalised recent real income of its households, weighted by the group's persons ([poverty](../society/poverty-inequality.md), feature `poverty`) |
| `money`, `debt` | the households' cash and deposits, and mortgages and consumer debt, shared per member |
| `sol` | the mean SoL of the group's persons (feature `poverty`) |
| `approval` | the mean approval of the group's adults (feature `approval`) |

A **breakdown** of a filter by an attribute returns one summary per value of the attribute (with the filter's other conditions), in the attribute's order.

## Watched groups
The UI can **watch** a filter: its summary then joins the statistics history at every close from then on ([statistics registry](statistics-registry.md); the registry's indicators `group.<name>.persons`, … are not registry entries but a separate list of watched series). "How the group changed since a chosen date" reads a watched group's history; for an unwatched group it needs a replay from a save (after M8's saves).

## API sketch
Names may change before lock; shapes should not. Tests come first (ADR-0014).

```rust
// crate econ-core, module `groups` (feature `groups`)
pub enum Condition { County(Vec<u8>), Region(Vec<u8>), Urban(bool), Age { from: u16, to: u16 },
    Sex(Vec<Sex>), Edu(Vec<EduLevel>), Activity(Vec<Activity>), Occupation(Vec<u8>),
    Industry(Vec<Option<u16>>), Fifth(Vec<u8>), Class(Vec<u8>), Enrolled(Vec<Enrolled>) }
pub struct Filter { pub conditions: Vec<Condition> }
pub enum Attribute { County, Region, Urban, AgeBand, Sex, Edu, Activity, Occupation, Industry, Fifth, Class, Enrolled }
pub struct GroupSummary { pub persons: u64, pub households: u64, pub person_records: u32, pub household_records: u32,
    pub small_sample: bool, pub employed: u64, pub unemployed: u64, pub unemployment_rate_ppm: u32,
    pub wages_gross: Bani, pub wages_net: Bani, pub pensions: Bani, pub unemployment_benefits: Bani,
    pub household_transfers: Bani, pub taxes_paid: Bani, pub money: Bani, pub debt: Bani,
    pub income_per_adult: Option<Bani>, pub sol_milli: Option<u32>, pub approval_milli: Option<i32> }
impl World {
    pub fn group(&self, f: &Filter) -> GroupSummary;
    pub fn group_breakdown(&self, f: &Filter, by: Attribute) -> Vec<(u32, GroupSummary)>;
    pub fn watch_group(&mut self, name: &str, f: Filter);          // recorded from the next close
    pub fn watched(&self, name: &str) -> Option<&[GroupSummary]>; // one per close since watched
}
```

## Acceptance tests
IDs are stable. Tests in `crates/econ-core/tests/acceptance/groups.rs` (protected; the test-writer's), feature `groups`.

- [ ] **AC-GRP-01** `[sim]` At 1:1000 under the 2021 law, after 3 closes: the empty filter's `persons` equals `people.population` of the registry, its `employed` and `unemployed` equal the month's `employed` and `unemployed`, and its wages, pensions and benefits equal the pay slips' weighted national sums of the month exactly.
- [ ] **AC-GRP-02** `[sim]` Partitions: breaking the empty filter down by `region`, `county`, `sex`, `edu`, `activity`, `fifth`, `urban`, `industry` and age bands, the members' `persons`, `employed` and `unemployed` add up to the whole group's exactly, and every money field exactly for `region`, `county`, `edu`, `activity`, `fifth`, `urban` and `industry` and to within one ban per household of the group for `sex` and age bands. *(Wording fixed 2026-11-01: `sex` was listed in both halves.)*
- [ ] **AC-GRP-03** `[sim]` Consistency with the M5 indicators: with `poverty` and `approval` compiled, a group by one region has the `sol_milli` and `approval_milli` of that region in the poverty and approval stats, to within one thousandth of a point; its persons equal the registry's `people.population` for the region.
- [ ] **AC-GRP-04** `[unit]` Filters: two conditions on the same attribute intersect; a condition with no value gives an empty group (all fields zero, `small_sample` true); a group of fewer than `min_records` person records is flagged; an age range `[from, to)` includes `from` and not `to`.
- [ ] **AC-GRP-05** `[sim]` Watched groups: a filter watched on day 0 has one summary per close, each equal to the group's summary at that close; watching changes no state (the state hash after 6 closes is the same with and without).
- [ ] **AC-GRP-06** `[bench]` A summary of the empty filter at 1:100 takes at most 50 ms in a release build, and a breakdown by county at most 200 ms.

## Implementation status
- **2026-11-01:** built (feature `groups`, off by default; `world/groups_state.rs`). The `World` keeps each household's locality type (`hh_urban`, now read from the scenario's households table into `StartState`) and county; the life cycle's new records copy them from the record they are made like. Age bands of the breakdown: 0–14, 15–24, …, 55–64, 65 and over. Debt is the banks' mortgages and consumer debt of the household (feature `banking`, on by default). The approval mean of a group is computed as the approval stats' (the five parts' rounded means plus 50), so a region's group equals the stats. Watched groups are recorded at the end of each close's day. AC-GRP-01 to 06 pass (AC-GRP-01 reads the month's employment without the People phase, as it states).

## Open questions
- [x] **Household quantities per person.** *Proposed (2026-11-01):* shared equally among the resident members (the household is the unit of money; an equivalence scale would make groups of children look poor in a way the player cannot read).
- [x] **Change since a date for any group.** *Proposed:* watched groups now; any group by replay after M8's saves.
