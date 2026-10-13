---
id: research/inactivity-enrolment-romania
title: Kinds of inactivity and enrolment in Romania, 2021 — what data exists
status: draft
owner: horia
depends_on: [research/overview]
updated: 2026-10-13
---

# Kinds of inactivity and enrolment in Romania, 2021 — what data exists

## Question
Stage C of the population generator ([population-attributes](../../02-design/society/population-attributes.md)) leaves two gaps: 2.25 million persons are "other inactive" with no reason given, and pupils and students have no level of education they are enrolled at. What can Stage K ([population-inactivity-enrolment](../../02-design/society/population-inactivity-enrolment.md)) be built from?

## Short answer
- **Enrolment is measured well.** Eurostat's education statistics (`educ_uoe_enra02`) give Romania's pupils and students of the school year 2021/22 by level, sex and single year of age: 3.50 million, the same total as INS's own count (3,495,800). Confidence high.
- **Homemakers are counted by the census.** INS's final Census 2021 results split the 10.87 million inactive into pupils and students, pensioners, homemakers (1,169,829), persons supported by others, by the state, by other sources, and other situations. The split is not by age or sex in what is published; "over 80%" of homemakers are women. Confidence high for the count, medium for the sex split.
- **Inactive for reasons of health is not counted.** Neither the census tables on Eurostat nor the Labour Force Survey's published tables give it for Romania in 2021. About 415,000 adults of 18 to 64 hold a disability certificate, but most of them draw an invalidity pension (the census counts them as retired) or work. Stage K assumes 200,000. Confidence low.
- **The new Labour Force Survey asks the reason only of those who want to work.** Since 2021 (`lfsa_igar`) the reason for not seeking work is asked only of the inactive who would like a job (252,000 aged 15 to 64 in Romania, 2021), so it cannot split all the inactive. More than half of those (56%) believe no job is available: the discouraged.
- **The census and the school statistics disagree at some ages.** The census gives everyone aged 6 to 14 as below the working age (Stage C makes them all pupils), but only 80–90% of them are counted in school. At 15 to 17 the census has more persons "in education" than the schools count; from 18 on it has fewer, because students who work are counted as employed.

## Findings

