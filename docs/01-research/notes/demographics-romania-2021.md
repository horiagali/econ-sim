---
id: research/demographics-romania-2021
title: Demographics of Romania, 2021 — births, deaths, retirement and the projections
status: draft
owner: horia
depends_on: [research/overview]
updated: 2026-10-26
---

# Demographics of Romania, 2021 — births, deaths, retirement and the projections

## Question
What does the first version of [demographics](../../02-design/economy/demographics.md) and [household life cycle](../../02-design/society/households-lifecycle.md) need to start from, and what should a 50-year run be compared with? Births, deaths, fertility and mortality by age, life expectancy, age structure, the retirement ages of the 2021 law, how young people leave home and form couples, and the official projections to 2070.

**Owner decision, 2026-10-26: the game runs as if COVID never happened.** 2020 to 2022 carry a large excess of deaths. This note gives both the actual 2021 figures and the pre-pandemic ones (2017–2019) the simulation uses; see "COVID" below.

## Short answer
- **Population** on 1 January 2022: 19.04 million (Eurostat, census-revised); census of 1 December 2021: 19.05 million. Under 15: 3.09 million (16.2%); 65 or more: 3.71 million (19.5%). Confidence high.
- **Births 2021:** 193,191 live births (99,294 boys, 51.4%). Total fertility rate 1.81; mean age of mothers 28.2. 2019: 199,720 births, TFR 1.77. Confidence high.
- **Deaths 2021:** 335,527 (crude rate 17.5 per 1,000), the COVID peak. Pre-pandemic level: 262,321 (2017), 263,911 (2018), 259,889 (2019), average about 262,000 (13.4 per 1,000 in 2019). **The simulation uses the pre-pandemic level.** Confidence high.
- **Life expectancy at birth:** 2021 men 69.2, women 76.6 (total 72.8); 2019 men 71.9, women 79.4 (total 75.6). Pre-COVID trend 2010–2019: about +0.2 years a year. **The simulation uses 2019 (or the 2017–2019 average table).** Confidence high.
- **Retirement ages (Law 263/2010, Annex 5):** men 65 since 2015; women about 61 years 9 months at the end of 2021 (61y10m in 2022), rising to 63 by January 2030. Full contribution period 35 years (women's rising to 35 by 2030), minimum 15. Early pension up to 5 years before the standard age, reduced by 0.75% a month early (partial early pension). Confidence medium.
- **Projections:** EUROPOP2023 (baseline) has Romania falling from 19.04 million (2022) to 15.04 million (2070), with 65+ rising from 19.5% to 29.1% and the old-age dependency ratio from 31 to 51. With no migration: 15.75 million in 2070. Confidence high (that this is what the projection says).
- **Fertility responses** (section 8): child benefits raise fertility modestly (Quebec about +12%, Poland's 500+ from 0 to +4.5%, mostly timing and among targeted groups); recessions lower it a little (about 0.1–0.2% per point of unemployment). The simulation's elasticities are small, bounded and lagged 9 months. Confidence low to medium.
- **Leaving home and couples:** mean age of leaving the parental home 28.0 (men 30.3, women 25.6); 38.7% of 25–34-year-olds lived with their parents in 2021. Marriages 128,610 (2019), 114,207 (2021); divorces 30,197 (2019), about 27,000 (2021, 1.4 per 1,000). Confidence high for the rates, low for how the simulation turns them into hazards.

## Findings

### 1. Population and age structure (confidence: high)
Eurostat `demo_gind` (population on 1 January) and `proj_23np` (the projection's base year):

| | 1 Jan 2019 | 1 Jan 2021 | 1 Jan 2022 |
|---|---|---|---|
| Population | 19,414,458 | 19,201,662 | 19,043,098 |
| Women | 9,914,008 | 9,814,072 | 9,802,984 |
| Men | 9,500,450 | 9,387,590 | 9,240,114 |

On 1 January 2022: 3,089,023 under 15 (16.2%), 3,706,321 aged 65 or more (19.5%); median age 43.7 in 2023 (EUROPOP2023's first year). The census of 1 December 2021 counted 19,053,815 residents; the 2022 figure above is already revised to the census. Net migration in 2021 was −16,228 (Eurostat's corrected figure), turning positive in 2022 (+102,622) — migration is [another spec](../../02-design/economy/demographics.md#migration).

### 2. Births and fertility (confidence: high)
Eurostat `demo_gind`, `demo_find`, `demo_frate`:

| | 2017 | 2018 | 2019 | 2020 | 2021 | 2022 | 2023 |
|---|---|---|---|---|---|---|---|
| Live births | 210,590 | 202,744 | 199,720 | 198,302 | 193,191 | 182,890 | 160,078 |
| TFR | — | — | 1.77 | 1.80 | 1.81 | 1.75 | 1.54 |

2021: 51.4% of births were boys (99,294 of 193,191); mean age at birth 28.2, at first birth 27.1; 52% of births were first children, 30% second, 17% third or later; 33% outside marriage.

**Age-specific fertility rates** (births per woman of that age per year), 2019 and 2021:

| Age of mother | 2019 | 2021 |
|---|---|---|
| 15–19 | 0.0359 | 0.0339 |
| 20–24 | 0.0737 | 0.0764 |
| 25–29 | 0.1091 | 0.1109 |
| 30–34 | 0.0864 | 0.0875 |
| 35–39 | 0.0383 | 0.0411 |
| 40–44 | 0.0080 | 0.0082 |
| 45–49 | 0.0005 | 0.0005 |
| TFR | 1.77 | 1.81 |

Single years are in `demo_frate` (peak 0.117 at 29 in 2021). Fertility was not depressed by the pandemic in 2021 (it was slightly higher than in 2019); the fall comes in 2022–2023. The 2023 TFR (1.54) partly reflects the census revision of the number of women; treat it with care.

**Use:** the 2019 single-year rates (TFR 1.77), close to EUROPOP2023's assumption (1.80 in 2023 falling to 1.77 in 2070). The 2021 rates are an acceptable alternative; the difference is 2%.

### 3. Deaths and mortality (confidence: high)

| | 2017 | 2018 | 2019 | 2020 | 2021 | 2022 | 2023 |
|---|---|---|---|---|---|---|---|
| Deaths | 262,321 | 263,911 | 259,889 | 299,524 | 335,527 | 273,382 | 244,061 |
| of which men | — | — | 136,245 | 159,764 | 174,898 | 143,377 | 127,743 |
| Natural change | — | — | −60,169 | −101,222 | −142,336 | −90,492 | −83,983 |

**Probability of dying within the year** (`demo_mlifetable`, indicator PROBDEATH), selected single years of age:

| Age | Men 2019 | Men 2021 | Women 2019 | Women 2021 |
|---|---|---|---|---|
| 0 | 0.00619 | 0.00567 | 0.00528 | 0.00517 |
| 1 | 0.00051 | 0.00047 | 0.00050 | 0.00046 |
| 20 | 0.00070 | 0.00084 | 0.00023 | 0.00038 |
| 40 | 0.00284 | 0.00307 | 0.00096 | 0.00136 |
| 50 | 0.00827 | 0.00957 | 0.00332 | 0.00389 |
| 60 | 0.02217 | 0.02591 | 0.00822 | 0.01125 |
| 65 | 0.02871 | 0.03792 | 0.01174 | 0.01695 |
| 70 | 0.03882 | 0.05278 | 0.01853 | 0.02687 |
| 80 | 0.08492 | 0.11021 | 0.05929 | 0.07370 |
| 90 | 0.17646 | 0.23925 | 0.17467 | 0.20917 |

Above 60 the 2021 rates are 20–40% higher than 2019's: that is the COVID excess. Romanian men die young by EU standards: at 60 a man's probability is 2.7 times a woman's.

**Life expectancy** (`demo_mlexpec`):

| | 2010 | 2015 | 2018 | 2019 | 2020 | 2021 | 2022 | 2023 |
|---|---|---|---|---|---|---|---|---|
| At birth, men | 70.0 | 71.3 | 71.6 | 71.9 | 70.3 | 69.2 | 71.3 | 72.6 |
| At birth, women | 77.7 | 78.5 | 79.1 | 79.4 | 78.2 | 76.6 | 79.1 | 80.2 |
| At 65, men | | | | 14.9 | 13.4 | 12.5 | 14.0 | 15.0 |
| At 65, women | | | | 18.6 | 17.6 | 16.4 | 18.1 | 19.0 |

**Improvement trend (confidence: medium).** From 2010 to 2019 life expectancy at birth rose about 0.21 years a year (men 0.21, women 0.19). In probabilities of death that is roughly 1% a year at 80 and 2–3% a year at 40 (single-year rates are noisy: men at 80 went from 0.0925 to 0.0849, women from 0.0686 to 0.0593). A uniform improvement of 1.5% a year in every probability is a fair first approximation; EUROPOP2023 assumes a faster gain (men 73.7 in 2030, 79.0 in 2050, 83.3 in 2070; women 80.9, 85.0, 88.5), which is about 0.25 years a year.

### 4. COVID (owner decision, 2026-10-26)
The simulation runs as if the pandemic had not happened. Consequences:
- The **mortality table** is the pre-pandemic one: the average of the 2017–2019 single-year probabilities by sex (or 2019 alone), improved each year by the pre-COVID trend. Applied to the 2021 population it gives about 260,000–265,000 deaths in the first year, not 2021's 335,527.
- **Fertility** is barely affected in 2021; the 2019 rates are used for consistency.
- **Alignment and checks** compare with the pre-pandemic path: deaths and life expectancy of 2017–2019 projected forward. EUROPOP2023 (base year 2022) shows a little of the pandemic in its first years: its 2023 life expectancy (men 71.4, women 79.0) is below the actual 2023 figure (72.6, 80.2), and its projected 2023 deaths (269,660) are above the actual (244,061; the low 2023 figure itself reflects deaths brought forward by COVID). The comparison therefore uses EUROPOP2023's path from 2025 on, and EUROPOP2019 as a pre-COVID cross-check.

### 5. Projections (confidence: high that this is what they say)
**EUROPOP2023** (Eurostat, `proj_23np`, `proj_23ndbi`, `proj_23nalexp`, `proj_23naasfr`), Romania, population on 1 January:

| | 2022 | 2025 | 2030 | 2040 | 2050 | 2060 | 2070 |
|---|---|---|---|---|---|---|---|
| Baseline (BSL) | 19.04 m | 18.83 m | 18.22 m | 17.23 m | 16.44 m | 15.69 m | 15.04 m |
| No migration (NMIGR) | 19.04 m | | 18.48 m | 17.79 m | 17.15 m | 16.44 m | 15.75 m |
| Under 15, BSL | 3.09 m | 2.98 m | 2.77 m | 2.50 m | 2.40 m | 2.22 m | 2.09 m |
| 65+, BSL | 3.71 m | 3.84 m | 3.82 m | 4.36 m | 4.69 m | 4.72 m | 4.37 m |
| 65+ share, BSL | 19.5% | 20.4% | 21.0% | 25.3% | 28.5% | | 29.1% |
| Old-age dependency (65+ / 15–64), BSL | | 32.0 | 32.9 | 42.0 | 50.2 | | 50.9 |
| Births, BSL | | 179,059 | 168,027 | 164,431 | 150,349 | | 136,253 |
| Deaths, BSL | | 259,356 | 251,709 | 243,653 | 231,721 | | 213,280 |
| Median age, BSL | | 44.3 | 45.5 | 47.7 | 47.2 | | 48.7 |

Assumptions: TFR 1.80 (2023) to 1.77 (2070); net migration −37,000 a year to 2030, then turning slightly positive after 2045.

**EUROPOP2019** (pre-COVID, `proj_19np`): baseline 19.28 million (2020) → 13.69 million (2070); no migration 19.35 → 15.25 million. Its baseline assumed far more emigration than EUROPOP2023.

**INS** publishes its own projections ("Proiectarea populației României în profil teritorial, la orizontul 2070/2060", several variants). Not checked in this note; the series is said to show the same direction (a fall below 16 million by 2060 in the reference variant). Confidence low for the numbers; use EUROPOP2023 as the reference.

**What a demographics-only run (no migration) should show:** from 19.05 million at the start to about 16.4–16.8 million fifty years later (EUROPOP2023 NMIGR: 15.75 million in 2070, 48 years after 2022, with faster mortality improvement than the pre-COVID trend would give, so the run should sit within about ±5% of it); the share aged 65+ rising from 19.5% to about 27–29%; the under-15 share falling from 16.2% to about 14–15%; births falling from about 195,000 to about 140,000; deaths staying around 240,000–265,000.

### 6. Retirement (confidence: medium)
Law 263/2010 on the public pension system, as in force on 1 December 2021 (from memory of the law and press reports; check against the legal text when `law.json` is filled, as for the rest of [law in force](law-in-force-romania-2021.md)):
- **Standard retirement age** (`vârsta standard de pensionare`): men 65 (reached in 2015); women rising from 60 (2015) to 63 by January 2030 by birth cohort (Annex 5). Women retiring in 2022 needed 61 years and 10 months ([press report](https://www.bugetul.ro/a-crescut-varsta-de-pensionare-in-romania-pentru-femei/)), so about 61 years 9 months at the start date. In calendar time the rise from 61y9m (December 2021) to 63y (January 2030) is 15 months of age over 97 months, about one month of age every 6.5 months.
- **Full contribution period** (`stagiu complet de cotizare`): 35 years for men; women rising to 35 by January 2030 (about 32 years in 2021). **Minimum contribution period:** 15 years. Below it there is no old-age pension.
- **Early pension** (`pensie anticipată`): up to 5 years before the standard age for those who exceed the full contribution period by at least 8 years, unreduced. **Partial early pension** (`pensie anticipată parțială`): up to 5 years before the standard age with the full contribution period, reduced by 0.75% for each month early (up to 45%), the reduction lifted at the standard age.
- **The formula:** pension = average annual points × the pension point value (1,442 lei in 2021). Each year's points are the person's gross wage over the national average gross wage of that year (`câștigul salarial mediu brut`); the average is the sum of the yearly points divided by the full contribution period. Thirty-five years at the average wage give one point a year on average, so 1,442 lei.
- **Survivor pension** (`pensie de urmaș`): the surviving spouse at the standard age (or earlier if disabled, or with children under 7), married at least 15 years, receives 50% of the deceased's pension (or of the pension he or she would have had); a survivor with a pension of their own chooses the larger. Children receive it until 16 (26 if in education): 50% for one, 75% for two, 100% for three or more beneficiaries. In 2021 survivors' pensions were 6.3 billion lei of the 104.7 billion pension bill ([population-pensions](../../02-design/society/population-pensions.md), Eurostat `spr_exp_pens`).
- **Employment contracts** end by law when the employee reaches the standard age and the minimum contribution period (Labour Code, art. 56(1)(c)), unless the employer agrees to continue.
- **Effective retirement.** The census of 2021 counted 4.41 million retired (including disability and survivor pensioners and capital-income recipients), more than the 3.71 million aged 65 or more: many retire before the standard age (early, disability, special schemes). The share retired by single year of age is in the census table `cens_21a_r2` that Stage C of the population generator already reads.

### 7. Leaving home, couples and separation (confidence: high for the data, low for the hazards)
- **Mean age of leaving the parental household** (Eurostat `yth_demo_030`), 2021: 28.0 (men 30.3, women 25.6). Romania is among the later leavers of the EU.
- **Young adults living with their parents** (`ilc_lvps08`), 2021: 95.7% of 16–19, 83.4% of 18–24, 50.8% of 25–29, 38.7% of 25–34.
- **Marriages** (`demo_nind`): 143,292 (2018), 128,610 (2019), 81,343 (2020), 114,207 (2021). **Divorces** (`demo_ndivind`): 30,857 (2018), 30,197 (2019), 22,785 (2020), about 27,000 in 2021 (1.4 per 1,000 inhabitants); 24 divorces per 100 marriages.
- Cohabitation without marriage is common (a third of births are outside marriage), so marriages undercount unions formed. Educated guess for the simulation (low confidence): about 140,000 new co-resident unions and about 35,000 separations a year, pre-COVID.

### 8. Fertility responses to policy and the economy
The owner decided on 2026-10-26 that fertility responds to policy and the economy from M5. What the literature says, and the parameters the spec takes from it:

| Study | Policy or shock | Finding | Confidence |
|---|---|---|---|
| Milligan (2005), *Review of Economics and Statistics* 87(3) | Quebec's Allowance for Newborn Children, 1988–1997, up to C$8,000 per birth (most for third children) | fertility up about 12% on average, about 25% for the largest (third-child) bonus; a large effect per dollar spent was not found: the cost per extra birth was high | medium (the 12% is from a summary of the paper) |
| Lalive & Zweimüller (2009), *Quarterly Journal of Economics* 124(3) | Austria 1990: paid parental leave extended from 12 to 24 months | more higher-order births within 3 years and within 10 years; part of the effect is timing (births brought forward), part is more children | medium |
| Bokun (2024), *Demographic Research* 51(28) | Poland's Family 500+ (2016: 500 zł a month per child from the second, and for poor families the first; about a fifth to a quarter of income per adult in poor households) | birth probability up 1.5 percentage points in the short run, mostly among women aged 31–40 and lower-income households; younger and higher-income women did not respond | medium |
| Bartnicki & Alimowski (2022), *Studia Socjologiczne* | Family 500+ | births about 4.5% higher a month to the end of 2017, no detectable effect by mid-2019; another study (2022, Journal of Management and Finance) found none | low |
| Gauthier (2007), *Population Research and Policy Review* (review) | cash benefits in rich countries | positive but small effects, often on timing more than on completed fertility | medium |
| Goldstein, Kreyenfeld, Jasilioniene & Örsal (2013), *Demographic Research* 29(4) | the Great Recession in Europe | rising unemployment lowered birth rates: about 0.1–0.2% lower birth rates per percentage point of unemployment, strongest among women under 25 and for first births; little effect over 40 | medium |
| Sobotka, Skirbekk & Philipov (2011), *Population and Development Review* 37(2) | economic recessions (review) | fertility is pro-cyclical in the short run: recessions delay births, mostly of the young | medium |
| Romania, OUG 111/2010 | the child-raising allowance cut in 2011 (from 85% of earnings with a high cap to 75% with a lower cap, plus a one-year option), amid austerity | births fell from about 212,000 (2010) to 196,000 (2011); the cut cannot be separated from the recession and from the census correction | low (attribution); medium (the cut) |

**What the spec takes (all low confidence, tunable in M7):**
- **Benefits:** a semi-elasticity `fertility_benefit_response` = 0.1: a permanent rise in child support worth 10% of a household's income per equivalised adult raises its birth probability by 1%. Check against 500+: support of about 20–25% of a poor household's income per adult gives +2–2.5% here, inside the range of the studies (0 to 4.5%; Bokun's 1.5 points on a yearly birth probability of about 8–10% among women in the relevant ages is about +15–20% for them, but only for the targeted group, and partly timing). Bounded to 0.8–1.3.
- **Unemployment:** `unemployed_fertility_ratio` = 0.85 for women who, or whose partner, are unemployed. With 20–44-year-olds' unemployment up 5 points, about 10% more women in couples are touched (her or him), and births fall by about 1%, which is 0.2% per point: the upper end of Goldstein et al.
- **Cycle:** `fertility_cycle_response` = 0.2: real income growth 5 points below normal lowers births by 1%, bounded to ±3%.
- **Lag:** 9 months from the change to the birth.

## Data gaps and educated guesses
- **Union formation and separation hazards by age** are not published as such. The simulation uses hazards tuned so the share of adults living in a couple by age and the share of 25–34-year-olds living with parents stay near their 2021 values (low confidence).
- **Contribution histories** are not in the population; the pension formula of new retirees uses a proxy (low confidence), calibrated so the average new pension and the pension bill's growth look right.
- **INS projections** not read in detail (see above).
- **Mortality by education or income** exists in the literature but not in official tables for Romania; not used in v1.

## Sources
- Eurostat, population change (`demo_gind`): <https://ec.europa.eu/eurostat/databrowser/view/demo_gind/default/table>
- Eurostat, fertility indicators (`demo_find`) and rates by age (`demo_frate`): <https://ec.europa.eu/eurostat/databrowser/view/demo_find/default/table>, <https://ec.europa.eu/eurostat/databrowser/view/demo_frate/default/table>
- Eurostat, life tables (`demo_mlifetable`) and life expectancy (`demo_mlexpec`): <https://ec.europa.eu/eurostat/databrowser/view/demo_mlexpec/default/table>
- Eurostat, EUROPOP2023 (`proj_23np`, `proj_23ndbi`, `proj_23nalexp`, `proj_23naasfr`): <https://ec.europa.eu/eurostat/databrowser/view/proj_23np/default/table>
- Eurostat, EUROPOP2019 (`proj_19np`): <https://ec.europa.eu/eurostat/databrowser/view/proj_19np/default/table>
- Eurostat, marriages and divorces (`demo_nind`, `demo_ndivind`): <https://ec.europa.eu/eurostat/databrowser/view/demo_nind/default/table>
- Eurostat, age of leaving the parental household (`yth_demo_030`) and young adults living with parents (`ilc_lvps08`): <https://ec.europa.eu/eurostat/databrowser/view/yth_demo_030/default/table>
- Eurostat, pension expenditure (`spr_exp_pens`), as used by [population-pensions](../../02-design/society/population-pensions.md).
- Law 263/2010 on the unitary public pension system, art. 53 and Annex 5: <https://legeaz.net/legea-pensiilor/art-53-lege-263-2010>
- Women's retirement age in 2022: <https://www.bugetul.ro/a-crescut-varsta-de-pensionare-in-romania-pentru-femei/>
- Milligan, K. (2005), "Subsidizing the Stork: New Evidence on Tax Incentives and Fertility", *Review of Economics and Statistics* 87(3): 539–555; working paper <https://www.nber.org/papers/w8845>; summary <https://www.theglobeandmail.com/news/national/paying-the-stork-worked-in-quebec/article25431151>
- Lalive, R. and Zweimüller, J. (2009), "How Does Parental Leave Affect Fertility and Return to Work? Evidence from Two Natural Experiments", *Quarterly Journal of Economics* 124(3): 1363–1402.
- Bokun, A. (2024), Poland's Family 500+ and fertility, *Demographic Research* 51(28): <https://demographic-research.org/articles/volume/51/28>; summary <https://www.niussp.org/fertility-and-reproduction/lessons-from-polands-pro-natalist-family-500-program/>
- Bartnicki, S. and Alimowski, M. (2022), *Studia Socjologiczne* 2022(1): <https://journals.pan.pl/Content/122796/PDF/Studia_Socjologiczne_2022_nr1_s.193_219.pdf>; no-effect study: <https://journal.mostwiedzy.pl/reme/article/view/2689>
- Gauthier, A. H. (2007), "The impact of family policies on fertility in industrialized countries: a review of the literature", *Population Research and Policy Review* 26: 323–346.
- Goldstein, J., Kreyenfeld, M., Jasilioniene, A. and Karaman Örsal, D. (2013), "Fertility reactions to the 'Great Recession' in Europe", *Demographic Research* 29(4): <https://www.demographic-research.org/volumes/vol29/4/29-4.pdf>; press release <https://www.demogr.mpg.de/mediacms/2800_main_MPIDR-PR_Economic_crisis_lowers_birth_rates.pdf>
- Sobotka, T., Skirbekk, V. and Philipov, D. (2011), "Economic Recession and Fertility in the Developed World", *Population and Development Review* 37(2): 267–306.
- INS, regional natural movement 2021 (example of the INS series): <https://calarasi.insse.ro/wp-content/uploads/2023/08/reg_miscarea_naturala_a_populatiei_pe_medii_2021.pdf>

All Eurostat figures were read through the Eurostat dissemination API on 2026-10-26.
