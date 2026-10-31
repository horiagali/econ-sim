---
id: economy/migration-remittances
title: Migration, Diaspora & Remittances
status: draft
owner: horia
depends_on: [society/social-outcomes, economy/demographics, economy/households, economy/labor-market, economy/trade-fx, economy/eu-membership, economy/social-transfers, economy/transport-network, society/population-groups, adr/0003-people-representation, adr/0006-determinism-contract, adr/0007-money-and-ledger, adr/0008-explainability-architecture, adr/0017-time-base]
research: [research/migration-remittances-romania-2021, research/return-immigration-drivers]
updated: 2026-10-31
---

# Migration, Diaspora & Remittances

## Purpose
People move between counties, leave Romania, come back, and send money home while they are away. For Romania this is first-order: about 4 million people born here live abroad, about 217,000 left in 2021, and their remittances (17.25 billion lei net in 2021) are about 1.5% of GDP and a steady credit of the current account. This spec is roadmap M5's "moves between counties, emigration and return, the diaspora and remittances". It sits on the demographics core ([demographics](demographics.md): ageing, births, deaths, retirement, household formation), which it does not repeat.

The game needs it because:
- a long run without emigration overstates the labour force and the pension contributors, and understates ageing;
- households' income in the `World` is about 70 billion lei a year short of the input-output table's consumption (roadmap M7); remittances are a quarter of that gap;
- the player's choices (taxes on wages, the minimum wage, wages in the public sector) should change how many people leave, and a world setting (`world.wages_abroad`) should too.

## Real-world basis
Evidence and sources: [research note](../../01-research/notes/migration-remittances-romania-2021.md).

- **Flows, 2021** (Eurostat, long-term): 216,861 emigrants; 194,642 immigrants, of whom 149,513 Romanian citizens coming back, 9,878 other EU citizens and 35,165 non-EU citizens. Net about −22,000. Emigrants are young: 29% aged 25–34, 21% 35–44, 18% 15–24, 9% children. Confidence high.
- **Diaspora:** about 4 million people born in Romania (Italy, Germany, Spain, the UK, Hungary). The exodus came mostly after EU accession in 2007; since 2009 departures are 160,000 to 250,000 a year, and returns rose strongly from 2022. Confidence medium.
- **Remittances, 2021** (BNR's balance of payments, BPM6): personal transfers received 19,056 million lei, paid 1,808 million: **17,248 million net**, secondary income. Compensation of employees received (residents working abroad for less than a year) was a further €3.8 billion, primary income. Confidence high.
- **Internal migration, 2021** (INS): 374,526 changes of domicile between localities; net county gains concentrated in Ilfov (+13,602), Cluj, Timiș, Bucharest, Brașov, Iași, Sibiu; the poorer counties of Moldavia and the south lose. Confidence high; the share that crosses a county border is an educated guess (35–45%, low).
- **Behaviour:** emigration follows the expected-income gap (Harris–Todaro), unemployment at home, age and networks (family already abroad); EU free movement removes legal barriers ([EU membership](eu-membership.md)). Remittances rise with the sender's income and fall with years abroad. Returns rise when home wages catch up. Internal moves go toward jobs and wages and fall with distance. Confidence medium for the directions, low for the sizes.

## Representation
How migrants fit the weighted synthetic population ([ADR-0003](../../03-architecture/decisions/0003-people-representation.md)): every record has an integer weight, persons share their household's `hh_weight`, and no rule may depend on the population scale.

**Moves are whole records.** A move takes a household record with all its weight, or one person of a record for all of its weight: at 1:100 a chosen record is a hundred real households in which the same thing happens. The number of real persons that should move in a cell (a group of similar persons, below) is the sum of their weighted probabilities; records are taken in keyed priority order until the cell's target is met, and the difference (positive or negative, less than one record's weight) is carried to the cell's next month. Over months the moves equal the expected numbers exactly, at every scale, and no record is ever split for migration.

> **Simplification:** moving whole records makes migration lumpy at coarse scales (at 1:1000 a record is a thousand households). The carry keeps totals right; the diversity of movers is the diversity of the sample. A split of records, if the demographics core introduces one, could later make moves finer.