### 1. Enrolment by level, sex and age (confidence: high)
Eurostat `educ_uoe_enra02`, Romania. Eurostat labels a school year by the calendar year in which it ends: "2022" is 2021/22, which INS reports as 3,495,800 pupils and students (3,494,600 in 2020/21; [INS, as reported](https://www.g4media.ro/ins-a-crescut-numarul-elevilor-cu-1-200-de-la-un-an-la-altul-este-prima-crestere-de-acest-fel-dupa-2014.html)). Counts are taken at the start of the school year; ages are on 1 January ([Eurostat metadata](https://ec.europa.eu/eurostat/cache/metadata/en/educ_uoe_enr_esms.htm)). Short-cycle tertiary (ISCED 5) is not reported for Romania.

| Level (ISCED 2011) | Pupils and students, 2021/22 |
|---|---|
| Early childhood (0) | 539,588 |
| Primary (1) | 866,699 |
| Lower secondary (2) | 743,242 |
| Upper secondary (3) | 702,775 |
| Post-secondary non-tertiary (4) | 89,480 |
| Bachelor (6) | 356,331 |
| Master (7) | 175,282 |
| Doctorate (8) | 22,394 |
| **All** | **3,495,791** |

Patterns worth keeping:
- Pre-school: 37,000 at age 2, 134,000 at 3, 161,000 at 4, 169,000 at 5, and 33,000 still in kindergarten at 6.
- Primary at 6 to 10 (the preparatory class at 6), lower secondary at 11 to 14, upper secondary at 15 to 18; repeaters and late starters give tails of a few thousand on each side.
- At 19, most are in tertiary (62,000 bachelor, 6,800 master: integrated long programmes such as medicine are ISCED 7 from the first year).
- From 25 on, about 2,300 persons of each year of age are in upper secondary and 1,000–2,700 in post-secondary: adult and second-chance education. The flat profile suggests the national figures are spread over ages by a rule.

### 2. The inactive population in the census (confidence: high)
INS, Census 2021 final results ([Curs de Guvernare, from INS](https://cursdeguvernare.ro/capitalul-uman-populatia-activa-s-a-redus-cu-1-milion-in-ultimii-10-ani-cea-inactiva-se-pastreaza-a-crescut-numarul-pensionarilor-si-cel-al-elevilor-si-studentilor.html)):

| Category | Persons, 1 December 2021 | 2011 |
|---|---|---|
| Pupils and students | 3,477,027 | |
| Pensioners | 4,288,077 | 4,268,000 |
| Homemakers (*casnice*) | 1,169,829 | about 1,185,000 |
| Supported by other persons | 1,049,167 | 1,665,000 |
| Supported by the state or other organisations | 112,636 | 200,996 |
| Supported from other sources | 273,215 | 181,382 |
| Other economic situation | 498,815 | |
| **Inactive** | **10,868,766** | |

The Eurostat census table that Stage C uses (`cens_21a_r2`) groups these differently: "below the minimum working age" (3.07 million), "in education" from 15 (1.13 million), "retired or living on capital income" (4.41 million) and "other inactive" (2,252,621). Children not yet at school are inside "supported by other persons". So "other inactive" is, roughly, homemakers (1.17 million) plus the persons of 15 or more supported by others, by the state or from other sources, and those in other situations (about 1.08 million). The census does not say who among them is ill or disabled.

### 3. Disability (confidence: medium for the count, low for the inactive part)
- 856,069 persons with disabilities were registered on 31 March 2021, of whom 414,773 adults of 18 to 64 ([Ministry of Labour, as reported](https://economedia.ro/ministerul-muncii-856-069-persoane-cu-dizabilitati-erau-inregistrate-la-31-martie-2021.html)). The age split in the report is given both as a share and as a count, and they do not quite agree; the count is used.
- About half a million persons drew an invalidity pension in 2021 (the pension statistics, from memory; confidence medium). The census counts them as pensioners, so they are "retired" in Stage C, not "other inactive".
- Neither the census tables on Eurostat nor the published Labour Force Survey tables give the inactive by reason of health for Romania in 2021. EU-SILC asks for a self-defined status that includes "unable to work due to long-standing health problems", but Eurostat publishes it only grouped into "other inactive" (`ilc_lvhl02`).

### 4. Reasons for not seeking work (confidence: high, but narrow)
Eurostat `lfsa_igaww` and `lfsa_igar`, Romania 2021, ages 15 to 64: 4.25 million inactive persons not seeking work, of whom 252,000 would like to work. Their reasons (`lfsa_igar`, percent of those 252,000): believe no job is available 55.6; care of children or adults with disabilities 10.9; other family or personal reasons 12.0; other 13.5; education 3.6; lay-off 2.6. Since the 2021 change of the survey the reason is asked only of those who want to work, so it does not split the 2.25 million "other inactive".

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Pupils and students, school year 2021/22 | 3,495,791 | Eurostat `educ_uoe_enra02` (2022) | high |
| Tertiary students (ISCED 6–8) | 553,007 | same | high |
| Homemakers, Census 2021 | 1,169,829 | INS | high |
| Share of homemakers who are women | over 80% | INS, as reported | medium |
| Persons "other inactive", Census 2021 | 2,252,621 | Eurostat `cens_21a_r2` | high |
| Adults 18–64 with a disability certificate, March 2021 | 414,773 | Ministry of Labour | medium |
| Inactive 15–64 who want to work but do not seek | 252,200 | Eurostat `lfsa_igaww` | high |
| of whom believe no job is available | 55.6% | Eurostat `lfsa_igar` | high |

## Implications for the game
- [Social transfers](../../02-design/economy/social-transfers.md) pay disability and sickness benefits to the inactive for reasons of health, and student grants to students: both need Stage K's columns.
- [Education](../../02-design/society/education.md) reads `enrolled`; the start should match the school statistics level by level.
- The [labour market](../../02-design/economy/labor-market.md) can treat the three kinds differently when jobs appear: "other" (including the discouraged) re-enters most easily, homemakers when childcare and pay allow, the disabled least.
- **Educated guesses** (owner's rule of 2026-10-11): 200,000 inactive for reasons of health, 55% of them men (low); 82% of homemakers women (medium); the age pattern of each kind (low). The census totals of "other inactive" by sex and age are kept exact, and so is the census count of homemakers.
- **Better data, if wanted later:** the INS census tables by age and sex of the inactive categories (published on the census website, not on Eurostat), and the Labour Force Survey's microdata variable for self-declared main status.

## Sources
- [Eurostat `educ_uoe_enra02`](https://ec.europa.eu/eurostat/databrowser/view/educ_uoe_enra02/default/table) — pupils and students by level, sex and age.
- [Eurostat metadata, education statistics](https://ec.europa.eu/eurostat/cache/metadata/en/educ_uoe_enr_esms.htm) — reference period and age date.
- [Curs de Guvernare: Census 2021, active and inactive population](https://cursdeguvernare.ro/capitalul-uman-populatia-activa-s-a-redus-cu-1-milion-in-ultimii-10-ani-cea-inactiva-se-pastreaza-a-crescut-numarul-pensionarilor-si-cel-al-elevilor-si-studentilor.html) — the inactive categories of INS's final results.
- [Forbes România: first final census results](https://www.forbes.ro/?p=307163) — active 8.1 million, inactive 10.8 million, structure of the inactive.
- [Economedia: 856,069 persons with disabilities on 31 March 2021](https://economedia.ro/ministerul-muncii-856-069-persoane-cu-dizabilitati-erau-inregistrate-la-31-martie-2021.html) — disability counts by age.
- [G4Media: pupils and students in 2021/22](https://www.g4media.ro/ins-a-crescut-numarul-elevilor-cu-1-200-de-la-un-an-la-altul-este-prima-crestere-de-acest-fel-dupa-2014.html) — INS total, matching Eurostat's "2022".
- [Eurostat `lfsa_igar`](https://ec.europa.eu/eurostat/databrowser/view/lfsa_igar/default/table) and [`lfsa_igaww`](https://ec.europa.eu/eurostat/databrowser/view/lfsa_igaww/default/table) — reasons for not seeking work, and willingness to work.
