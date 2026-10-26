---
id: research/education-flows-romania
title: Education flows in Romania around 2021 — graduates, early leavers, attainment, dropout
status: draft
owner: horia
depends_on: [research/inactivity-enrolment-romania]
updated: 2026-10-26
---

# Education flows in Romania around 2021 — graduates, early leavers, attainment, dropout

## Question
What yearly flows through school and university (who finishes, who drops out, who goes on) should the first version of [education](../../02-design/society/education.md) (roadmap M5) reproduce, starting from Stage K's enrolment of the school year 2021/22 ([population-inactivity-enrolment](../../02-design/society/population-inactivity-enrolment.md))?

## Short answer
- **Graduates of 2021** (Eurostat `educ_uoe_grad01`): 175,934 completed upper secondary (78,065 general, 97,869 vocational), 31,705 post-secondary non-tertiary, 131,534 tertiary (76,593 bachelor, 52,377 master, 2,564 doctorate). Confidence high.
- **Early leavers from education and training** (18–24, at most lower secondary, not in education or training): 15.3% in 2021, 15.6% in 2022, 16.6% in 2023 (EU 9.7% in 2021). Confidence high.
- **Tertiary attainment of 25–34-year-olds:** 23.3% in 2021 (24.7% in 2022). Confidence high.
- **Dropout during the school year** ("rata abandonului", INS): primary and lower secondary together 1.2% in 2020/21 (about 18,400 pupils). Upper secondary and vocational are higher; no 2020/21 value was found (educated guess 3% a year, confidence low). Tertiary: about a third of a bachelor cohort does not finish (educated guess from graduates against enrolment, confidence low).
- **The stock is about one cohort per year of study.** Stage K's 2021/22 enrolment (3.50 million) has 0.87 million in primary (five grades with the preparatory one), 0.74 million in lower secondary (four), 0.70 million in upper secondary (four, three in vocational), 0.36 million in bachelor programmes (three, four in engineering, six in medicine), 0.20 million in master and doctorate. A year of age has 190,000 to 220,000 persons.

## Findings

### 1. Graduates by level, 2021 (confidence: high)
Eurostat `educ_uoe_grad01`, Romania, reference year 2021 (the school year 2020/21), both sexes, all ages:

| ISCED | Graduates |
|---|---|
| 3 upper secondary | 175,934 |
| of which 34 general | 78,065 |
| of which 35 vocational | 97,869 |
| 4 post-secondary non-tertiary | 31,705 |
| 5–8 tertiary | 131,534 |
| 6 bachelor | 76,593 |
| 7 master | 52,377 |
| 8 doctorate | 2,564 |

Upper-secondary graduates are about 85% of a year of age (about 205,000), but they include adults finishing late and the census overcounts children living abroad (research note on [inactivity and enrolment](inactivity-enrolment-romania.md), finding on the census against the school statistics), so the share of a real cohort that completes is lower: about 75–80% (confidence low).

### 2. Early leavers and attainment (confidence: high)
- Eurostat `edat_lfse_14` (early leavers from education and training, % of 18–24): Romania 15.3 (2019), 15.6 (2020), 15.3 (2021), 15.6 (2022), 16.6 (2023), 16.8 (2024), 15.5 (2025). Among the highest in the EU; the EU 2030 target is 9%.
- Eurostat `edat_lfse_03` (ISCED 5–8, % of 25–34): Romania 25.5 (2019), 24.9 (2020), 23.3 (2021), 24.7 (2022), 22.5 (2023), 23.2 (2024), 23.0 (2025). Flat for a decade: young graduates emigrate, and tertiary entry is limited by the baccalaureate (about two thirds of candidates pass; confidence medium, press reports of the Ministry's results).

### 3. Dropout during the school year (confidence: medium for compulsory school, low for the rest)
- INS defines the dropout rate as the difference between the pupils enrolled at the start of the school year and those on the register at its end, as a share of the start. Primary and lower secondary: 1.2% in 2020/21, about 18,400 pupils ([Ziarul Financiar, citing INS](https://www.zf.ro/special/educatie-stil-de-viata-si-wellness-tabloul-invatamantului-21777674)).
- The Ministry's report on the state of pre-university education for 2020/21 has the rates by level ([edu.ro](https://www.edu.ro/sites/default/files/_fișiere/Minister/2021/Transparenta/Stare%20invatamant/Raport_stare_invatamant_preuniversitar_RO_2020_2021.pdf), section I.D.4); the values for upper secondary and vocational were not extracted here. Earlier years' values reported in Parliament were around 2–4% a year for upper secondary and higher in vocational schools (confidence low).
- Tertiary: bachelor graduates of 2021 (76,593) against a bachelor stock of about 356,000 in three to six years of study suggests that about a third of entrants do not graduate on time or at all (confidence low).

### 4. Pupils outside school
About 380,000 persons of 6 to 19 are not enrolled in the starting population ([Stage K](../../02-design/society/population-inactivity-enrolment.md)); 15–19% of each year of age from 7 to 14. Part is real (never enrolled, early leavers), part is the census counting children who live abroad. The share of a single year of age that is enrolled at 7 (about 0.82) is what the first version keeps for new cohorts.

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Upper-secondary graduates, 2021 | 175,934 | Eurostat `educ_uoe_grad01` | high |
| Post-secondary graduates, 2021 | 31,705 | same | high |
| Bachelor graduates, 2021 | 76,593 | same | high |
| Master and doctorate graduates, 2021 | 54,941 | same | high |
| Early leavers, 18–24, 2021 | 15.3% | Eurostat `edat_lfse_14` | high |
| Tertiary attainment, 25–34, 2021 | 23.3% | Eurostat `edat_lfse_03` | high |
| Dropout, primary and lower secondary, 2020/21 | 1.2% a year | INS via ZF | medium |
| Dropout, upper secondary | about 3% a year | educated guess | low |
| Share of a bachelor cohort that does not finish | about one third | educated guess from graduates and stock | low |
| Baccalaureate pass rate | about two thirds | press reports of the Ministry's results | medium |

## Implications for the game
- The first version of [education](../../02-design/society/education.md) takes its continuation and dropout rates from these numbers and checks graduates, early leavers and attainment against them (AC-EDU-04 to 06).
- The flat tertiary attainment with a large graduate flow is partly emigration of graduates ([migration and remittances](../../02-design/economy/migration-remittances.md)); without emigration the simulated attainment of 25–34 would drift up.

## Sources
- [Eurostat `educ_uoe_grad01`](https://ec.europa.eu/eurostat/databrowser/view/educ_uoe_grad01/default/table) — graduates by level, sex and age.
- [Eurostat `edat_lfse_14`](https://ec.europa.eu/eurostat/databrowser/view/edat_lfse_14/default/table) — early leavers from education and training.
- [Eurostat `edat_lfse_03`](https://ec.europa.eu/eurostat/databrowser/view/edat_lfse_03/default/table) — attainment by age group.
- [Ziarul Financiar, "Tabloul învățământului"](https://www.zf.ro/special/educatie-stil-de-viata-si-wellness-tabloul-invatamantului-21777674) — INS dropout rate of 2020/21.
- [Ministry of Education, state of pre-university education 2020/21](https://www.edu.ro/sites/default/files/_fișiere/Minister/2021/Transparenta/Stare%20invatamant/Raport_stare_invatamant_preuniversitar_RO_2020_2021.pdf) — rates by level.
