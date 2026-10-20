---
id: economy/budget-2021
title: The Starting Budget — Spending by Ministry Line, Public Employment and Wages
status: locked
owner: horia
depends_on: [economy/fiscal-policy, economy/accounting, economy/social-transfers, economy/industries, economy/state-capacity, society/population-employers, game/levers, adr/0007-money-and-ledger, adr/0016-firm-representation, adr/0017-time-base, adr/0018-scenario-and-commands, adr/0014-agent-workflow-guardrails]
research: [research/budget-romania-2021, research/law-in-force-romania-2021]
updated: 2026-10-20
---

# The Starting Budget — Spending by Ministry Line, Public Employment and Wages

## Purpose
Give the scenario the budget in force at the start date (1 December 2021) as data: what the state spends, on which ministry line, in which economic form (wages, goods and services, social benefits by programme, subsidies, interest, investment, other), how many posts it pays and in which public firm unit they sit. It is the starting point of [fiscal policy](fiscal-policy.md): the starting value of every spending lever, the government's monthly payments before any mechanic changes them, and the check that the public units and the public employees of the starting population agree with what the state pays. It adds no behaviour.

## Real-world basis
From the [research note](../../01-research/notes/budget-romania-2021.md):
- **Measured (confidence high):** general government spending in 2021 was 474,031.5 million lei, 40.18% of GDP, by COFOG subgroup and ESA transaction (Eurostat `gov_10a_exp`, `gov_10a_main`). Every published total is kept exactly.
- **Measured (confidence high):** 1,262,318 occupied public posts in December 2021, with the central and local split and the six largest ministries (Ministry of Finance staffing statistics).
- **Guessed (confidence low to medium):** how employers' contributions split between the CAM and imputed contributions by function; the posts of health, social care and the smaller order and justice bodies. Each guess is listed in the fixture's `provenance.guesses` and in the research note.

The table follows the ESA accounts, not the Ministry of Finance's cash execution (459.63 billion lei): ESA is complete, accrual-based and what the national accounts and the EU's fiscal rules use. The cash execution is carried in the fixture for comparison only.

> **Simplification:** the year 2021 stands for the budget in force on 1 December 2021. Spending is spread evenly over the months; Romania's real December (investment and bonus payments bunched at year end) is not reproduced. Confidence: medium that this matters little for a sandbox that starts and keeps running.

## Scope
**This increment:** the budget table and its loader (data), its checks against the published totals, its reconciliation with the firm units, the earnings, the pensions, the input-output table and the starting population, the monthly amounts in bani, the mapping of every cell to a lever and to a flow code, and the Python reference.

**Not in this increment:** revenue (it comes from the [taxation](taxation.md) mechanics and the law in force), debt and its financing, any behaviour (how a ministry spends a changed budget, how headcount follows targets: [labour market](labor-market.md), M4), the effect of spending on service quality ([state capacity](state-capacity.md)).

## The table
One file, `budget2021_ro.json` (pipeline fixture; in the scenario, `budget.json`, see open questions). Amounts are **whole lei a year** in the file and **bani** in code (`Bani(i64)`, ADR-0007). Posts are counts.

### Economic types
Twelve types, in this order in every amount vector. The seven headings are what the player sees.

| # | Type id | ESA | Heading | National total 2021, million lei |
|---|---|---|---|---|
| 0 | `wages` | D11 | wages | 120,015.5 |
| 1 | `employer_contributions` | D12 | wages | 12,642.9 |
| 2 | `goods_services` | P2 | goods and services | 70,481.5 |
| 3 | `social_benefits_cash` | D62 | social transfers | 148,156.7 |
| 4 | `social_benefits_in_kind` | D632 | social transfers | 8,342.7 |
| 5 | `subsidies` | D3 | subsidies | 6,077.7 |
| 6 | `interest` | D4 | interest | 17,323.8 |
| 7 | `investment` | P5 | investment | 48,178.8 |
| 8 | `non_produced_assets` | NP | investment | −140.0 |
| 9 | `current_transfers` | D7 | other | 25,712.6 |
| 10 | `capital_transfers` | D9 | other | 16,222.0 |
| 11 | `other_taxes` | D29+D5+D8 | other | 1,017.3 |
| | **Total** | TE | | **474,031.5** |

By heading: wages 132,658.4; goods and services 70,481.5; social transfers 156,499.4; subsidies 6,077.7; interest 17,323.8; investment 48,038.8; other 42,951.9.

