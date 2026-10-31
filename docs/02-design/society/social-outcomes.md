---
id: society/social-outcomes
title: Social Outcomes — Crime, Health, Wellbeing, Poverty
status: draft
owner: horia
depends_on: [society/population-groups, society/education, economy/fiscal-policy, society/poverty-inequality, economy/demographics]
research: [research/return-immigration-drivers]
updated: 2026-10-31
---

# Social Outcomes — Crime, Health, Wellbeing, Poverty

## Purpose
Translate economic and policy conditions into outcomes people care about and that feed back into the economy: crime (cost, investment climate), health (labour supply, mortality, productivity), poverty and wellbeing.

## Real-world basis
- Crime rises with unemployment (esp. youth), inequality and low education; falls with policing and education (Becker economics of crime; education-crime literature).
- Health improves with income, education and health spending, with diminishing returns; life expectancy correlates with GDP per capita (Preston curve).
- Poverty measured relative (below 60% of median equivalised income) and absolute.

## Crime
**State:** `crime_rate` per region (offences per 1,000 / year), `policing_capacity` per region.
```math
\text{crime}_{r} = c_0 \cdot f(u^{youth}_r,\ \text{Gini}_r,\ \text{edu}_r,\ \text{poverty}_r) \cdot g(\text{policing}_r)
```
with $g$ decreasing and diminishing. Effects: property-crime losses (wealth transfer + destruction recorded in accounts), lower investment confidence in the region, health penalties, emigration push.

## Health
**State:** `health` per person; `health_system_capacity` per region (hospitals, staff).
- Health target per person = f(age, sex, income class, education, health spending per capita, capacity, air quality from [environment](../economy/environment.md), heatwaves, unemployment).
- Effects: mortality (demographics), labour-force participation (sick → inactive), productivity, healthcare consumption demand.

## Wellbeing and poverty
- `poverty_rate` (relative and anchored), `gini`, `s80_s20`, `hardship_rate` and the standard-of-living classes are computed from incomes by [poverty and inequality](poverty-inequality.md) (roadmap M5); `real_income_per_capita` for any group.
- **Standard of living (SoL)**, the material number per household, class and group (Victoria-3-style), is [poverty and inequality](poverty-inequality.md)'s step 9: actual consumption per adult, public health and education included.

