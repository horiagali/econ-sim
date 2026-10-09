---
id: glossary
title: Glossary & Variable Registry
status: draft
owner: horia
depends_on: []
updated: 2026-10-09
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
| $n_f$ | `firm_count[f]` | Real firms represented by firm unit *f* (integer; 1 for a named firm). Read from the scenario; never hardcoded (ADR-0016) | count | state | economy/production |
| — | `sample_scale` | Real people per synthetic person (default 100). Read from the scenario; never hardcoded (ADR-0003) | ratio | parameter | society/population-groups |
| — | `ideo_econ`, `ideo_social`, `ideo_national` | Person's ideology axes | −1…+1 | state | society/opinion-approval |
| $m_{i,g}$ | `interest[g]` | Person's membership intensity in interest group *g* | 0–1 | state | society/interest-groups |
| $A_i$ | `approval` | Person's approval of head of state | 0–100 | state | society/opinion-approval |
| $A$ | `approval_national` | Weighted average approval | 0–100 | derived | society/opinion-approval |
| — | `ethnicity`, `mother_tongue`, `religion`, `religiosity` | Person's identity attributes | category / 0–1 | state | society/identity |
| $Y^{disp}_h$ | `hh_disposable_income` | Disposable income of household *h* | LCU / tick | flow | economy/households |
| $C_{h,g}$ | `hh_consumption[g]` | Household consumption of good *g* | LCU / tick | flow | economy/households |
| $P_h$ | `hh_price_index` | Personal price index of household *h* | index | derived | economy/households |
| $Rem_h$ | `hh_remittances_in` | Remittances received from abroad | LCU / tick | flow | economy/demographics |
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
| — | `vat_gap` | Share of theoretical VAT not collected | ratio | derived | economy/informal-economy |
| — | `foreign_share[j]` | Foreign-owned share of industry *j* capital, derived from its firm units' ownership shares (ADR-0016) | 0–1 | derived | economy/foreign-ownership |
| — | `house_price[c]`, `rent[c]` | House price and rent per m² by county | LCU / m² | state | economy/housing |
| — | `ets_price` | EU ETS carbon allowance price | EUR / tCO₂ | exogenous | economy/environment |
| — | `emissions_co2e` | Greenhouse-gas emissions | tCO₂e / tick | flow | economy/environment |
| — | `air_quality[c]` | PM2.5 index by county | index | state | economy/environment |
| — | `admin_capacity[m]` | Administrative capacity of ministry/agency *m* | 0–1 | state | economy/state-capacity |
| — | `corruption` | Corruption prevalence | 0–1 | state | economy/state-capacity |

Indices: $g$ good, $j$ industry (same set as goods, except that the eight electricity generation industries share the good `wholesale_electricity` and `power_supply` sells `electricity`), $f$ firm unit (belongs to one industry; per-industry variables such as `output[j]` are also kept per unit as `output[f]`, and the industry value is the `firm_count`-weighted sum), $o$ occupation, $s$ skill tier, $r$ region, $c$ county, $h$ household, $i$ person, $p$ power plant, $\ell$ education level. LCU = leu (RON).

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
- **Tick:** one simulation step.
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