### Budget lines
Thirteen lines, one per ministry line of the game. Each is a set of COFOG subgroups; together they are a partition of the 80 subgroups, so the lines add up to the total exactly. Research and development of every function is one line, and public debt transactions are their own line.

| Line id | COFOG | Total, million lei | Starting lever (see "Player levers") |
|---|---|---|---|
| `administration` | GF01 less GF0104, GF0105, GF0107 | 38,797.5 | Budget per ministry: administration |
| `public_debt` | GF0107 | 17,637.3 | none (interest follows the debt) |
| `defence` | GF02 less GF0204 | 17,029.3 | none in v1 (military budget is not in v1); held at its starting value |
| `police_justice` | GF03 less GF0305 | 31,830.2 | Budget per ministry: police and justice |
| `economic_affairs` | GF04 less GF0405, GF0406, GF0408 | 32,765.6 | proposed key of Budget per ministry |
| `transport` | GF0405, GF0406 | 37,484.9 | Budget per ministry: infrastructure maintenance |
| `environment` | GF05 less GF0505 | 8,448.0 | Budget per ministry: environment |
| `housing_community` | GF06 less GF0605 | 13,598.8 | proposed key of Budget per ministry |
| `health` | GF07 less GF0705 | 64,998.0 | Budget per ministry: health |
| `culture` | GF08 less GF0805 | 10,915.4 | Budget per ministry: culture |
| `education` | GF09 less GF0907 | 37,986.3 | Budget per ministry: education |
| `social_protection` | GF10 less GF1008 | 158,572.5 | Budget per ministry (proposed key) for its own running; the social-transfer levers for its benefits |
| `research` | GF0104, GF0105, GF0204, GF0305, GF0408, GF0505, GF0605, GF0705, GF0805, GF0907, GF1008 | 3,967.7 | Budget per ministry: research |