**Three kinds of migrant:**
1. **A linked emigrant** is a person who left a household that stays in Romania. Its person row stays in the household record with `abroad` set: it is counted in the diaspora, not in the resident population, and it is skipped by work, consumption, taxes and benefits. It ages and can die like any person (the demographics core's mortality). It sends remittances to its household and can come back to it. Because persons share their household's weight, a linked emigrant stands for exactly one emigrant per real household of the record.
2. **The diaspora stock** holds every other emigrant as counts: whole households that left, linked emigrants whose home household dissolved, and the starting diaspora that has no link. It is a table of integer counts of real persons by origin region, sex, year of birth and skill tier (`diaspora_stock[k]`). It sends no remittances.
3. **Foreign immigrants and returns from the stock** enter as new records cloned from a resident donor (below).

A record that loses all its members to emigration (a whole household) gets weight 0 and becomes a free slot; slots are skipped by every loop and reused by new records. Person rows of a freed record are freed with it.

**Resident population** = the weighted persons without `abroad` in records of positive weight. **Diaspora** = the weighted persons with `abroad` plus the stock.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| Remittances paid to linked households | monthly, on the 15th, with imputed rent |
| Emigration, returns, foreign immigration, moves between counties | monthly close, after the demographics core's month-end step, in that order |
| `months_abroad` of linked emigrants | monthly close |
| Ageing and deaths of the diaspora stock | yearly, in January, with the demographics core's ageing |
| Statistics (flows, diaspora, remittances, net migration by county) | monthly close |

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| — | `abroad` | person is a linked emigrant | bool |
| — | `months_abroad` | months since a linked emigrant left | months |
| — | `diaspora_stock[k]` | unlinked emigrants by origin region, sex, year of birth, skill tier | persons |
| — | `migration_carry[cell]` | real persons owed to a cell's next month (emigration, return, moves) | persons |
| $\bar r$ | `remittance_base` | euro a month one linked emigrant sends at the start (before decay) | EUR cents / month |
| $\alpha_c$ | `county_appeal[c]` | fixed attractiveness of county $c$ for internal moves, fitted to 2021 | utility |
| $p^0_a$, $q^0_a$ | `emigration_base[a]`, `return_base[a]` | monthly base rates by age band $a$, fitted at load | ppm / month |
| $\omega$ | `wages_abroad` | world setting (lever); 1 at the start | ratio |

## Inputs
- Persons and households: age, sex, `edu_level`, occupation, `employer`, `wage`, `months_unemployed`, `hh_county`, `hh_region`, `hh_weight`, deposits and loans (population, [labour market](labor-market.md), [money and banking](money-banking.md)).
- Net wage by skill tier (gross wage less the employee's contributions and income tax, [taxation](taxation.md)), the regional unemployment rate (labour market).
- The exchange rate $e$ ([trade and FX](trade-fx.md)).
- Births, deaths, ageing, household dissolution ([demographics](demographics.md)).
- Travel times between county seats ([transport network](transport-network.md), from M11; before it, distances between county seats from a scenario fixture).

## Outputs
Emigrants, returns, foreign immigrants and net migration by month, age, skill and county; the diaspora (linked and stock); remittances (`hh_remittances_in` per household, the national flow); moves between counties and each county's net balance. Through them: the labour force, pension contributors, households' income, the current account and the leu.

## Update rule

### 1. The pull of abroad
For each skill tier $s$ (low, mid, high, as in the [labour market](labor-market.md); a person without a job takes the tier its education qualifies for), the monthly **pull** $R_s$ (`emigration_pull[s]`) compares wages abroad with what the tier earns at home, both in euro and relative to the start:
```math
R_{s,t} = \frac{\omega_t}{H_{s,t}}, \qquad H_{s,t} = \frac{\bar W^{net}_{s,t} / e_t}{\bar W^{net}_{s,0} / e_0}
```
$\bar W^{net}_{s,t}$ is the mean net wage of the tier's employees at the last monthly close. At the start $R = 1$. A tax cut on wages, a minimum-wage rise or faster wage growth lowers the pull; a weaker leu or higher wages abroad raise it.

### 2. Emigration (monthly close)
Each resident person aged 18 to 64, not a pensioner, has a monthly probability (younger persons leave only with their household)
```math
p_i = \min\Big(p^{max},\ p^0_{a(i)} \cdot R_{s(i)}^{\varepsilon} \cdot \frac{1 + \beta_u u_{r(i)}}{1 + \beta_u u_{r(i),0}} \cdot (1 + \beta_{net} \cdot \text{linked}_h) \cdot m^{unemp}_i \cdot B\Big)
```
- $p^0_a$ is fitted at load so that, with every factor at its starting value, the expected emigrants of the first year equal the 2021 figures by age band (Eurostat, the research note), less the children who leave with whole households.
- $\text{linked}_h$ is 1 if the household already has a linked emigrant (network effect).
- $m^{unemp}_i$ is `unemployed_factor` for a person looking for work, else 1.
- $B$ is 1 while Romania is in the EU ([EU membership](eu-membership.md)); leaving the EU (M18) sets it to `exit_barrier`.

**Cells and alignment:** cells are (region, age band, skill tier). Each cell's target is $\sum_i w_i p_i$ plus its carry; records are chosen by keyed priority (ADR-0006: a draw keyed by the day, the purpose and the person's index) scaled by $p_i$, until the target is met; the carry takes the remainder.

**Who goes:** a chosen person leaves alone (becomes a linked emigrant) or with the whole household, by a keyed draw with probability `household_share`. The whole household goes in any case when the person is its only adult and minors remain, or when it is a one-person household. Households with a pensioner never leave whole; their working-age members can leave alone.

**What leaves with them:**
- An employed emigrant separates at the month close like a quit ([labour market](labor-market.md), step 3): the unit's staff falls and it posts vacancies on its next staffing day. Contributions and income tax stop.
- A linked emigrant takes nothing: the household keeps its money.
- A whole household's cash is deposited, its loans are repaid from its deposits (`finance.loan`), any loan left is written off by its bank (`finance.write_off`), and the rest of its deposits become deposits of the rest of the world at the same bank (flow code `transfer.migrant_capital`, capital account): emigrants keep Romanian accounts. Its persons join `diaspora_stock`; the record becomes a free slot. Its Pillar II claim stays with the funds ([accounting](accounting.md)) and is not paid out.

> **Simplification:** BPM6 treats a change of residence as an "other change in volume", not a transaction. Here it is a capital transfer, so that the rest of the world's deposits change only through flows and the balance of payments closes without an amendment to [trade and FX](trade-fx.md) (AC-TFX-03).

### 3. Remittances (monthly, on the 15th)
Each linked emigrant $i$ in household $h$ sends, in euro:
```math
r_{i,t} = \bar r \cdot \omega_t^{\eta} \cdot (1 - \delta)^{\text{months\_abroad}_i / 12}
```
paid in lei at the day's rate, $r_{i,t} \cdot e_t$, from the rest of the world's lei deposits to the household's deposits, multiplied by $w_h$ (ADR-0007), flow code `transfer.remittances` (secondary income). It enters `hh_income_received` and `hh_remittances_in` like any income ([households](households.md)), and is not taxed.

$\bar r$ is fitted at load: the sum over linked emigrants at the start, at $e_0$, is the 2021 net flow, 17,248 million lei a year (1,437 million a month). Because remittances are in euro, a weaker leu raises them in lei: a cushion for linked households and a support for the leu.

> **Paid on the 15th** (restated 2026-10-30, as proposed under the owner's delegation, as imputed rent was on 2026-10-21): on the last day it would change pay day's income, which AC-ST-05 holds to the pay slips; the exchange rate moves only at the monthly close, so the amount in lei is the same, and it is still in the next decision's income.

> **First version (2026-10-30):** step 3 runs before the rest of the spec, on by default with the outside world (feature `trade`): the links are the starting ones (stage L), months abroad advance with the months, and ω stays at its starting 100% until `world.wages_abroad` is wired with emigration (the lever stays `planned`). The `[sim]` criteria stay behind the feature `migration` until the other steps exist; AC-MIG-06 passes with it on.

> **Simplification:** the 2021 *net* flow is paid as an inflow; the 1,808 million lei that residents (mostly immigrants) send abroad are not modelled separately. The diaspora stock sends nothing; all remittances run through links.

### 4. Returns (monthly close)
- A **linked emigrant** returns with a monthly probability $q_i = q^0_{a} \cdot \text{link\_return} \cdot R_{s}^{-\varepsilon_r}$; it rejoins its household as a resident (`abroad` cleared), looking for work if aged 15 to 64.
- The **diaspora stock** returns with $q^0_a \cdot R_{s}^{-\varepsilon_r}$ per real person. Returning persons from a stock cell form new one-person household records, each of weight `sample_scale` (the scenario's, never a constant; the cell's carry takes the rest), placed in a county of the origin region by its share of the region's population, with the other columns cloned from a resident donor of the same county, sex, age band and education (the donor with the lowest keyed draw).
- $q^0_a$ is fitted at load to the 2021 returns of Romanian citizens by age (149,513 a year), shared between links and the stock by their sizes and `link_return`.
- Each returning adult brings `return_savings` (euro, at the day's rate) from the rest of the world into the household's deposits under `transfer.migrant_capital`.

> **Simplification:** a returnee comes back with the skills it left with; experience gained abroad, and pension rights earned abroad, are not modelled.

### 5. Foreign immigration (monthly close)
Foreign immigrants a month:
```math
I_t = \frac{I^{EU}_{2021} + I^{nonEU}_{2021}}{12} \cdot H_t^{\varepsilon_i}
```
with $H_t$ the all-tier home wage index in euro of step 1 (Romania richer, more immigrants). They enter as one-person household records of weight `sample_scale`, aged as 2021's immigrants, in counties by their share of the labour market's vacancies at the last close, cloned from a resident donor, looking for work, with no money.

> **Simplification:** no quota until M14. The yearly quota of non-EU work permits (25,000 in 2021, 100,000 in 2022) becomes a lever there, capping the non-EU part.

### 5b. Expected home quality (owner, 2026-10-31)
*People who know Romania weigh the life they expect there, not only wages (owner, 2026-10-31). Research: [return and immigration drivers](../../01-research/notes/return-immigration-drivers.md): Romanians abroad name corruption, health care and bureaucracy as what keeps them away; foreign immigrants follow wages and jobs.*

Each region $r$ has an **expected home quality** $Q_{r,t}$ (`home_quality[r]`), 1 at the start, a slow memory of what people hear about home:
```math
Q_{r,t} = Q_{r,t-1} + \frac{1}{m}\,\big(q_{r,t} - Q_{r,t-1}\big), \qquad
q_{r,t} = \frac{W_{r,t}}{W_{r,0}} \cdot \Big(\frac{1 + \beta_u u_{r,0}}{1 + \beta_u u_{r,t}}\Big)^{\theta_u}
```
- $W_{r,t}$ is the region's **wellbeing index** ([social outcomes](../society/social-outcomes.md): life expectancy, schooling, and the standard of living SoL of [poverty and inequality](../society/poverty-inequality.md), which counts public health and education); $W_{r,0}$ the first close's. Governance, safety and environment reach migration when they join the index.
- $u_{r,t}$ is the region's unemployment rate (step 2's, shocks included): jobs at home bring people back, not only keep them.
- $m$ is `quality_memory` (months): expectations move slowly, over years.

It multiplies:

| Flow | Wages (steps 1, 5) | Jobs | Expected home quality |
|---|---|---|---|
| Returns (step 4), links and stock, of the region they return to | yes | in $Q$ | $Q_r^{\varepsilon_q}$ |
| Emigration (step 2), the other side of the same choice | yes | yes (step 2) | $Q_r^{-\varepsilon_{q,e}}$, still capped at $p^{max}$ |
| Foreign immigration (step 5) | yes | (vacancies, later) | $\bar Q^{\varepsilon_{q,i}}$, weak; $\bar Q$ the residents' mean |

SoL reaches migration only through the wellbeing index, so wages are not counted twice. At the start every factor is 1, so the 2021 fit of $p^0$ and $q^0$ is unchanged.

**Tests (2026-10-31):** AC-MIG-18 to 20 are behind a crate feature `home_quality` (`migration` and the wellbeing index's `wellbeing`; AC-WB-04 puts step 5b behind its own feature). They read the test writer's additions: the pure rules `instant_quality_ppm`, `home_quality_ppm` and `quality_factor_ppm` in `econ-mech-migration`, the five parameters above at the end of `MigrationWorldParams` (`quality_return_elasticity_ppm`, `quality_emigration_elasticity_ppm`, `quality_immigration_elasticity_ppm`, `quality_jobs_weight_ppm`, `quality_memory_months`), `World::home_quality_ppm` (per development region, after the latest close) and `MigrationMonth::returns_by_region`.

### 6. Moves between counties (monthly close)
Only moves that cross a county border are modelled (a move inside a county changes nothing in the `World`). Each resident household in county $c$ moves with a monthly probability
```math
m_h = m^0_{a(h)} \cdot \frac{1 + \beta^{move}_u u_{r(c)}}{1 + \beta^{move}_u u_{r(c),0}}
```
by the age band $a(h)$ of its oldest working-age member: unemployment in the household's region pushes it out, as in step 2. $m^0_a$ is fitted at load so that, at the starting unemployment, the moves equal `between_county_share` of 2021's changes of domicile (educated guess 0.4: about 150,000 persons a year). Cells are (county, age band), aligned as in step 2. A moving household picks a destination $d \neq c$ with logit shares
```math
P(d \mid c) = \frac{\exp V_{cd}}{\sum_{d' \neq c} \exp V_{cd'}}, \qquad V_{cd} = \alpha_d + \beta_w \ln\frac{\bar W_d}{\bar W_c} - \beta_u (u_d - u_c) - \beta_t \, t_{cd}
```
taken by keyed draw. $\bar W$ is the county's mean net wage, $u$ its region's unemployment, $t_{cd}$ the travel time in hours. $\alpha_d$ is fitted by the Python tooling so that, at the starting wages and unemployment, each county's expected net balance is 2021's net balance (INS) times the between-county share; it is a scenario column.

The household's `hh_county` and `hh_region` change. Its employed members keep their jobs if the destination is in the same region, else they quit and look for work in the new region. Its money moves with it (no posting).

> **Simplification:** housing costs enter $V_{cd}$ with the housing market (M13), and $t_{cd}$ becomes the network's passenger time with M11. Students and others who move without changing domicile are not counted, as in INS's data.

### 7. Explainability
Emigration, return, the destination choice and the remittance amount are `behaviour_rule!`s ([ADR-0008](../../03-architecture/decisions/0008-explainability-architecture.md)): the "why" of a month's emigrants names the pull (wages at home in euro, wages abroad), unemployment, the network and the base rate; the "why" of remittances names the number of linked emigrants, their decay, wages abroad and the exchange rate.

### Implementation status
- **2026-10-31, slice (a) of four** (owner: linked emigrants and the diaspora view; then emigration, returns and immigration, moves): with the feature `migration` (off by default; it now includes `demography`, since linked emigrants age and die with the People phase), the linked emigrants of `tables/linked_emigrants.arrow` become person rows of their households at load, with `abroad` and `months_abroad`: inert (no job, no search, no pension, no income) and skipped by every loop of residents (the People phase's households, births, retirement and life cycle; poverty, approval, education, the child allowance, revenue's keys). They age and die by the People phase's mortality. A record split copies them with the record, a union takes them along, and when their household ends they join the stock (`diaspora_stock`, tiers low 0 to high 2). Remittances read the rows. `StartEdit::NoDiaspora` drops both tables. AC-MIG-06 and 07 pass. AC-MIG-14's first month fails on the wording of 2026-10-29 (the test is to be restated as the owner did on 2026-10-30). *(Restated by the test writer on 2026-10-31: the test now reads the first month as worded on 2026-10-30, the month's imports being the rest of the world's receipts under `imports`.)*
- **2026-10-31, slice (b):** emigration at each monthly close after education (steps 1 and 2). The pull by tier compares each tier's mean net wage of the pay slips in euro with the first close's. Regional unemployment is the residents' (with `MigrationUnemploymentShock` added), its start that of the first close. `p0` by band (15–24 to 55–64) is fitted at load by solving the five bands' equations, a year's chosen persons plus the companions who leave with their households, against 2021's emigrants (`MigrationWorldParams::emigrants_2021`). Cells are region × band × tier, records taken by keyed priority over `p_i` (stream `Migration`). A chosen person leaves alone (a linked emigrant: inert, job ended, a new head if it was the head) or with the whole household: its money settles like an estate, but the net deposits go to the rest of the world under the new code 4111 `transfer.migrant_capital` (a capital-account line inside the trade month's total, as the EU's capital grants), and its persons join the stock. `world.wages_abroad` is active when the feature is on (the registry stays `planned` without it) and moves the pull and remittances (ω^η). The tables are compacted at every close with migration, so freed rows leave at once. AC-MIG-08, 09, 12 and 16 pass. **Owner questions (calibration):** the first year has about 248,000 emigrants against 216,861 ± 10%: the fit holds at the start's factors, but regional unemployment rises in the first months (in the baseline too), and children are 15% of emigrants against the spec's 9% (`household_share` and the only-adult rule); AC-MIG-13's contributions are 0.7% higher at 130% wages abroad (wages rise with the tighter labour market), to be rechecked with returns; AC-MIG-06's remittances are 3.2% over the band until returns end some links (slice c).
- **2026-10-31, slice (c):** returns, foreign immigration and the stock's deaths (steps 4 and 5). `migration` now includes `lifecycle` (new records are made with its record operations). `q0` is fitted at load to 2021's returns of citizens by group: nobody under 15 is abroad at the start (stage L's links and stock are adults), so the returns of children are folded into the 15–64 group (144,082 a year) and 65 and over has 5,431. Links return aligned by group × tier and rejoin their household (seeking work if 15 to 64); the stock returns aligned by origin region × group × tier (adults), each return a one-person record of weight `sample_scale` in a county of its region by residents, cloned from the donor of the lowest keyed draw in that county with its sex, age band and tier (then fewer criteria). Returning adults bring `return_savings` (3,000 euro) under `transfer.migrant_capital`. Foreign immigrants are 45,043/12 × H^ε_i a month, aged as 2021's emigrants of 18 to 64 (no age table of foreign immigrants), 54.7% men, in counties by residents (the World's vacancies have no county), with no money. The stock loses its deaths each January by the year's mortality table. Records formed by the life cycle the same day do not emigrate that month. AC-MIG-06 to 12, 14 and 16 pass; in the first year returns are about 144,000 and immigrants 44,000, and the age shares are within 3 points. **Owner questions:** AC-MIG-05's emigrants (about 250,000 against 216,861 ± 10%, slice b's calibration question) and AC-MIG-13's pension contributions (0.3% higher at 130% wages abroad).
- **2026-10-31, slice (d):** moves between counties (step 6). Data: the pipeline's `county_moves2021_ro.json` (new `normalise_county_moves.py`, from INS TEMPO POP303B and POP301B through the fetcher's new `ins_tempo` source: each county's 2021 net balance of changes of domicile, which sums to zero, the 374,526 changes of 2021, and the county seats' positions), written to the scenario as `county_moves.json` and read into `PeopleStart::county_moves`. Readings the code takes where the spec leaves room, or amends it: `m0` is one rate for every household (no age profile in the first version), so cells are counties, not (county, age band); `α` is fitted at load by iteration on the starting population, as `p0` and `q0` are, not by the Python tooling (a county's balance in INS's data is already its moves across borders, as moves inside it net to zero, so the target is the balance itself, not the balance times `between_county_share`); `W` is the county's mean gross wage of those with a job (the same measure at load and in the run); $t_{cd}$ is the straight-line distance between county seats × 1.3 at 70 km/h until M11; destinations are allocated as whole records with a carry per (origin, destination) pair, each record to the destination most owed, instead of a keyed draw per record (at 1:100 a draw left a county's year within the noise of a few records). Employees of a household that changes region quit and look for work; the self-employed keep their work. AC-MIG-15: Ilfov, Cluj and Timiș gain and Teleorman and Vaslui lose in the first year at 1:100; Bacău does not lose (+204 persons): the fit gives its 2021 balance at the start, but the model's regional unemployment converges in the first months, so its region's relative unemployment improves (calibration, M7). AC-MIG-13 and 14 sit on knife edges: any change of the run flips them. AC-MIG-14 counts migrant capital in the trade month's total (AC-MIG-06's reading), so the gap without the diaspora can exceed the month's remittances when returnees bring savings (owner question).

## Effects on the rest of the economy
- **Labour supply:** emigrants leave the labour force and their jobs; returns and immigrants join as job seekers. Tight tiers raise wages ([labour market](labor-market.md), step 5), which lowers the pull: a self-correcting loop.
- **Pensions:** fewer contributors pay pension contributions; pension spending is unchanged (pensioners do not emigrate). The ratio worsens over decades, which M5's exit asks for ([social transfers](social-transfers.md)).
- **Households:** remittances raise linked households' income and spending; linked households are more often rural and older, so the money goes mostly to food and basic goods (by their baskets).
- **Current account and the leu:** remittances are a current-account credit; the capital transfers of migrants are in the capital account. Both enter the exchange-rate step through the rest of the world's flows ([trade and FX](trade-fx.md), step 4): more remittances, a stronger leu.
- **Housing (M13):** emigration and moves leave dwellings vacant in losing counties ([housing](housing.md)).

## Player levers
- **`world.wages_abroad`** (world setting, registered `planned` in the [lever system](../game/lever-system.md), wired here): wages abroad relative to the start, 25% to 400%, read at the monthly close. It moves the pull (more emigration, fewer returns) and the amount each linked emigrant sends. Lag: the next monthly close for flows; the stock and the labour market respond over years.
- Indirect: every lever on net wages (income tax, contributions, minimum wage, public pay) moves the pull of step 1; unemployment moves emigration and moves between counties.
- Not in the first version (open questions): a return incentive, the non-EU immigration quota (M14), a tax on remittances.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| $\varepsilon$ `emigration_elasticity` | 1.5 | 0.5–3 | how strongly emigration follows the pull (literature about 1–2, low confidence) |
| $\varepsilon_r$ `return_elasticity` | 1.0 | 0–3 | how strongly returns follow home wages |
| $\varepsilon_i$ `immigration_elasticity` | 1.0 | 0–3 | foreign immigration and home wages |
| $\beta_u$ `emigration_unemployment` | 5 | 0–20 | +1 point of regional unemployment raises emigration by about 5% |
| $\beta_{net}$ `network_effect` | 1.0 | 0–3 | a household with a member abroad is twice as likely to send another |
| `unemployed_factor` | 2.0 | 1–5 | a job seeker's emigration multiple |
| $p^{max}$ `max_emigration_rate` | 2% a month | — | cap on a person's monthly probability |
| `household_share` | 0.3 | 0–1 | share of chosen emigrants who leave with their whole household (fitted so children are about 9% of emigrants) |
| `link_return` | 1.5 | 1–3 | a linked emigrant's return rate against the stock's |
| $\eta$ `remittance_elasticity` | 0.8 | 0–1.5 | remittances per emigrant and wages abroad |
| $\delta$ `remittance_decay` | 5% a year | 0–15% | ties weaken with time abroad |
| `return_savings` | 3,000 euro per adult | 0–20,000 | savings a returnee brings (educated guess) |
| `between_county_share` | 0.4 | 0.3–0.6 | share of changes of domicile that cross a county border (educated guess) |
| $\beta_w$, $\beta_u$, $\beta_t$ (moves) | 1.0, 5, 0.1 per hour | — | destination choice: wages, unemployment, travel time |
| $\beta^{move}_u$ `move_unemployment` | 5 | 0–20 | +1 point of the origin region's unemployment raises moves out by about 5% (step 6) |
| `exit_barrier` | 0.4 | 0–1 | emigration multiple outside the EU (M18) |
| $\varepsilon_q$ `quality_return_elasticity` | 1.0 | 0–3 | returns and expected home quality (step 5b; a guess, no estimate exists) |
| $\varepsilon_{q,e}$ `quality_emigration_elasticity` | 0.5 | 0–3 | emigration and expected home quality (step 5b) |
| $\varepsilon_{q,i}$ `quality_immigration_elasticity` | 0.25 | 0–1 | foreign immigration and expected home quality (step 5b; weak in the evidence) |
| $\theta_u$ `quality_jobs_weight` | 1.0 | 0–2 | jobs in the instant quality $q$ (step 5b) |
| $m$ `quality_memory` | 36 months | 6–120 | how slowly expectations of home follow it (step 5b) |

## Starting state
- **Linked emigrants** come from a new population stage (owed, in the Python pipeline, [population generator](../society/population-generator.md)): it adds one `abroad` person to chosen households, with age, sex and education drawn like 2021's emigrant stock, and `months_abroad` from a distribution centred on 2007–2014 departures. Targets (educated guesses, low confidence): about 1.2 million real households with a linked emigrant (15% of households), more in rural Moldavia, Maramureș and the south; fewer in Bucharest. It is the only change to the starting population.
- **The diaspora stock:** about 4 million less the linked emigrants, about 2.8 million, by origin region, sex, year of birth and tier, from the research note's destination and age data; a scenario table.
- $\bar r$, $p^0$, $q^0$, $m^0$ fitted at load from the scenario's population and 2021 targets (in the scenario's `migration.json`), so the starting month gives 2021's rates at any scale.
- A scenario without these tables runs with no diaspora: no remittances, emigration and moves still on.