### Wellbeing index (first version, owner 2026-10-31)
A human-development-style index (UNDP's HDI: health, education, living standard) for the country and each region, kept separate from SoL so that each lever's effect stays visible in its own number: a budget moves SoL at once, literacy and life expectancy move over years, and health spending counts once (in SoL), not twice. It is shown to the player (with its history) and read by [migration](../economy/migration-remittances.md)'s expected home quality (step 5b); no other behaviour reads it.

Each monthly close, for the nation and each development region $r$:
```math
W_r = \big(H \cdot E_r \cdot L_r\big)^{1/3}, \qquad
H = \frac{e_0 - 20}{85 - 20}, \qquad
E_r = \tfrac12\Big(\frac{\text{MYS}_r}{15} + \frac{\text{EYS}_r}{18}\Big), \qquad
L_r = \frac{\overline{\text{SoL}}_r - 10}{60 - 10}
```
each dimension index clamped to $[0, 1]$ (UNDP's goalposts for health and education; the living standard's are the game's: SoL 10 is subsistence, 60 is about 32 times it).
- $e_0$ (`life_expectancy`): period life expectancy at birth from [demographics](../economy/demographics.md)' mortality table of the year (a standard period life table, deaths at mid-year, 100 the last age), both sexes weighted by the residents' sex shares. National until health has regional mortality.
- $\text{MYS}_r$ (`mean_years_schooling`): the weighted mean years of schooling of residents aged 25 and over, from `edu_level` (`schooling_years`: ISCED 0–2 8, 3 12, 4 13, 5–6 15.5, 7–8 17.5 years).
- $\text{EYS}_r$ (`expected_years_schooling`): the sum over ages 5 to 29 of the share of the region's residents of that age who are enrolled (`enrolled`, [education](education.md)).
- $\overline{\text{SoL}}_r$: the region's persons' mean SoL.
- **Later dimensions** (each when its mechanic exists, the index becoming the geometric mean of all dimensions with equal weights): safety (crime), environment (air quality), health outcomes beyond mortality, governance (corruption, with the institutions milestone).

> **Simplification:** the index is the game's, not UNDP's HDI: its living standard is consumption (SoL) rather than income per head, and life expectancy is national. Its level is not compared with Romania's HDI (0.821 in 2021); its parts are (life expectancy, mean and expected years of schooling).

### Implementation status
- **2026-10-31:** the wellbeing index on the `World` (feature `wellbeing`, off by default), computed at each close after the poverty indicators, outside the state hash: pure functions in `econ_core::wellbeing`, the close in `world/wellbeing_state.rs`. Life expectancy uses the People phase's table of the year when it acts, else the scenario's 2017–2019 table, ages 0 to 100. Regions are the residents' development regions; SoL by region is step 9's mean over the private households' persons. AC-WB-01 to 04 pass. At 1:1000, month 0: life expectancy 75.4 years (2019: 75.6), mean years of schooling 12.0 (UNDP 2021: 11.3), expected years 14.5 (14.2), index 0.69 (Bucharest-Ilfov 0.72 highest, Nord-Est 0.68 lowest); expected years dip after each June close until the August intake.

## Player levers
Police and justice budget, health budget, hospitals (infrastructure), education (see education), transfers (poverty), power plant choice (pollution).

## Acceptance tests
**Wellbeing index (first version, M5; tests in `crates/econ-core/tests/acceptance/wellbeing.rs` behind a feature `wellbeing`):**
- [ ] **AC-WB-01** `[unit]` The index is the geometric mean of the three dimension indices, each clamped to [0, 1], in millionths rounded to the nearest; with any dimension at 0 the index is 0; with all at 1 it is 1; the goalposts give 0 and 1 at life expectancy 20 and 85, mean years 0 and 15, expected years 0 and 18, SoL 10 and 60.
- [ ] **AC-WB-02** `[unit]` Life expectancy from a mortality table: a table in which nobody dies before 100 and everyone dies at 100 gives 100.5 years (deaths at mid-year); a constant yearly probability q below 100 gives the life table's closed form; from the 2019 table of the Romania scenario (pre-pandemic), 74.5 to 76.5 years (2019: 75.6).
- [ ] **AC-WB-03** `[sim]` At 1:1000, at the close of month 0: mean years of schooling of residents 25 and over between 10.0 and 12.5 (UNDP 2021: 11.3), expected years of schooling between 12.5 and 16.0 (14.2), and every region's index between 0 and 1 with the national index between the regions' lowest and highest.
- [ ] **AC-WB-04** `[sim]` Isolation: at 1:1000, the state hash after 24 months is the same with the index computed and not computed (it is read by migration only through step 5b, behind its own feature). *(Test 2026-10-31: "not computed" is `WorldParams::wellbeing` `None`; step 5b's feature is `home_quality`, and with it compiled in step 5b's elasticities are 0 in both runs.)*

- [ ] A recession raising youth unemployment by 10 points raises crime in the affected regions within 12 ticks.
- [ ] Doubling police spending lowers crime with diminishing returns.
- [ ] Higher health spending raises life expectancy slowly (years), more where the baseline is low.
- [ ] Raising minimum income lowers the absolute poverty rate within 3 ticks.

## API sketch (wellbeing index)
```rust
// crate econ-core, module `wellbeing` (pure functions), feature `wellbeing` (needs `poverty` for SoL)
pub fn life_expectancy_milli(yearly_q_ppb: &[[u32; 101]; 2], men_share_ppm: u32) -> u32;  // AC-WB-02, thousandths of a year
pub fn wellbeing_ppm(life_exp_milli: u32, mean_years_milli: u32, expected_years_milli: u32, sol_milli: u32) -> Wellbeing; // AC-WB-01
pub struct Wellbeing { pub index_ppm: u32, pub health_ppm: u32, pub education_ppm: u32, pub living_ppm: u32 }
// in the World
pub struct WellbeingStats { pub month: Month, pub national: Wellbeing, pub by_region: Vec<Wellbeing>,
    pub life_expectancy_milli: u32, pub mean_years_milli: Vec<u32>, pub expected_years_milli: Vec<u32> } // by region, national last
impl World { pub fn wellbeing_stats(&self) -> &[WellbeingStats]; }
pub struct WellbeingParams { pub schooling_years_milli: [u32; 5] }   // WorldParams::wellbeing: Option<WellbeingParams>, None: not computed (AC-WB-04)
```

## Open questions
- [x] Pollution and environment → [environment](../economy/environment.md).
- [ ] Should crime have a black-market economy component (untaxed income)?
