---
id: glossary
title: Glossary & Variable Registry
status: draft
owner: horia
depends_on: []
updated: 2026-10-31
---

# Glossary & Variable Registry

**Single source of truth for names.** Specs and code use these symbols and `code_name`s. Add new variables here in the same change that introduces them.

Conventions: subscript `t` = tick; stocks are end-of-tick; flows are per tick; money amounts are in nominal local currency unless marked `real`.

| Symbol | code_name | Meaning | Unit | Type | Owner spec |
|---|---|---|---|---|---|
| $Y$ | `gdp_real` | Real output | real LCU / tick | flow | economy/production |
| $P$ | `price_level` | Consumer price index | index (base = 1) | state | economy/prices-inflation |
| $\pi$ | `inflation` | Price inflation, annualised | % / year | derived | economy/prices-inflation |
| $N$ | `employment` | Employed persons | persons | state | economy/labor-market |
| $L$ | `labor_force` | Labour force | persons | state | economy/demographics |
| $u$ | `unemployment_rate` | $1 - N/L$ | ratio | derived | economy/labor-market |
| $W$ | `wage_nominal` | Average nominal wage | LCU / worker / tick | state | economy/labor-market |
| $C$ | `consumption` | Household consumption | LCU / tick | flow | economy/households |
| $I$ | `investment` | Gross fixed investment | LCU / tick | flow | economy/investment-capital |
| $K$ | `capital_stock` | Capital stock | real LCU | state | economy/investment-capital |
| $G$ | `gov_spending` | Government purchases | LCU / tick | flow | economy/fiscal-policy |
| $T$ | `tax_revenue` | Total tax revenue | LCU / tick | flow | economy/fiscal-policy |
| $B$ | `gov_debt` | Government bonds outstanding | LCU | state | economy/fiscal-policy |
| $i$ | `policy_rate` | Central bank policy rate | % / year | lever | economy/monetary-policy |
| $e$ | `exchange_rate` | LCU per unit foreign currency | ratio | state | economy/trade-fx |
| $X$ | `exports` | Exports | LCU / tick | flow | economy/trade-fx |
| $M$ | `imports` | Imports | LCU / tick | flow | economy/trade-fx |
| $\text{CAR}$ | `capital_adequacy_ratio` | Bank equity / risk-weighted assets | ratio | derived | economy/money-banking |
| $r^L, r^D, r^M$ | `loan_rate`, `deposit_rate`, `mortgage_rate` | Bank rates | % / year | state | economy/money-banking |
| $r^B$ | `bond_yield` | Government bond yield | % / year | state | economy/fiscal-policy |
| $\text{cred}$ | `cb_credibility` | Central bank credibility | 0–1 | state | economy/monetary-policy |
| $\pi^{target}$ | `inflation_target` | Inflation target | % / year | lever | economy/monetary-policy |
| $\pi^e$ | `inflation_expectation` | Expected inflation (national; households may differ) | % / year | state | economy/prices-inflation |
| $p_g$ | `price[g]` | Price of good *g* | LCU / unit | state | economy/prices-inflation |
| $\mu_g$ | `markup[g]` | Price markup over unit cost | ratio | state | economy/prices-inflation |
| $uc_j$ | `unit_cost[j]` | Unit cost of industry *j* | LCU / unit | derived | economy/production |
| $Y_j$ | `output[j]` | Real output of industry *j* (also per firm unit, `output[f]`) | units / tick | flow | economy/production |
| $\bar Y_j$ | `capacity[j]` | Output capacity | units / tick | state | economy/production |
| $cu_j$ | `capacity_utilisation[j]` | $Y_j/\bar Y_j$ | ratio | derived | economy/production |
| $V_j$ | `inventory[j]` | Finished-goods inventory | units | state | economy/production |
| $A_j$ | `tfp[j]` | Total factor productivity | index | state | economy/production |
| $a_{ij}$ | `io_coefficient[i,j]` | Units of input *i* per unit of output *j* | ratio | parameter | economy/industries |
| $W_{j,o}$ | `wage[j,o]` | Wage by industry and occupation (persons vary around it) | LCU / worker / tick | state | economy/labor-market |
| $W^{min}$ | `minimum_wage` | Minimum wage | LCU / month | lever | economy/labor-market |
| $V_{j,s,r}$ | `vacancies` | Open positions | jobs | state | economy/labor-market |
| $\omega_f$, $\omega_j$ | `state_share[f]`, `state_share[j]` | State ownership share of firm unit *f* (the `state` component of `ownership[f]`); of industry *j*, derived as the capital-weighted average over its units (ADR-0016) | 0–1 | lever/state | economy/state-enterprises |
| — | `ownership[f]` | Ownership vector of firm unit *f*: equity shares `state`, `domestic_private`, `foreign`, summing to 1 (free float folded in by holder) | shares | state | adr/0016-firm-representation |
| — | `controller[f]` | Controlling holder of firm unit *f*: the holder with ≥ 50%, else the largest bloc; state / domestic_private / foreign. Sets which behaviour rules apply | category | derived | adr/0016-firm-representation |
| $m_f$ | `equity_multiple[f]` | Multiple on book equity used to value stakes in transactions; rises with ROE, falls with rates | ratio | derived | adr/0016-firm-representation |
| — | `firm_exit_rate[f]`, `firm_entry_rate[f]` | Bankruptcy and birth rate of cohort unit *f* (per real firm); counts change by alignment with a carried remainder | rate / tick | derived | adr/0016-firm-representation |
| — | `administration_ticks` | Ticks a failed named firm operates under administration before being absorbed into its cohort if not rescued | ticks | parameter | adr/0016-firm-representation |
| — | `posting_remainder[f]` | Bani of firm unit *f*'s total balance not divisible by `firm_count[f]`, carried so totals stay exact | Bani | state | adr/0007-money-and-ledger |
| — | `wage_clearing[j]`, `sales_clearing[j]` | Per-industry clearing accounts for wages and sales between weighted records; net to zero every tick (I-8) | LCU | state | adr/0007-money-and-ledger |
| — | `hh_pension_assets` | Household's claim on Pillar II pension funds | LCU | state | economy/accounting |
| $p^E$ | `electricity_price` | Retail electricity price (price of the good `electricity` sold by `power_supply`) | LCU / MWh | state | economy/energy |
| $p^W$ | `wholesale_electricity_price` | Wholesale electricity price, set by merit order across generation technologies | LCU / MWh | state | economy/energy |
| — | `capture_price[j]` | Average price earned per MWh by generation industry *j* (below $p^W$ for solar and wind) | LCU / MWh | derived | economy/energy |
| — | `cfd_contract[f]` | Contract for difference held by generation unit *f*: strike price and volume | LCU / MWh, MWh | state | economy/energy |
| $\text{Cap}_p$ | `plant_capacity[p]` | Installed capacity of plant *p* (owned by one firm unit in its technology's generation industry) | MW | state | economy/energy |
| $K^g_{type,r}$ | `public_capital[type,r]` | Public infrastructure stock | real LCU | state | economy/infrastructure |
| $\Psi_r$ | `infra_multiplier[r]` | Infrastructure effect on capacity | index | derived | economy/infrastructure |
| $w_h$ | `hh_weight` | Real households represented by synthetic household *h* (integer; see ADR-0007) | count | state | society/population-groups |
| — | `hh_size` | Number of persons in synthetic household *h* | count | derived | society/population-generator |
| — | `edu_level` | Person's highest completed education, in five ISCED 2011 groups: 0–2, 3, 4, 5–6, 7–8 | category (5) | state | society/population-groups |
| — | `activity` | Person's main activity (child, pupil, student, employed kinds, unemployed, retired, inactive kinds) | category | state | society/population-groups |
| — | `employment_status` | Person's status in employment: not employed, employee, employer, own-account worker, family worker | category (5) | state | society/population-jobs |
| — | `occupation` | Person's occupation: ISCO-08 major group (0–9) | category (10) | state | society/population-groups |
| — | `industry_group` | Broad group of NACE sections a person works in (ten groups); the catalogue industry lies inside it. Not "sector", which means an institutional sector | category (10) | state | society/population-jobs |
| — | `wage` | Person's gross monthly wage as an employee; 0 for everyone else. Integer bani in code | LCU / month | state | society/population-wages |
| — | `wage_dispersion` | Share of the earnings quantile curve's spread that is left inside a cell when starting wages are set (default 0.5) | ratio | parameter | society/population-wages |
| — | `wage_floor_share` | The wage floor of an industry group is the minimum wage, or this share of the group's mean wage if that is lower (default 0.8) | ratio | parameter | society/population-wages |
| — | `pension` | Person's gross monthly pension; 0 for everyone who is not retired. Integer bani in code | LCU / month | state | society/population-pensions |
| — | `pension_earnings_link` | How much of the earnings gap between education groups starting pensions keep (default 0.5) | ratio | parameter | society/population-pensions |
| — | `pension_dispersion` | Share of the pension quantile curve's spread that is used when starting pensions are set (default 1) | ratio | parameter | society/population-pensions |
| — | `industry` | The catalogue industry a person works in (one of 90); none if not employed | category (90) | state | society/population-employers |
| — | `employer` | The firm unit a person works in (ADR-0016); none if not employed | id | state | society/population-employers |
| — | `employer_wage_link` | How closely an employee's place among the firm units of an industry group follows the wage (default 0.5) | ratio | parameter | society/population-employers |
| — | `inactive_kind` | Kind of inactivity of a person whose `activity` is other inactive: homemaker (looks after the home or family), disabled (cannot work for reasons of health), other (supported by others, discouraged, other situations); none for everyone else | category (4) | state | society/population-inactivity-enrolment |
| — | `enrolled` | Level of formal education a person is enrolled at: none, pre-school, primary, lower secondary, upper secondary, post-secondary non-tertiary, tertiary (ISCED 5–6), master or doctorate (ISCED 7–8) | category (8) | state | society/population-inactivity-enrolment |
| — | `homemaker_child_link` | Chance that a person other inactive who lives with a child younger than 7 is placed first among the homemakers of their sex and age band when starting kinds are set (default 0.5) | ratio | parameter | society/population-inactivity-enrolment |
| — | `study_year` | Year of study within the level a person is enrolled at, from 0 | years | state | society/education |
| $\text{Cap}_{r,\ell}$ | `edu_capacity` | School and university places by region and education level, in weighted persons; in M5 one national pool per level, the enrolment of the start, fixed until infrastructure builds schools (M11) | persons | state | society/education |
| — | `dropout_rate[ℓ]` | Yearly chance that a person enrolled at level ℓ leaves it before completing it | ratio / year | parameter | society/education |
| — | `continue_rate[ℓ→ℓ']` | Chance that a person completing level ℓ goes on to level ℓ' | ratio | parameter | society/education |
| — | `primary_intake` | Share of a year of 6-year-olds who seek a primary school place (default 0.97) | ratio | parameter | society/education |
| — | `preschool_intake[a]` | Chance that a child not enrolled starts pre-school at age *a* (3 to 5) | ratio | parameter | society/education |
| — | `leaver_seek_share[e]` | Share of persons leaving education with `edu_level` *e* who look for work; the rest become inactive | ratio | parameter | society/education |
| — | `min_leaving_age` | Age from which a person who leaves education enters the labour market (default 16) | years | parameter | society/education |
| — | `early_leavers_rate` | Share of persons of 18 to 24 with at most lower secondary education who are not enrolled | ratio | derived | society/education |
| — | `tertiary_attainment` | Share of persons of 25 to 34 with tertiary education (ISCED 5–8) | ratio | derived | society/education |
| — | `school_age` | Age from which a child below the minimum working age is a pupil (default 6) | years | parameter | society/population-attributes |
| — | `hh_locality_size` | Size class of the locality a household lives in (six classes, from under 2,000 to 200,000 or more inhabitants) | category (6) | state | society/population-housing |
| — | `hh_urban` | The household's locality is urban (for now: 10,000 inhabitants or more) | bool | derived | society/population-housing |
| — | `hh_tenure` | How a private household holds its dwelling: owner, tenant, other | category | state | society/population-groups |
| — | `hh_cash`, `hh_deposits` | Currency and bank deposits of a household. Integer bani in code | LCU | state | society/population-wealth |
| — | `hh_dwelling_value` | Market value of the dwelling a household owns; 0 if it does not own its home | LCU | state | society/population-wealth |
| — | `hh_mortgage`, `hh_consumer_debt` | Housing loan and consumer loans a household owes | LCU | state | society/population-wealth |
| — | `hh_collective` | Record stands for people not living in a private household (institutions, homeless); always one member | bool | state | society/population-generator |
| $n_f$ | `firm_count[f]` | Real firms represented by firm unit *f* (integer; 1 for a named firm). Read from the scenario; never hardcoded (ADR-0016) | count | state | economy/production |
| — | `sample_scale` | Real people per synthetic person (default 100). Read from the scenario; never hardcoded (ADR-0003) | ratio | parameter | society/population-groups |
| — | `ideo_econ`, `ideo_social`, `ideo_national` | Person's ideology axes | −1…+1 | state | society/opinion-approval |
| $m_{i,g}$ | `interest[g]` | Person's membership intensity in interest group *g* | 0–1 | state | society/interest-groups |
| $A_i$ | `approval` | Person's approval of head of state | 0–100 | state | society/opinion-approval |
| $A$ | `approval_national` | Weighted average approval | 0–100 | derived | society/opinion-approval |
| $s_{i,k}$ | `approval_component[k]` | The smoothed part *k* of a person's approval (income, unemployment, inflation, hardship, bounds); approval is 50 plus their sum | points | state | society/opinion-approval |
| $r_h$ | `hh_real_income_reference` | The real income a household is used to, against which approval judges its recent income | LCU of day 0 / month | state | society/opinion-approval |
| $\alpha_y$ | `approval_income_weight` | Approval points per unit of relative change in real income (default 60) | points | parameter | society/opinion-approval |
| $\kappa$ | `approval_loss_aversion` | How many times a loss of real income counts against an equal gain (default 2) | ratio | parameter | society/opinion-approval |
| $c^{max}$ | `approval_income_cap` | Largest income part of approval, either way (default 25) | points | parameter | society/opinion-approval |
| $\lambda_r$ | `approval_reference_speed` | Share of the gap to recent income that the reference closes each month (default 1/24) | ratio / month | parameter | society/opinion-approval |
| $u_{self}$, $u_{hh}$ | `approval_unemployed_self`, `approval_unemployed_household` | Approval lost when the person, or another member of the household, is looking for work (defaults 10 and 4) | points | parameter | society/opinion-approval |
| $\alpha_\pi$, $\pi_0$ | `approval_inflation_weight`, `approval_inflation_neutral` | Approval lost per percentage point of twelve-month inflation above the neutral rate (defaults 1 and 3%) | points, % / year | parameter | society/opinion-approval |
| $h$ | `approval_hardship` | Approval lost when the household is in hardship (default 8) | points | parameter | society/opinion-approval |
| $\lambda_A$ | `approval_smoothing` | Share of the gap to its target each part of approval closes each month (default 0.5) | ratio / month | parameter | society/opinion-approval |
| — | `ethnicity`, `mother_tongue`, `religion`, `religiosity` | Person's identity attributes | category / 0–1 | state | society/identity |
| $Y^{disp}_h$ | `hh_disposable_income` | Disposable income of household *h* | LCU / tick | flow | economy/households |
| $C_{h,g}$ | `hh_consumption[g]` | Household consumption of good *g* | LCU / tick | flow | economy/households |
| $P_h$ | `hh_price_index` | Personal price index of household *h* | index | derived | economy/households |
| $Rem_h$ | `hh_remittances_in` | Remittances received from abroad | LCU / tick | flow | economy/migration-remittances |
| — | `abroad` | Person is a linked emigrant: lives abroad, belongs to a resident household, counted in the diaspora and not in the resident population | bool | state | economy/migration-remittances |
| — | `months_abroad` | Months since a linked emigrant left | months | state | economy/migration-remittances |
| — | `diaspora_stock[k]` | Emigrants not linked to a resident household, by cell *k* (origin region, sex, year of birth, skill tier) | persons | state | economy/migration-remittances |
| — | `linked_households`, `diaspora_total` | Starting-state targets of stage L: real private households with a linked emigrant (about 1.2 million) and people born in Romania living abroad (about 4 million: the linked emigrants plus the stock) | households, persons | parameter | economy/migration-remittances |
| — | `migration_carry[cell]` | Real persons owed to a migration cell's next month by whole-record alignment (signed) | persons | state | economy/migration-remittances |
| $\bar r$ | `remittance_base` | Euro a month one linked emigrant sends at the start, before decay; fitted at load to the 2021 flow | EUR / month | parameter | economy/migration-remittances |
| $\alpha_c$ | `county_appeal[c]` | Fixed attractiveness of county *c* in the choice of destination of internal moves, fitted to 2021's net balances | utility | parameter | economy/migration-remittances |
| $p^0_a$, $q^0_a$ | `emigration_base[a]`, `return_base[a]` | Monthly base rates of emigration and return by age band *a*, fitted at load to 2021 | ppm / month | parameter | economy/migration-remittances |
| $R_s$ | `emigration_pull[s]` | Wages abroad over tier *s*'s net wage at home in euro, both relative to the start | ratio | derived | economy/migration-remittances |
| — | `emigrants`, `returnees`, `immigrants` | Persons who left, came back (Romanian citizens) and came in (foreigners) in the month | persons / month | flow | economy/migration-remittances |
| — | `diaspora` | Linked emigrants plus the diaspora stock | persons | derived | economy/migration-remittances |
| $Q^{edu}_{r,\ell}$ | `edu_quality` | Education quality by region and level | 0–1 | state | society/education |
| — | `crime_rate` | Offences per 1,000 people per year, by region | rate | state | society/social-outcomes |
| — | `health` | Person's health index | 0–1 | state | society/social-outcomes |
| — | `eurron`, `usdron` | Exchange rates, RON per EUR / USD | ratio | state | economy/trade-fx |
| — | `currency_stage` | leu_float / erm2 / euro / exited | category | state | economy/euro |
| — | `convergence[criterion]` | Maastricht criteria values and pass/fail | mixed | derived | economy/euro |
| — | `schengen_member` | Whether Romania is in Schengen | bool | state | economy/eu-membership |
| — | `border_wait[route]` | Freight waiting time at internal EU borders | hours | state | economy/eu-membership |
| — | `edp_status` | Excessive deficit procedure stage | category | state | economy/eu-membership |
| — | `eu_compliance[area]` | Compliance score per EU rule area | 0–1 | state | economy/eu-membership |
| — | `eu_absorption_rate` | EU funds reimbursed / available (cumulative) | ratio | derived | economy/eu-funds |
| — | `formality` | Person's job status: formal / partly declared / informal | category | state | economy/informal-economy |
| — | `vat_rate_standard`, `vat_rate_reduced`, `vat_rate_reduced_2` | VAT rates set by the player: the standard rate and two reduced rates; integer basis points in code | ratio | parameter | economy/vat |
| — | `vat_gap` | Share of theoretical VAT not collected | ratio | derived | economy/informal-economy |
| — | `wage_declared` | The part of a person's `wage` that is on the contract; tax and contributions are due on it | LCU / month | state | economy/informal-economy |
| — | `informal_income` | Person's monthly income from undeclared activity outside a job | LCU / month | state | economy/informal-economy |
| — | `self_employment_income`, `self_employment_declared` | Income of an employer or own-account worker, and its declared part | LCU / month | state | economy/informal-economy |
| — | `hh_farm_land` | Agricultural land a household owns or uses | ha | state | society/population-groups |
| — | `hh_subsistence_income` | Value of what a household grows and consumes itself (in kind, untaxed) | LCU / month | state | economy/informal-economy |
| — | `undeclared_sales_share[f]`, `undeclared_wage_share[f]` | Share of firm unit *f*'s sales not reported for VAT and profit tax, and of its wage bill paid outside contracts | ratio | state | economy/informal-economy |
| — | `undeclared_wages[f]`, `undeclared_value_added[f]`, `undeclared_sales[f]` | Firm unit *f*'s wages paid outside contracts, value added not declared (undeclared wages, undeclared household income and undeclared profit), and the sales that carry it; for all its firms together | LCU / year | state | economy/firm-units-undeclared-inventories |
| — | `inventory_materials[f]` | Materials, supplies and goods bought for resale held by firm unit *f* (its own output in stock is `inventory`) | LCU | state | economy/firm-units-undeclared-inventories |
| — | `informal_employment_share` | Share of employed persons whose job is partly declared or informal | ratio | derived | economy/informal-economy |
| — | `undeclared_labour_share` | Share of private-sector labour income that is not declared | ratio | derived | economy/informal-economy |
| — | `revenue_lost` | Tax not collected because of undeclared wages, income and sales, by tax | LCU / month | derived | economy/informal-economy |
| — | `foreign_share[j]` | Foreign-owned share of industry *j* capital, derived from its firm units' ownership shares (ADR-0016) | 0–1 | derived | economy/foreign-ownership |
| — | `house_price[c]`, `rent[c]` | House price and rent per m² by county | LCU / m² | state | economy/housing |
| — | `ets_price` | EU ETS carbon allowance price | EUR / tCO₂ | exogenous | economy/environment |
| — | `emissions_co2e` | Greenhouse-gas emissions | tCO₂e / tick | flow | economy/environment |
| — | `air_quality[c]` | PM2.5 index by county | index | state | economy/environment |
| — | `admin_capacity[m]` | Administrative capacity of ministry/agency *m* | 0–1 | state | economy/state-capacity |
| — | `corruption` | Corruption prevalence | 0–1 | state | economy/state-capacity |
| $L_f$ | `unit_loans[f]` | A firm unit's bank loans (all its firms together) | LCU | state | economy/money-banking |
| $E^b$ | `bank_equity` | The banks' net worth | LCU | state | economy/money-banking |
| $NPL$ | `non_performing_loans` | Share of loans written off in the last twelve months | ratio | derived | economy/money-banking |
| $R$ | `reserves` | Banks' reserves at the central bank | LCU | state | economy/money-banking |
| $A$ | `cb_advances` | Central bank advances to banks | LCU | state | economy/monetary-policy |
| — | `monetary_financing` | The central bank's claim on the government: its deposits below zero and bonds bought from it | LCU | state | economy/monetary-policy |
| — | `debt_maturity[k]` | Government bonds outstanding maturing in year *k* | LCU | state | economy/fiscal-policy |
| — | `debt_coupon` | Average coupon of the government's bonds | % / year | state | economy/fiscal-policy |
| — | `gov_cash_target` | Deposits at the central bank the treasury keeps | LCU | parameter | economy/fiscal-policy |
| — | `import_share[g]` | Imported part of the uses of good *g*, by use | ratio | state | economy/trade-fx |
| $ar X_g$ | `export_base[g]` | Exports of good *g* at starting relative prices | real units / day | parameter | economy/trade-fx |
| — | `hh_stagger_day` | The household's own day of the month (1 to 28) for its monthly decisions; a keyed draw, fixed for life (ADR-0017) | day of month | state | economy/households |
| — | `hh_income_received` | Money a household received since its last decision day | LCU | state | economy/households |
| — | `hh_hardship` | The household could not pay for the subsistence basket this month | bool | state | economy/households |
| $e_h$ | `eq_adults` | Household's equivalised size on the modified OECD scale: 1 for the first adult, 0.5 per further member of 14 or more, 0.3 per child under 14; thousandths in code | adults | state | society/poverty-inequality |
| $\tilde y_h$ | `hh_real_income_recent` | Household's recent real income: a moving average (1/3 a month) of its money income at prices of day 0 | LCU of day 0 / month | state | society/poverty-inequality |
| — | `hh_income_year` | Household's money income in the calendar year so far | LCU | state | society/poverty-inequality |
| $m^{2021}$ | `poverty_anchor` | Median equivalised recent income of month 0 (December 2021), real; frozen, the base of the class lines | LCU of day 0 / month | parameter | society/poverty-inequality |
| — | `median_eq_income` | Weighted median over persons of equivalised recent income | LCU / month | derived | society/poverty-inequality |
| — | `poverty_rate` | At-risk-of-poverty rate: persons in private households below 60% of the current median equivalised income | ratio | derived | society/poverty-inequality |
| — | `poverty_rate_anchored` | Persons below 60% of `poverty_anchor` in real terms (the class at risk) | ratio | derived | society/poverty-inequality |
| — | `gini` | Gini coefficient of equivalised income over persons | 0–1 | derived | society/poverty-inequality |
| — | `s80_s20` | Equivalised income of the richest fifth of persons over that of the poorest fifth | ratio | derived | society/poverty-inequality |
| — | `hardship_rate` | Share of persons living in a household in hardship | ratio | derived | society/poverty-inequality |
| — | `class_share[k]`, `class_flows[k]` | Share of persons in standard-of-living class *k* (at risk, lower-middle, middle, affluent), and persons who entered and left it since the last close | ratio, persons | derived | society/poverty-inequality |
| — | `poverty_line_share` | Poverty line as a share of the median (0.6, Eurostat) | ratio | parameter | society/poverty-inequality |
| — | `class_lines` | Bounds of the standard-of-living classes as multiples of `poverty_anchor`: 0.60, 0.75, 2.00 (owner, 2026-10-26) | ratio | parameter | society/poverty-inequality |
| $\lambda_y$ | `income_recent_speed` | Share of the gap to the month's real income that `hh_real_income_recent` closes each month (default 1/3) | ratio / month | parameter | society/poverty-inequality |
| — | `basket_share[g]` | Share of household spending that goes to good *g* (from the input-output table; fixed in M2) | ratio | parameter | economy/households |
| — | `unit_stagger_day` | The firm unit's own day of the month (1 to 28) for its plan and staffing (ADR-0017) | day of month | state | economy/labor-market |
| — | `months_unemployed` | Months since the person lost a job or started looking | months | state | economy/labor-market |
| — | `pay_share[f]` | Share of its wage bill firm unit *f* paid on the last pay day (1 when it paid in full; below 1 only under the firm-payment backstop); millionths in code | ratio | state | economy/labor-market |
| — | `wages_unpaid` | Gross wages firm units owed on pay day and could not pay (the firm-payment backstop); not a debt in v1 | LCU / month | flow | economy/labor-market |
| — | `taxes_unpaid` | Taxes and other payments to the government that firm units owed and could not pay (the firm-payment backstop); not a debt in v1 | LCU / month | flow | economy/labor-market |
| $N^*_f$ | `desired_employment[f]` | Staff a firm unit wants, from its production plan | persons | state | economy/production |
| $D^e_f$ | `expected_demand[f]` | Sales a firm unit expects | units / month | state | economy/production |
| $S_{l,k}$ | `budget_spending[l,k]` | Spending of budget line *l* on economic type *k* (wages, goods and services, social benefits, subsidies, interest, investment, other) in the starting budget; integer bani in code | LCU / year | parameter | economy/budget-2021 |
| — | `public_posts[f]` | Occupied public posts that public firm unit *f* stands for at the start; its starting headcount target | posts | parameter | economy/budget-2021 |
| — | `pension_points` | A pensioner's pension points; the pension is points × `pension_point_value`, not below the minimum pension. Millionths of a point in code | points | state | economy/social-transfers |
| — | `pension_point_value` | Value of one pension point (1,442 lei on the start date) | LCU / month | parameter | economy/social-transfers |
| — | `last_wage` | Gross wage of the person's last job, kept when it is lost; the base of the unemployment benefit | LCU / month | state | economy/social-transfers |
| — | `benefit_months_left` | Months of unemployment benefit the person has left | months | state | economy/social-transfers |
| $W^{g}$ | `public_wage_bill` | Wages the government pays its employees (ESA D11): what the public units' employees earn | LCU / tick | flow | economy/budget-2021 |
| $A_l$ | `ministry_budget[l]` | Lever: the year's budget of budget line *l* (running and investment parts together); integer bani in code | LCU / year | lever | game/lever-system |
| $s_l$ | `ministry_investment_share[l]` | Lever: the share of `ministry_budget[l]` that goes to investment (projects and maintenance); the rest is running (staff and inputs); basis points in code | ratio | lever | game/lever-system |
| — | `public_pay[f]` | Lever: pay of public firm unit *f*'s employees as a share of their pay on the start date (1 = unchanged); basis points in code | ratio | lever | game/lever-system |
| — | `income_tax_schedule` | Lever: the personal income tax brackets, (threshold, marginal rate) rows from 0; one row is a flat tax. Bani and basis points in code | LCU / month, ratio | lever | economy/taxation |
| — | `deduction_base[d]` | Lever: personal deduction at a low gross wage, by dependants *d* (0 to 4 or more); integer bani in code | LCU / month | lever | economy/taxation |
| — | `deduction_full_until`, `deduction_zero_at` | Lever: gross wage up to which the full personal deduction applies, and from which there is none | LCU / month | lever | economy/taxation |
| — | `child_tax_credit` | Lever: income tax credit per child (none on the start date) | LCU / month | lever | economy/taxation |
| — | `pension_tax_free` | Lever: part of a pension free of income tax | LCU / month | lever | economy/taxation |
| — | `dividend_tax_rate`, `interest_tax_rate` | Lever: tax withheld on dividends and on interest; basis points in code | ratio | lever | economy/taxation |
| — | `profit_tax_rate` | Lever: corporate profit tax rate; basis points in code | ratio | lever | economy/taxation |
| — | `micro_rate_with_staff`, `micro_rate_no_staff` | Lever: micro-enterprise tax on turnover, with and without employees; basis points in code | ratio | lever | economy/taxation |
| — | `micro_threshold` | Lever: turnover a year under which a micro unit pays the micro-enterprise tax | LCU / year | lever | economy/taxation |
| — | `vat_category[g]` | Lever: VAT category of good *g*: standard, reduced, second reduced, zero, exempt | category | lever | economy/vat |
| — | `cas_rate`, `cass_rate`, `cam_rate` | Lever: employee's pension (CAS) and health (CASS) contributions and the employer's labour insurance contribution (CAM), on the gross declared wage; basis points in code | ratio | lever | economy/taxation |
| — | `pillar2_share` | Lever: points of `cas_rate` paid to the person's Pillar II pension fund; basis points in code | ratio | lever | economy/taxation |
| — | `contribution_ceiling` | Lever: gross wage a month above which no contributions are due (0 = no ceiling) | LCU / month | lever | economy/taxation |
| — | `property_tax_rate` | Lever: national tax rate on dwelling value; basis points a year in code | % / year | lever | economy/taxation |
| — | `wealth_tax_threshold`, `wealth_tax_rate` | Lever: household net worth above which the wealth tax is due, and its rate (none on the start date) | LCU, % / year | lever | economy/taxation |
| — | `excise_duty[g]` | Lever: excise duty per unit of good *g* (fuel, tobacco, alcohol); bani per 1,000 units in the lever registry | LCU / unit | lever | economy/excise |
| — | `public_headcount[f]` | Lever: headcount target of public firm unit *f*; starts at `public_posts[f]` | posts | lever | economy/labor-market |
| — | `new_issue_years` | Lever: maturity of new government bonds | years | lever | economy/fiscal-policy |
| — | `minimum_pension` | Lever: the social indemnity for pensioners, the floor of every state pension | LCU / month | lever | economy/social-transfers |
| — | `retirement_age[sex]` | Lever: standard retirement age by sex, in months of age; changes are phased in | months | lever | economy/demographics |
| — | `pension_indexation` | Lever: how `pension_point_value` is indexed each January: none, prices, wages or a mix | category | lever | economy/social-transfers |
| — | `reference_social_indicator` | Lever: the reference social indicator (ISR) that unemployment benefits are multiples of | LCU / month | lever | economy/social-transfers |
| — | `ui_isr_share` | Lever: flat part of the unemployment benefit, as a share of the ISR; basis points in code | ratio | lever | economy/social-transfers |
| — | `ui_wage_share[b]`, `ui_months[b]` | Lever: the unemployment benefit's share of `last_wage` and its months, by band *b* of contribution years | ratio, months | lever | economy/social-transfers |
| — | `child_allowance_under2`, `child_allowance` | Lever: child allowance a month under 2, and from 2 to 18 | LCU / month | lever | economy/social-transfers |
| — | `gmi_floor[n]`, `gmi_per_extra` | Lever: guaranteed minimum income floor for a household of *n* (1 to 5) members, and the amount added per further member | LCU / month | lever | economy/social-transfers |
| — | `reserve_requirement` | Lever: reserves banks must hold, as a share of deposits; basis points in code | ratio | lever | economy/money-banking |
| — | `qe_purchases` | Lever: government bonds the central bank buys (> 0) or sells (< 0) a month | LCU / month | lever | economy/monetary-policy |
| — | `cb_deficit_funding` | Lever: the central bank's direct funding of the deficit a month; it adds to `monetary_financing` | LCU / month | lever | economy/monetary-policy |
| — | `fx_regime` | Lever: exchange-rate regime: float, managed float, peg to the euro | category | lever | economy/trade-fx |
| — | `fx_intervention` | Lever: foreign currency the central bank buys (> 0) or sells (< 0) a month beyond its rule | LCU / month | lever | economy/trade-fx |
| — | `fx_devaluation` | Lever: a one-off step of `exchange_rate` on its effective day (> 0: the leu weaker); the managed float continues from the new level; basis points in code | % | lever | economy/trade-fx |
| — | `inspection_budget`, `audit_budget` | Lever: labour inspection and tax audit spending | LCU / year | lever | economy/informal-economy |
| — | `penalty_multiple` | Lever: fine as a multiple of the evaded tax; tenths in code (10 = 1×) | ratio | lever | economy/informal-economy |
| — | `foreign_price_growth[b]`, `foreign_demand_growth[b]` | World setting: growth a year of import prices and of demand for exports, by bloc *b* (EU, non-EU); basis points in code | % / year | lever | economy/trade-fx |
| — | `fuel_price[fuel]` | World setting: world price of oil (a barrel), gas (a MWh) and coal (a tonne) | LCU / unit | lever | economy/energy |
| — | `foreign_rate` | World setting: interest rate abroad (the ECB's); basis points in code | % / year | lever | economy/trade-fx |
| $Q_r$ | `home_quality[r]` | Expected quality of life at home in region *r* that migrants weigh: the wellbeing index and jobs, a slow memory, relative to the start | ratio | derived | economy/migration-remittances |
| — | `sol` | Standard of living of a household: 10 + 10·log2 of its real actual consumption per equivalised adult (purchases, home-grown food, imputed rent, public health and education in kind) over the subsistence basket; 10 is subsistence | points (0–100) | derived | society/poverty-inequality |
| $W_r$ | `wellbeing_index[r]` | Wellbeing index of region *r* (and the nation): geometric mean of life expectancy, schooling and SoL dimension indices | index (0–1) | derived | society/social-outcomes |
| — | `mean_years_schooling[r]`, `expected_years_schooling[r]` | Mean years of schooling of residents 25 and over; expected years from the enrolment shares of ages 5 to 29 | years | derived | society/social-outcomes |
| — | `wages_abroad` | World setting: wages abroad relative to Romania's at the start; basis points in code | ratio | lever | economy/migration-remittances |
| — | `age`, `sex`, `role` | Person's age (months), sex (F, M) and role in the household (head, partner, child, other) | months / category | state | society/population-groups |
| — | `person_key` | Stable identity of a person, never reused; the entity of keyed draws about the person (indices change at compaction) | id | state | economy/demographics |
| — | `hh_key` | Stable identity of a household, never reused; the entity of keyed draws about the household | id | state | society/households-lifecycle |
| — | `origin_household` | `hh_key` of the household a person left when leaving home; the kinship link estates use | id | state | society/households-lifecycle |
| — | `hh_moved_to_date` | Money moved into a household by demographic operations (leaving home, unions, separations, estates) since day 0, net, per household | LCU | state | society/households-lifecycle |
| — | `alive`, `hh_alive` | The person's or household's record counts (false for a tombstone, kept until the yearly compaction) | bool | state | economy/demographics |
| — | `months_since_birth` | Months since a woman last gave birth in the run (none if she has not) | months | state | economy/demographics |
| — | `mortality_rate[sex, a]` | Probability of dying within a year at age *a* (pre-pandemic 2017–2019 table); monthly parts per billion in code | ratio / year | parameter | economy/demographics |
| — | `mortality_improvement` | Yearly fall of every death probability from the base year (default 1.5%) | % / year | parameter | economy/demographics |
| — | `fertility_rate[a]` | Births per woman aged *a* a year (2019 rates) | births / woman / year | parameter | economy/demographics |
| — | `sex_ratio_at_birth` | Share of boys among births (default 0.514) | ratio | parameter | economy/demographics |
| — | `partner_fertility_ratio` | How much more likely a woman living with a partner is to give birth than one who is not (default 3) | ratio | parameter | economy/demographics |
| — | `contribution_months` | Months of pension contributions of a person | months | state | economy/demographics |
| — | `career_points` | Sum of a person's yearly pension points (declared wage over `wage_nominal`); millionths of a point in code | points | state | economy/demographics |
| — | `survivor_points` | Pension points a widowed person may draw as a survivor (`survivor_share` of the deceased spouse's) | points | state | economy/demographics |
| — | `retirement_age_schedule[sex]` | The law's dated schedule of the standard retirement age (women 61y9m in December 2021 rising to 63 in January 2030; men 65) | months | parameter | economy/demographics |
| — | `full_contribution_years`, `min_contribution_years` | Law: the full contribution period that divides career points (35) and the minimum for any pension (15) | years | parameter | economy/demographics |
| — | `early_retirement_rate[sex, y]` | Yearly hazard of retiring *y* whole years before the standard age, derived from the starting population | ratio / year | parameter | economy/demographics |
| — | `early_retirement_reduction` | Law: pension reduction per month of retirement before the standard age (0.75%) | ratio / month | parameter | economy/demographics |
| — | `survivor_share` | Law: share of the deceased spouse's pension points a survivor may draw (50%) | ratio | parameter | economy/demographics |
| — | `career_density[activity]` | Share of a person's adult years with contributions, by today's activity, for the career proxy at load | ratio | parameter | economy/demographics |
| — | `nonemployed_wage_share` | Career wage of a person who is not an employee at load, over the mean wage of employees of the same sex and education, for the career proxy (default 0.8) | ratio | parameter | economy/demographics |
| — | `fertility_benefit_response` | Semi-elasticity of the birth probability to the extra child support a newborn brings (vs the 2021 law) relative to household income per equivalised adult (default 0.1) | ratio | parameter | economy/demographics |
| — | `unemployed_fertility_ratio` | Birth probability of a woman who, or whose partner, is unemployed, relative to others (default 0.85) | ratio | parameter | economy/demographics |
| — | `fertility_cycle_response`, `normal_income_growth` | Response of births to real income growth above or below the normal growth (defaults 0.2 and 2% a year) | ratio, % / year | parameter | economy/demographics |
| — | `fertility_lag` | Months from a change in policy or income to its effect on births (default 9) | months | parameter | economy/demographics |
| — | `population`, `births`, `deaths` | Weighted live persons; births and deaths a month or year | persons | derived | economy/demographics |
| — | `life_expectancy[sex]`, `total_fertility_rate` | Period life expectancy at birth and total fertility rate from the year's rates | years / births per woman | derived | economy/demographics |
| — | `old_age_dependency_ratio`, `pensioners` | Persons 65+ over persons 15–64; weighted persons drawing a state pension | ratio / persons | derived | economy/demographics |
| — | `leave_home_rate[sex, band]`, `union_rate[band]`, `separation_rate` | Yearly rates of leaving the parental home, forming a couple (women living alone) and separating | ratio / year | parameter | society/households-lifecycle |
| — | `leave_home_share`, `separation_share` | Share of the household's money a leaver (of an adult's part) or a separating partner takes | ratio | parameter | society/households-lifecycle |

Indices: $g$ good, $j$ industry (same set as goods, except that the eight electricity generation industries share the good `wholesale_electricity` and `power_supply` sells `electricity`), $f$ firm unit (belongs to one industry; per-industry variables such as `output[j]` are also kept per unit as `output[f]`, and the industry value is the `firm_count`-weighted sum), $o$ occupation, $s$ skill tier, $r$ region, $c$ county, $h$ household, $i$ person, $p$ power plant, $\ell$ education level, $l$ budget line, $k$ economic type of spending. LCU = leu (RON).

## Terms
- **SFC (stock-flow consistent):** modelling approach where every flow changes a stock and all balance sheets reconcile.
- **Synthetic population:** the weighted sample of households and persons that represents Romania's population. See society/population-groups and ADR-0003.
- **Group:** any filter over persons (e.g. students in Cluj). Not stored; computed. Replaces the earlier "pop" concept.
- **Interest group:** an overlapping group with per-person membership intensity. See society/interest-groups.
- **Alignment:** selecting exactly the expected number of persons for a transition, ranked by individual probability.
- **Industry / good:** each industry produces exactly one good or service with the same id. Exception: the electricity generation industries (`power_nuclear`, `power_hydro`, `power_coal`, `power_gas`, `power_wind`, `power_solar`, `power_biomass`, `power_storage`) all produce `wholesale_electricity`, and `power_supply` sells the retail good `electricity`. See economy/industries.
- **Wholesale electricity (`wholesale_electricity`):** the homogeneous good sold by all generation industries, priced by merit order. See economy/energy.
- **Merit order:** dispatching plants from the lowest marginal cost upward; the marginal plant sets `wholesale_electricity_price`. See economy/energy.
- **Capture price:** the average price a technology actually earns; for solar and wind it falls as their share rises (cannibalisation). See economy/energy.
- **Contract for difference (CfD):** a state contract per technology that pays a generator the strike price minus the market price (or receives the difference when the market price is higher). See economy/energy.
- **Prosumer:** a household or firm owning rooftop panels; self-consumption lowers its electricity purchases. Not a generation industry. See economy/energy.
- **Skill tier:** low / mid / high job requirement. See economy/labor-market.
- **Project:** something the government builds over time (infrastructure, power plant, school). See economy/infrastructure.
- **SOE:** state-owned enterprise; a firm unit whose `controller[f]` is the state, usually a named firm. Minority state stakes only receive dividends. See economy/state-enterprises and ADR-0016.
- **Ownership vector (`ownership[f]`):** a firm unit's equity shares by holder (state, domestic private, foreign). Dividends and sale proceeds are paid pro rata to it. See ADR-0016.
- **Controller (`controller[f]`):** the holder with ≥ 50% of a firm unit (else the largest bloc); state-controlled units follow SOE directives, foreign-controlled units follow foreign-ownership rules, domestic-private-controlled units the default profit rules. See ADR-0016.
- **Pillar II pension funds:** a financial sub-sector holding equity, bonds and deposits on behalf of member households (`hh_pension_assets`). See economy/accounting and ADR-0016.
- **Non-market producer:** a government-owned firm unit (public administration, public schools, hospitals, social care) whose output is valued at cost and paid from the budget. See ADR-0016.
- **Administration:** the state of an insolvent named firm that keeps operating for `administration_ticks` while it can be rescued (bailout, nationalisation or buyer); if not, it is absorbed into its size-class cohort. See ADR-0016.
- **Firm unit:** one row of the firm table: a named firm or a cohort firm, belonging to one industry, with its own balance sheet, employment by occupation, regional capacity vector and ownership shares. An industry is the `firm_count`-weighted sum of its units. See ADR-0016.
- **Named firm:** a real company (real name) modelled as its own firm unit with a firm count of 1; 0–5 per industry, chosen by a threshold (≥ 5% of industry output or employment, top ~20 exporter, or SOE with ≥ 1% of output). Defined in the `firms` data file, never in code. See ADR-0016.
- **Cohort firm:** a representative firm unit standing for all non-named firms of one size class (micro < 10 employees, small 10–49, medium/large 50+) in an industry, split into domestic and foreign-owned cohorts where the foreign share is material. Its variables are per real firm. See ADR-0016.
- **Firm count (`firm_count`):** the integer number of real firms a firm unit represents; the firm equivalent of `hh_weight`. Postings with sector-level counterparties are multiplied by it; bankruptcies and entries change it. See ADR-0016 and ADR-0007.
- **Tick:** one simulation step: one day. Each process runs at its own period (daily, monthly, yearly). See ADR-0017.
- **Day:** the tick counter: days since the scenario's start date, which is day 0 (`Day` in `econ-types`). The `Calendar` of a scenario turns it into a **date** of the real calendar. **Month** as a type is a calendar month counted from the start date's month: the index of a monthly statistic, not a tick.
- **Period:** how often a process runs: daily, staggered (once a month for each household or firm unit, on its own day 1 to 28: `hh_stagger_day`, `unit_stagger_day`), on a fixed day of the month, at month end, quarterly or yearly (`Period` in `econ-core`). See ADR-0017.
- **Lever:** a variable directly set by the player.
- **Sample scale (`sample_scale`):** real people per synthetic person; 1 : 100 by default. The research reports call it `scale` (pipeline parameter) and `scale_factor` (save manifest); in this repo both are `sample_scale`. Recorded in every save, history block and export header. See ADR-0003.
- **Bani (`Bani`):** the money type in code: whole bani (1 RON = 100 bani) as a 64-bit integer with checked arithmetic. Specs still state money in LCU. See ADR-0007.
- **Ledger:** the double-entry record of every money movement. Mechanics move money only by posting to it. See economy/accounting and ADR-0007.
- **Posting (transfer):** one balanced ledger entry `{debit, credit, flow code, amount > 0}`. Postings with several legs (e.g. a wage with tax withholding) are linked and apply all or nothing. Postings between a weighted record and a sector are multiplied by the record's integer weight.
- **Flow code (reason code):** the code on every posting that says why money moved, e.g. `tax.income`. Stored as an append-only `u16` (never renumbered or reused); sums by flow code give the national accounts and the "why" panel. "Reason code" in economy/accounting means the same thing.
- **Discrepancy account:** a named `9xxx` account that holds residuals from reconciling the opening data, so nothing is hidden in a silent plug.
- **Invariant:** an accounting or conservation check run every tick (I-1 to I-8 in ADR-0007).
- **Clearing account:** a per-industry ledger account (`wage_clearing[j]`, `sales_clearing[j]`, dividends) through which flows between two weighted records pass: payers post totals (per-unit amount × weight), receivers are paid per unit × their own weight. It nets to zero every tick (I-8). See ADR-0007.
- **Behaviour rule:** a decision rule (price, wage, consumption, …) written with the `behaviour_rule!` macro as named terms with a shape (additive, log-linear or partial adjustment), so its result can be explained. See ADR-0008.
- **Contribution tree:** the nested breakdown returned by `explain()`; children sum exactly to the change in the parent indicator. See ADR-0008.
- **State hash:** a hash of the full simulation state after a tick; used to prove determinism across runs, machines and thread counts. See ADR-0006.
- **Golden run:** a committed seed + command log whose per-tick state hashes are checked in CI. Changing one needs a stated economic reason. See ADR-0010.
- **Spike:** a short, throwaway-quality prototype that settles one hard technical decision before mechanics are built. See the roadmap.