Each line holds an amount for each of the twelve types (the fixture's `lines[].spending`). The fixture also holds the same amounts by COFOG division (`by_function`), each equal to Eurostat's published division total.

### Social benefits by programme
| Programme id | COFOG | Cash + in kind, million lei | Lever |
|---|---|---|---|
| `old_age` | GF1002 | 111,286.2 | State pension |
| `survivors` | GF1003 | 1,652.6 | State pension |
| `sickness_disability` | GF1001 | 10,552.3 | none in the catalogue; proposed: Disability benefit |
| `family_children` | GF1004 | 19,280.5 | Child benefit (with the child-raising leave benefit) |
| `unemployment` | GF1005 | 603.3 | Unemployment benefit |
| `housing` | GF1006 | 74.3 | Housing benefit (M13) |
| `social_exclusion` | GF1007 | 2,384.6 | Minimum income / social assistance |
| `social_protection_other` | GF1009 (and GF1008) | 1,520.6 | none; held |
| `health_in_kind` | GF07 | 6,218.7 | follows Budget per ministry: health |
| `education_grants` | GF09 | 2,926.3 | Student finance (M14); held until then |

They add up to D62 + D632 exactly (156,499.4).

### Public employment
Occupied posts in December 2021 by sector, and the public firm unit ([industries](industries.md), "The firm units") whose employees they are.

| Sector | Posts | Public firm unit | Confidence |
|---|---|---|---|
| `education` | 290,417 | `education_services.public` | high |
| `health` | 243,421 | `health_services.public` | low |
| `defence` | 75,855 | `defence_services.public` | high |
| `police_justice` | 178,548 | `public_administration.public` | medium |
| `central_administration` | 184,321 | `public_administration.public` | medium (residual) |
| `local_administration` | 159,756 | `public_administration.public` | medium (residual) |
| `social_care` | 130,000 | `social_care.public` | low |
| **Total** | **1,262,318** | | high |

Per public unit (the fixture's `public_units`): posts, `wages` (D11) and `employer_contributions` (D12), lei a year. A COFOG subgroup's compensation goes to the unit of its division (GF02 defence, GF07 health, GF09 education), GF1001, GF1004, GF1006 and GF1007 go to social care, the rest to public administration.

| Unit | Posts | Wages, million lei | Employers' contributions | Wages per post a month, lei |
|---|---|---|---|---|
| `defence_services.public` | 75,855 | 5,898.4 | 4,109.7 | 6,480 |
| `education_services.public` | 290,417 | 24,358.8 | 548.1 | 6,990 |
| `health_services.public` | 243,421 | 27,082.4 | 609.4 | 9,271 |
| `public_administration.public` | 522,625 | 56,281.7 | 7,231.9 | 8,974 |
| `social_care.public` | 130,000 | 6,394.1 | 143.9 | 4,099 |
| **Total** | **1,262,318** | **120,015.5** | **12,642.9** | 7,923 |

> **Simplification:** employers' contributions are the CAM (2.25% of wages) everywhere plus imputed contributions (the counterpart of the special pensions of the army and the police) put 40% on defence and 60% on public order. Confidence low for the split; it moves money between "wages" and "employer contributions" of two lines and nothing else.

> **Simplification:** posts are posts, not persons and not full-time equivalents; the Ministry counts occupied posts at the end of the month. One post is one employee here. Confidence medium.

## State variables
None that change: the table is scenario data, read once. Names (see the [glossary](../../glossary.md)):

| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $S_{l,k}$ | `budget_spending[l,k]` | Spending of budget line *l* on economic type *k* in the starting budget | LCU / year |
| — | `public_posts[f]` | Occupied public posts that public firm unit *f* stands for at the start | posts |
| $W^{g}$ | `public_wage_bill` | Wages the government pays its employees (D11), from the public units' employees | LCU / tick |

## Inputs
- The fixture `budget2021_ro.json` (pipeline, from Eurostat and the Ministry of Finance).
- The firm-unit file (`firm_units2021_ro.json`): the five public units, their persons employed and wages.
- For the reconciliation only: the earnings file, the pension file, the input-output table, and the starting population after Stage J.

## Outputs
- The starting value of every spending lever, written to `law.json` by the scenario builder (ADR-0018).
- The budget table in the scenario (`budget.json`), read by fiscal policy for the government's monthly payments until a lever changes a line.
- The headcount target of each public unit at the start: `public_posts[f]`.
- A section of the scenario builder's consistency report: every check below with its measured value.

## Update rule
There is no per-tick rule in this increment. Two rules are fixed here because every user of the table needs them.

**Monthly amounts.** An amount $A$ in lei a year becomes twelve monthly amounts in bani: $q = \lfloor 100|A| / 12 \rfloor$, $r = 100|A| \bmod 12$; month $m$ (0 to 11, counted from the start month) gets $q + 1$ if $m < r$, else $q$, with the sign of $A$. The twelve add up to $100A$ exactly. A changed annual amount restarts the twelve from the month it takes effect.

**Ledger mapping.** Until the mechanics of fiscal policy exist, the government's spending in a month is the monthly amount of every cell, posted under the flow code of its type. Postings are from the government to the payee named below; the ledger's invariants (I-1 to I-7, [accounting](accounting.md)) hold after each.

| Type | Paid to | Flow code |
|---|---|---|
| `wages` | the public units, which pay their employees | `gov.purchases` (1002) from government to public unit for the non-market output at cost (ADR-0016); then `wages` (2001) from unit to persons |
| `employer_contributions` | CAM part: the public unit pays it to the government; imputed part: nothing moves | proposed `tax.employer_contribution` for the CAM; the imputed part is not posted (its cash is the special pensions inside `social_benefits_cash`) |
| `goods_services` | the public units, which buy inputs from industries | `gov.purchases` (1002) to the unit; the unit's purchases use production's intermediate code (not yet in `codes.rs`; production to propose) |
| `social_benefits_cash` | households, by programme | proposed `benefit.*` codes, one per programme |
| `social_benefits_in_kind` | the firms that deliver (pharmacies, private clinics) | proposed `benefit.in_kind` |
| `subsidies` | firm units | proposed `subsidy.production` |
| `interest` | holders of government debt | proposed `interest.gov_debt` |
| `investment`, `non_produced_assets` | industries that build (construction, equipment) | proposed `gov.investment` |
| `current_transfers` | the EU (D76) and others | proposed `transfer.eu_budget`, `transfer.other_current` |
| `capital_transfers` | firms and households | proposed `transfer.capital` |
| `other_taxes` | mostly other government units and the EU | proposed `transfer.other_current` |

> **Simplification:** `other_taxes` (1.0 billion lei, 0.2% of spending) is posted as an other current transfer; `non_produced_assets` (−0.14 billion) as negative investment.

**Proposed flow codes** (for `crates/econ-ledger/src/codes.rs`; not added here, append-only when they are):

| Code | Name | For |
|---|---|---|
| 1003 | `gov.investment` | government fixed investment and land (P5, NP) |
| 2002 | `interest.gov_debt` | interest on government debt (D4) |
| 3003 | `tax.employer_contribution` | employers' contributions actually paid (CAM), by every employer |
| 4001 | `benefit.pension` | old-age and survivors' benefits |
| 4002 | `benefit.disability` | sickness and disability benefits |
| 4003 | `benefit.family` | child allowance and child-raising leave |
| 4004 | `benefit.unemployment` | unemployment benefit |
| 4005 | `benefit.housing` | housing benefit |
| 4006 | `benefit.social_assistance` | minimum income and other social-exclusion benefits |
| 4007 | `benefit.student` | grants to pupils and students |
| 4008 | `benefit.other` | other social protection |
| 4009 | `benefit.in_kind` | social transfers in kind bought from market producers (D632) |
| 4101 | `transfer.eu_budget` | own resources paid to the EU (D76) |
| 4102 | `transfer.other_current` | other current transfers and other taxes paid |
| 4103 | `transfer.capital` | capital transfers (D9) |
| 5001 | `subsidy.production` | subsidies on production (D3) |

Range 4xxx (transfers) and 5xxx (subsidies) are new ranges; [social transfers](social-transfers.md) and [state enterprises](state-enterprises.md) may refine the benefit and subsidy codes before they are added.

### How public employment links to the firm units and the population
- A public post is a job in one of the five public firm units (ADR-0016: non-market producers, state share 100%). `public_posts[f]` is the unit's headcount target at the start; the labour market's public hiring (M4) moves the unit's employees toward it.
- The employees of a public unit are the persons whose `employer` is that unit ([Stage J](../society/population-employers.md)); `employer_type` follows from the unit's ownership. Stage J fills the units by their persons employed in the firm-unit file, which are this table's posts (since 2026-10-15), and ranks them by this table's wages per post.
- What the government pays in wages each month is what the public units' employees earn (their `wage`), not the budget's `wages` cell: [Stage J](../society/population-employers.md) takes the population's side as the truth for wage bills. The budget's `wages` cell is the appropriation; the difference at the start is reported, not posted.
- Measured on the fixtures (seed 42, 2026-10-15): the starting population has 1,049,083 public employees at 1:1000 (1,047,489 at 1:100) against 1,262,318 posts (−17%), paid 109.1 billion lei a year against 120.0 billion (−9%; 109.3 billion at 1:100). The census counts fewer people working in administration, education and health than the Ministry counts posts. Each public unit has about 83% of its posts; by unit the population's wage bill differs from the budget's (health +11%, social care +7%, administration −15%, education −18%, defence −22%) because Stage F sets wages by industry group, not by unit.

> **Simplification:** the public firm units take their persons employed and wage bills from this table (posts and `wages` by unit, 2026-10-15), but their output and value added still follow the input-output table by industry, and COFOG functions are not NACE industries. So social care's wages (6.39 billion lei) exceed its output (1.92 billion: the input-output table gives the whole industry 0.41 billion of compensation), administration's wages (56.28 billion) exceed its value added (54.58 billion), and by industry the firm units' wages no longer add up to the input-output table: education −1.8 billion, health −12.5, administration and defence +8.2, social care +6.1; nationally −0.08 billion (−0.03%). Confidence: medium that this matters little before production and the government's purchases use the public units' costs (open questions).

## Player levers
The budget sets the starting values; the levers belong to [fiscal policy](fiscal-policy.md), [social transfers](social-transfers.md) and the others. Names are those of the [lever catalogue](../game/levers.md); dotted ids are proposals for the lever registry (ADR-0018), which does not exist yet.

| Cell | Lever (catalogue name) | Proposed registry id | Starting value |
|---|---|---|---|
| `wages`, `employer_contributions`, `goods_services`, `investment`, `non_produced_assets` of a line | Budget per ministry, keyed by line | `spend.ministry_budget[line]` (money, bani a year) | the sum of those five cells of the line |
| posts of a public unit | Public sector headcount targets | `spend.public_headcount[unit]` (number) | `public_posts[f]` |
| pay of public employees | Public sector wages (% vs private equivalent) | `spend.public_wage_ratio` (rate) | the ratio in the starting population, measured by the scenario builder |
| `social_benefits_*` by programme | State pension, Child benefit, Unemployment benefit, Minimum income, Housing benefit, Student finance | their own ids ([social transfers](social-transfers.md)) | the law in force ([research note](../../01-research/notes/law-in-force-romania-2021.md)); the programme totals here are what those rules must pay |
| `subsidies` | Subsidies (M9) | — | held at the starting value until M9 |
| `interest` | none | — | follows the debt |
| `current_transfers`, `capital_transfers`, `other_taxes` | none | — | held; the EU contribution follows the EU's rules |

**Proposed here (new):**
- Keys `economic_affairs`, `housing_community`, `social_protection` (running costs of the benefit agencies) and `defence` (held: military budget is not in v1) for Budget per ministry; the catalogue's key "infrastructure maintenance" is the line `transport` (transport and communications, including their investment).
- **Disability benefit** (`transfer.disability_benefit`): amount and eligibility of sickness and disability benefits (10.6 billion lei in 2021), which no lever of the catalogue covers.
- `spend.public_investment[line]`: the investment part of a line until the projects levers (M11) take it over; or the investment stays inside Budget per ministry (open question).

## Tuning parameters
None. The guesses are data, in the fixture's `provenance.guesses`:

| Guess | Value | Confidence |
|---|---|---|
| CAM rate on public wages | 2.25% | high (the law) |
| Imputed contributions by function | defence 40%, public order 60% | low |
| Health posts outside the Ministry of Health | 60,000 central, 165,000 local | low |
| Social care posts (local) | 130,000 | low |
| Other public order and justice posts | 40,000 | medium |

## Interactions
- **Before:** the pipeline (Eurostat, Ministry of Finance); the firm units; the starting population (Stage J).
- **After:** [fiscal policy](fiscal-policy.md) (monthly spending, the deficit to finance), the [labour market](labor-market.md) (public headcount targets, M4), [social transfers](social-transfers.md) (programme totals as calibration targets), [state capacity](state-capacity.md) (spending per ministry), the [EU](eu-membership.md) (deficit against the reference value), the scenario builder (`law.json`, `budget.json`, the consistency report).

## Edge cases & failure modes
- **A published total that the subgroups do not reach** (rounding to 0.1 million lei): the difference goes to the largest cell of that transaction and is listed in `provenance.rounding` (none in the 2021 data).
- **Negative amounts** (`non_produced_assets`: the state sold more land than it bought): allowed for that type only; monthly amounts keep the sign.
- **A line with no spending of a type:** zero; never missing.
- **A public unit with posts and no synthetic employee** (none at 1:1000 since the units were built from the posts; possible at very small scales): its wage appropriation is unspent and reported; nothing is created.
- **A public unit with employees and no posts:** rejected by the loader.
- **Malformed file:** wrong number of amounts in a line, an unknown or repeated type or line, a sum that differs from its published total, a sector whose unit is not a public firm unit, posts that do not add up: rejected with a named error; nothing is loaded.
- **Overflow:** the largest amount (old-age pensions, 1.1 × 10^13 bani) is far inside `i64`; sums use checked arithmetic.

## Acceptance tests
IDs are stable. Tests are to be written in a test-authoring session (ADR-0014), in `crates/econ-io/tests/acceptance/budget.rs` (protected), against the loader sketched below. "The fixture" is `python/pipeline/fixtures/budget2021_ro.json`. Measured values are from the Python reference on 2026-10-13; those of AC-BUD-05, 06 and 09 on 2026-10-15, after the public units were rebuilt from this table. AC-BUD-01 to 13 were written there on 2026-10-20 against the API skeleton (`econ_io::budget`), compiled only with the crate feature `budget` until the implementation turns it on; the reconciliations AC-BUD-05 to 09 read the committed fixtures and the scenario that `econ_scenario::build` makes, since the builder's report has no budget section yet, and AC-BUD-12 runs the reference with `--tables`. AC-BUD-14 waits for the proposed flow codes and a function that posts a month of the budget, which the sketch does not have.

- [ ] **AC-BUD-01** `[unit]` For each of the twelve types, the amounts of the thirteen lines add up exactly (in bani) to the published national total the fixture carries (D11, D12, P2, D62, D632, D3, D4, P5, NP, D7, D9, D29+D5+D8), and the lines add up to total expenditure, 474,031,500,000 lei, exactly.
- [ ] **AC-BUD-02** `[unit]` The lines are a partition of the COFOG subgroups (no subgroup in two lines); for each of the ten divisions, the amounts of its subgroups add up exactly to Eurostat's published division total.
- [ ] **AC-BUD-03** `[unit]` The social benefits by programme add up exactly to D62 (cash) and D632 (in kind).
- [ ] **AC-BUD-04** `[unit]` Posts by sector add up to 1,262,318; every sector's unit is one of the five public firm units; the public units' posts, wages and employer contributions add up exactly to the total posts, D11 and D12.
- [ ] **AC-BUD-05** `[unit]` The public units' wage bill in the firm-unit file is within 2% of the budget's wages (D11). Measured: 0.00% (the units' wages are this table's).
- [ ] **AC-BUD-06** `[unit]` The public units' persons employed in the firm-unit file are within 2% of the posts, in total and for each unit. Measured: 0.00% (the units are built from the posts; tightened from 25% on 2026-10-15, when it measured +18.9%).
- [ ] **AC-BUD-07** `[unit]` The earnings file's wage bill of the industry group `public_education_health`, times twelve, is within 10% of the budget's wages (D11). Measured: −4.2%.
- [ ] **AC-BUD-08** `[unit]` Old-age and survivors' cash benefits are within 10% of the pension file's pension bill times twelve (measured +7.3%); government final consumption in the input-output table is within 1% of P3 (measured 0.00%).
- [ ] **AC-BUD-09** `[unit]` In the starting population at 1:1000 and 1:100 (seed 42), the weighted employees of the five public units are within 20% of the posts (measured −16.9% and −17.0%), and their wages times twelve within 10% of D11 (measured −9.1% and −8.9%).
- [ ] **AC-BUD-10** `[unit]` Every annual amount becomes twelve monthly amounts in bani that add up exactly to it, differ by at most one ban, put the remainder on the first months and keep the sign of a negative amount.
- [ ] **AC-BUD-11** `[golden]` The budget hash of the fixture equals the committed golden value. Hash: FNV-1a 64 over the line count (u32, little-endian); per line its id (one length byte, then UTF-8) and each amount in bani (i64, little-endian) in type order; the sector count (u32); per sector its id and posts (u32); per public unit in file order its id, posts (u32), wages and employer contributions in bani (i64). Python reference: `ffd4772468f8552e`.
- [ ] **AC-BUD-12** `[diff]` On the fixture, the Rust loader and the Python reference (`python/reference/budget_reference.py`) give the same lines, programmes, sectors, public units, first-month amounts in bani and hash.
- [ ] **AC-BUD-13** `[unit]` A line with the wrong number of amounts, an unknown or repeated type or line, a type whose lines do not add up to its published total, a sector whose unit is not a public firm unit, posts that do not add up, and a public unit with wages and no posts are each rejected with a named error, and nothing is loaded.
- [ ] **AC-BUD-14** `[unit]` Posting the first month of every cell under the flow codes of "Ledger mapping" moves exactly the monthly amounts from the government to the payees, and the ledger's invariants hold. (Waits for the proposed flow codes.)

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-BUD-14: the proposed flow codes in `codes.rs` and a function that posts a month of the budget (not in the API sketch)

## API sketch (Rust, no implementation)
In `econ-io` (loading) behind an off-by-default feature `budget`; the types in `econ-types` if fiscal policy needs them without the loader.

```rust
/// The twelve economic types, in file order.
#[repr(u8)]
pub enum EconType { Wages, EmployerContributions, GoodsServices, SocialBenefitsCash, SocialBenefitsInKind,
                    Subsidies, Interest, Investment, NonProducedAssets, CurrentTransfers, CapitalTransfers, OtherTaxes }
pub const N_TYPES: usize = 12;

pub struct BudgetLine {
    pub id: String,                       // "education", ...
    pub cofog: Vec<String>,               // its COFOG subgroups
    pub per_year: [Bani; N_TYPES],        // whole lei a year, as bani
}

pub struct Programme { pub id: String, pub cofog: Vec<String>, pub cash: Bani, pub in_kind: Bani }

pub struct PublicSector { pub id: String, pub posts: u32, pub unit: String }

pub struct PublicUnitBudget { pub unit: String, pub posts: u32, pub wages: Bani, pub employer_contributions: Bani }

/// Published national totals by ESA code, as bani (TE, D1, D11, D12, P2, D62, D632, D3, D4, P5, NP, D7, D9, ...).
pub struct PublishedTotals(pub Vec<(String, Bani)>);   // sorted by code; no HashMap (ADR-0006)

pub struct Budget {
    pub year: i32,
    pub start_date: Date,
    pub gdp: Bani,
    pub lines: Vec<BudgetLine>,
    pub programmes: Vec<Programme>,
    pub sectors: Vec<PublicSector>,
    pub public_units: Vec<PublicUnitBudget>,
    pub published: PublishedTotals,
}

pub enum BudgetError {
    Shape { line: String },                         // wrong number of amounts
    UnknownType(String), DuplicateLine(String),
    TotalMismatch { code: String, lines: Bani, published: Bani },
    DivisionMismatch { cofog: String },
    UnknownUnit(String), PostsMismatch, WagesWithoutPosts(String),
}

impl Budget {
    pub fn from_json(bytes: &[u8]) -> Result<Budget, BudgetError>;   // parses and runs `check`
    pub fn check(&self) -> Result<(), BudgetError>;                    // AC-BUD-01..04, 13
    pub fn monthly(per_year: Bani, month: u8) -> Bani;                 // AC-BUD-10; month 0..=11 from the start month
    pub fn total(&self, t: EconType) -> Bani;
    pub fn hash(&self) -> u64;                                          // AC-BUD-11
}
```

The reconciliations (AC-BUD-05 to 09) read other parts of the scenario and belong to the scenario builder's report, with the tolerances above.

## Open questions
- [x] **Done 2026-10-15, with the stage J re-golden (owner approved 2026-10-14; `CHANGELOG-sim.md`); the spec is ready to lock.** The public units' persons employed and wages are now the posts and `wages` of `public_units`; output and value added still follow the input-output table (see the simplification under "How public employment links"). **Rebuild the public units from this table?** Proposed: `normalise_firms.py` takes each public unit's persons from `public_posts` and its wage bill from the budget's `wages`, instead of 12.5 persons per million lei and the guessed public shares. It fixes the per-unit gaps (health, social care, and the arbitrary pay order of public units in Stage J), but it changes the firm-unit fixture and so the employer hashes recorded in the locked [Stage J](../society/population-employers.md) spec and their golden tests. Owner's decision.
- [x] **Accepted as vacant posts at the start (owner, 2026-10-14).** **The remaining gap of 16% in public employees** (census against posts): accept it as vacant posts at the start (the labour market fills them toward the target from M4), or move census workers of the group "public, education, health" from private to public units. Proposed: accept, and report.
- [x] **Accepted: `budget.json`, ADR-0018 Amendment 1 (owner, 2026-10-14).** **Scenario part.** ADR-0018 lists the scenario's parts; the table needs a place. Proposed: a new part `budget.json` (the fixture's shape), with the lever values in `law.json` as usual. Needs an amendment to ADR-0018.
- [x] **Yes, included (owner, 2026-10-14).** **Does "Budget per ministry" include the line's investment,** or does investment get its own lever until the projects of M11? Proposed: included, so one number per ministry, as the catalogue has it.
- [x] **Held, no lever in v1 (owner, 2026-10-14).** **Defence:** held at its starting value with no lever in v1 (the catalogue's "Not in v1: military budget"), or a key of Budget per ministry? Proposed: held.
- [x] **Public units whose wages exceed their output** (social care by 4.5 billion lei, administration's wages above its value added by 1.7 billion), since the rebuild. Before production or fiscal policy uses a public unit's costs: map the wages of the social-protection functions (GF1001, GF1004, GF1006, GF1007) to the industry that pays them in the national accounts (mostly public administration), or raise the public units' output to their cost (non-market output is the sum of costs) and take the difference out of the input-output table's other industries. Owner's decision. **Decided (owner, 2026-10-20):** the `World` pays a public producer's shortfall at cost (spec game/world-loop, open questions), so its costs are covered whatever the table's output; the mapping of social-protection wages stays for when ministries' budgets drive purchases (M4).
- [ ] **Check the guesses** of health and social care posts against the Ministry of Finance's full monthly staffing table (by ministry and financing), and the D11/D12 split against Eurostat's ESA table 11 detail if a breakdown of D12 by function exists.