**Built 2026-10-29 (stage L of the population generator, `econ-popgen`; data only, read by no mechanic yet).** The margin file is the pipeline's `linked_emigrants2021_ro.json` (`normalise_linked_emigrants.py`: 2021's emigrants by sex and single year of age from Eurostat `migr_emi2` as the age at departure; every other number is the guess above). The scenario builder runs it after stage J and writes two tables:
- `tables/linked_emigrants.arrow`: one row per linked emigrant (household, `person_key` after the residents', age, sex, role, education, `months_abroad`). **The linked emigrants are a table of their own, not rows of `persons.arrow` with `abroad` set**, so that no stage and no phase sees them until migration reads them; the migration PR may fold them into the persons table.
- `tables/diaspora_stock.arrow`: `diaspora_stock[k]` by origin region, sex, year of birth and tier (high, mid, low), the diaspora total less the linked emigrants (2.8 million).

> **Simplification (choices of the stage, where the spec leaves room):** a private household's chance is the product of its region's factor (Moldavia 1.5, the south and Maramureș's region 1.1–1.2, Bucharest-Ilfov 0.4), the rural factor (1.5) and its head's age factor (under 30: 0.3, 30–44: 0.8, 45–69: 1.6, 70 and over: 1.0); households are taken by keyed priority until 1.2 million real households, exact to half a record. The emigrant's sex and age at departure follow 2021's emigrants of 18 to 69; the departure year a guessed profile (2002–2006 low, 2007–2014 high, 2015–2021 steady); education 30% ISCED 0–2, 43% 3, 7% 4, 16% 5–6, 4% 7–8. Its role is child when the head is at least 15 years older, else other. The stock is split by the same profiles, by region with its residents times its factor; deaths abroad since departure are not taken out. At 1:1000, seed 42: 1,203 records standing for 1,200,306 households.

## Edge cases & failure modes
- **A cell's target exceeds its candidates:** all candidates move, the rest of the target is dropped (not carried), so a small county cannot be emptied by a debt of carries.
- **The last adult of a household with minors emigrates:** the whole household goes (no unaccompanied minors).
- **A linked household dissolves or dies out** (demographics core): its `abroad` persons move to the stock with their counts; remittances to it stop.
- **A linked emigrant dies abroad:** removed by the demographics core like any person; no estate moves.
- **A household that emigrates owes more than it holds:** the bank writes off the difference (`finance.write_off`); a loss for the bank, small in practice (emigrating households are mostly young renters).
- **Extreme pull** (wages abroad at 400%, home wages collapsing): $p^{max}$ caps each person; the population shrinks fast but stays non-negative.
- **Extreme exchange rate:** remittances in lei grow with $e$; amounts are computed in `i128` and checked back to `Bani`.
- **Free slots:** a run with heavy emigration leaves many weight-0 records; they are reused before the tables grow, and saves keep them (compaction is an open question).
- **No candidates in a destination county** (all moved out): the destination logit still works; a county with no residents has no wage, so $\bar W_d$ falls back to its region's.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-core/tests/acceptance/` (protected) and, for the pure functions, in the mechanic's crate.

- [ ] **AC-MIG-01** `[unit]` The pull is $\omega / H$ of step 1: 1 at the start; a 10% rise of wages abroad raises it by 10%; a 10% rise of home net wages in lei at a constant exchange rate lowers it by 1/1.1; a 10% depreciation raises it by 10%. The emigration probability is the formula of step 2, increasing in the pull, the unemployment rate and the network flag, never above $p^{max}$.
- [ ] **AC-MIG-02** `[unit]` Whole-record alignment: given weighted candidates with priorities and a target plus carry, records are taken in priority order, each with its whole weight, until the target is met; the new carry is the target less the weight taken; over 120 months of constant expectations the total moved differs from the expected total by less than one record's weight; reruns give the same records.
- [ ] **AC-MIG-03** `[unit]` The remittance amount is the formula of step 3: in euro it rises with wages abroad by the elasticity and falls by the decay per year abroad; in lei it is the euro amount times the exchange rate, to the ban.
- [ ] **AC-MIG-04** `[unit]` Destination shares of step 6 sum to one million ppm; a destination with higher wages, lower unemployment or a shorter travel time gets a larger share; ties are broken by county order.
- [ ] **AC-MIG-05** `[sim]` First year on the Romania scenario under the 2021 law: weighted emigrants are 216,861 ± 10%, returns of Romanian citizens 149,513 ± 10%, foreign immigrants 45,043 ± 10%; each age band's share of emigrants from 15 to 64 is within 3 points of the research note's table.
- [ ] **AC-MIG-06** `[sim]` First 12 months: remittances received by households total 17,248 million lei ± 3% (the exchange rate's drift aside: the test divides each month by $e_t / e_0$); every lei of them is posted from the rest of the world to households under `transfer.remittances`, and the month report counts them in the current account.
- [ ] **AC-MIG-07** `[sim]` Remittances close part of the households' income gap: over the first year, households' total income with migration on exceeds a counterfactual with the diaspora tables removed by at least 16 billion lei, and their consumption is higher.
- [ ] **AC-MIG-08** `[sim]` Conservation of persons, every month, at 1:1000 and 1:100: resident persons change by exactly births − resident deaths − emigrants + returns + foreign immigrants, and the diaspora (linked plus stock) by exactly emigrants − returns − deaths abroad; moves between counties sum to zero nationally; no weight or count is negative.
- [ ] **AC-MIG-09** `[sim]` Conservation of money with migration on, every day, at 1:1000 and 1:100: every ledger invariant holds (I-1 to I-8 of ADR-0007); a whole household's emigration moves exactly its net deposits to the rest of the world under `transfer.migrant_capital`; the weighted sum of household deposits equals the banks' deposit liabilities to households.
- [ ] **AC-MIG-10** `[sim]` Raising `world.wages_abroad` by 20% at the start: over the next 24 months emigrants are at least 10% above a baseline run, returns are lower, the diaspora is larger, and remittances in euro are higher.
- [ ] **AC-MIG-11** `[sim]` Lowering `world.wages_abroad` by 20%: over the next 24 months returns are higher and emigrants lower than in the baseline.
- [ ] **AC-MIG-12** `[sim]` A cut of the employee's pension contribution by 5 points (higher net wages): over the next 24 months, emigrants of working age are fewer than in the baseline.
- [ ] **AC-MIG-13** `[sim]` Emigration takes jobs and contributors: after 5 years with `world.wages_abroad` at 130%, the labour force, the employed and the pension contributions (`contrib.pension`) are lower than in the baseline, and pension spending is not.
- [ ] **AC-MIG-14** `[sim]` Remittances support the leu: in the counterfactual without the diaspora tables, the exchange rate after 24 months is higher (the leu weaker) than with them, and the current account lower: in the first month by at least the month's remittances less that month's extra imports (imports with the diaspora tables less without them), minus 5% of the remittances, and in every later month by more than zero and at most the month's remittances (spending on imports and the weaker leu narrow the gap; owner, 2026-10-29). *(First month restated by the owner, 2026-10-30: remittances arrive on the 15th, so part is spent on imports within the month; the test of 2026-10-29 assumes last-day arrival and is to be restated with this wording; restated in the test 2026-10-31.)*
- [ ] **AC-MIG-15** `[sim]` Moves between counties: in the first year Ilfov, Cluj and Timiș gain residents and Teleorman, Vaslui and Bacău lose them; a county whose region's unemployment rises 5 points (a scripted shock) gains fewer movers and sends more over the next 24 months.
- [ ] **AC-MIG-16** `[sim]` Determinism and scale: two runs with the same seed give identical persons, weights and ledger; at 1:1000 and 1:100 the first year's emigration rate per resident differs by less than 10% of its value.
- [ ] **AC-MIG-17** `[sim]` 50-year quiet run: the diaspora stays between 2 and 7 million, the resident population shrinks with net emigration (the direction of the official projection, M5's exit), and no table outgrows its starting size by more than half (free slots reused).
- [ ] **AC-MIG-18** `[unit]` *(step 5b, 2026-10-31)* Expected home quality is 1 at the start and stays 1 while the wellbeing index and unemployment hold their starting values; after a step change of $q$ it moves a share $1/m$ of the gap a month, and every multiplier of step 5b is 1 when $Q = 1$.
- [ ] **AC-MIG-19** `[sim]` *(step 5b)* Raising the health and education ministry budgets by 30% for five years: returns are higher and emigrants lower than in the baseline, and the gap in returns is larger in the fifth year than in the first (expectations build slowly).
- [ ] **AC-MIG-20** `[sim]` *(step 5b)* A region whose unemployment rises 5 points (`MigrationUnemploymentShock`) receives fewer returns over the next 24 months than in the baseline.

## API sketch
For the test writer and the implementer; names may change, shapes should not.

```rust
// crate econ-mech-migration (no I/O)
pub struct MigrationParams { pub emigration_elasticity_ppm: u32, pub return_elasticity_ppm: u32,
    pub unemployment_response_ppm: u32, pub network_effect_ppm: u32, pub unemployed_factor_ppm: u32,
    pub max_emigration_ppm: u32, pub household_share_ppm: u32, pub link_return_ppm: u32,
    pub remittance_elasticity_ppm: u32, pub remittance_decay_ppm_year: u32 }
pub fn pull_ppm(wages_abroad_bp: u32, home_net_wage_eur_index_ppm: u32) -> u32;                  // AC-MIG-01
pub fn emigration_ppm(base_ppm: u32, pull_ppm: u32, unemp_ppm: u32, unemp0_ppm: u32,
                      linked: bool, job_seeker: bool, p: &MigrationParams) -> u32;               // AC-MIG-01
pub struct Pick { pub record: u32, pub weight: u32, pub priority: u64 }
pub fn take_whole_records(target: i64, carry: &mut i64, candidates: &mut [Pick]) -> Vec<u32>;   // AC-MIG-02
pub fn remittance_eur_cents(base_cents: i64, wages_abroad_bp: u32, months_abroad: u16,
                            p: &MigrationParams) -> i64;                                         // AC-MIG-03
pub fn eur_to_lei(eur_cents: i64, exchange_rate_ppm: u64) -> Bani;                               // AC-MIG-03
pub fn destination_ppm(origin: u8, utility_ppm: &[i64]) -> Vec<u32>;                             // AC-MIG-04
pub fn instant_quality_ppm(wellbeing_ppm: u32, wellbeing0_ppm: u32, unemp_ppm: u32, unemp0_ppm: u32,
                           unemployment_response_ppm: u32, jobs_weight_ppm: u32) -> u32;          // AC-MIG-18, step 5b's q
pub fn home_quality_ppm(prev_ppm: u32, instant_ppm: u32, memory_months: u16) -> u32;            // AC-MIG-18, Q + (q − Q) / m
pub fn quality_factor_ppm(quality_ppm: u32, elasticity_ppm: i64) -> u32;                         // AC-MIG-18, Q^ε

// econ-core
impl World {
    pub fn migration_months(&self) -> &[MigrationMonth];  // emigrants, returns, immigrants, by age band and county; remittances
    pub fn diaspora(&self) -> DiasporaView;               // linked count, stock by cell
    pub fn home_quality_ppm(&self) -> &[u32];             // step 5b's Q by development region (feature home_quality)
}
pub enum StartEdit { /* … */ NoDiaspora }                 // the counterfactual of AC-MIG-07 and 14
```

Powers and exponentials use the pinned `libm` through `econ_num::math` (ADR-0006). New flow codes (append-only, ADR-0007): 4110 `transfer.remittances` (secondary income, rest of the world to households), 4111 `transfer.migrant_capital` (capital account, both directions). (Renumbered 2026-10-30, as proposed under the owner's delegation: 4109 is demographics' `transfer.estate`, which its test fixes.)

## Open questions
- [x] **Expected home quality (step 5b).** **Decided by the owner, 2026-10-31:** yes. Returns and emigration read the expected wellbeing index (which holds SoL) and jobs; foreign immigration reads wages and jobs, with at most a weak quality factor; SoL itself reaches migration only through the index. Built after SoL and the index, with the defaults above as guesses calibrated in M7 against 2022–2024's rising returns.
- [x] **Diaspora destinations:** one "abroad" or named countries (Italy, Spain, Germany, UK)? **Decided by the owner, 2026-10-26:** one abroad in v1, with `world.wages_abroad` as its wage; named destinations after v1, "to consider" in the roadmap.
- [x] **Compensation of employees** (€3.8 billion in 2021, residents working abroad under a year, primary income): model seasonal work abroad? Decided as proposed (owner's delegation, 2026-10-26): not in M5; list it with remittances as the second candidate in M7's household-income gap (added to the roadmap), as a person status "working abroad this season" if M7 needs it.
- [x] **Starting links:** how many households receive remittances, and where? Decided as proposed (owner's delegation, 2026-10-26): 1.2 million (15%), skewed to rural Moldavia and older heads, until a source (the household budget survey's income from abroad) is read; the total flow is exact whatever the count. The pipeline stage is owed in roadmap M5.
- [x] **Whole-household emigrants' money:** a capital transfer (as here) or an "other change in volume" (BPM6) with an amendment of AC-TFX-03? Decided as proposed (owner's delegation, 2026-10-26): the capital transfer; the difference is presentation only.
- [x] **Return incentives** (Romania ran "Diaspora Start-Up" grants): a lever? Decided as proposed (owner's delegation, 2026-10-26): yes, after M5, as a grant per returnee that multiplies $q$; it needs a cost line in the budget.
- [x] **Pensions abroad:** Romania pays pensions to former contributors living abroad, and returnees bring foreign pensions. Decided as proposed (owner's delegation, 2026-10-26): not in v1 (pensioners do not emigrate; returnees bring savings only).
- [x] **Remittances and need:** should a linked household in hardship or unemployment receive more (the insurance motive)? Decided as proposed (owner's delegation, 2026-10-26): not in the first version; add a factor if M7's distribution checks show linked households too poor.
- [x] **Alignment to the official projection:** force net migration to INS's or Eurostat's projection, or let it follow behaviour? **Decided by the owner, 2026-10-26: no.** Behaviour only (a forced total would kill the lever's effect); the projection is a check for M5's exit.
- [x] **Compaction of free slots** at save time? Decided as proposed (owner's delegation, 2026-10-26): no compaction in v1; reuse slots, and check table growth in AC-MIG-17.
- [x] **`levers.toml`** names `economy/demographics` as `world.wages_abroad`'s spec. Decided as proposed (owner's delegation, 2026-10-26): the wiring PR points `world.wages_abroad` to this spec.
