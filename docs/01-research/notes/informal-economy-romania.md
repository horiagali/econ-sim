---
id: research/informal-economy-romania
title: The informal economy of Romania in 2021 — how big, where, and who earns it
status: draft
owner: horia
depends_on: [research/overview]
updated: 2026-10-10
---

# The informal economy of Romania in 2021 — how big, where, and who earns it

## Question
How large is the part of Romania's economy that is not declared, what is it made of, and which people of the starting population earn it? The question came up in two places: the wage and pension stages of the population generator could not place the national accounts' "mixed income" of households, and the [informal economy](../../02-design/economy/informal-economy.md) spec had no numbers.

## Short answer
- National accounts already contain the undeclared economy. Romania's GDP includes estimates of hidden and informal production, so the wage bill and the households' mixed income that the model uses as totals **include** undeclared pay and undeclared sales. Nothing has to be added; what the model must do is split each total into a declared and an undeclared part, because only the declared part is taxed.
- Three measured anchors exist. **Undeclared work:** 21.7% of labour input and 27.1% of gross value added of the private sector in 2019, the highest in the EU. **VAT:** 36.7% of the VAT that should have been collected in 2021 was not, about 9 billion euro, also the highest in the EU. **Mixed income of households:** 160 billion lei in 2021, 15% of gross value added.
- Mixed income is three different things: farming by households (about 45 billion lei, much of it eaten by the household that grew it), declared self-employment, and informal production that the statistics office attributes to households (the largest part). Per self-employed person of the census it would be about 17,000 lei a month, which shows that most of it is not the income of the people the census calls self-employed.
- About one million people farm for themselves and are counted as self-employed by the national accounts but not as employed by the 2021 census, because since 2021 the labour force definition leaves out people who produce only for their own use.

## Findings

### 1. What the national accounts say (confidence: high for the totals)
Eurostat, Romania, 2021, current prices:

| | Billion lei | Source table |
|---|---|---|
| Gross value added | 1,065 | `nama_10_a10` |
| Compensation of employees | 449 (wages and salaries 428) | `nama_10_a10` |
| Mixed income of households | 160 | `nasa_10_nf_tr`, B3G, S14 |
| Operating surplus of households (mostly the imputed rent of owner-occupiers) | 71 | `nasa_10_nf_tr`, B2G, S14 |
| Agriculture: value added less compensation of employees | 45 | `nama_10_a10` |

Employment in the national accounts, 2021 (`nama_10_a10_e`): 6.59 million employees and 1.95 million self-employed, of whom 1.59 million in agriculture. The census of December 2021 has 6.55 million employees and 1.14 million employers, own-account and family workers, of whom 0.55 million in agriculture.

- The two sources agree on employees and disagree by one million on farmers. The census follows the labour force definition in force since 2021, in which a person who produces farm goods only for own consumption is not employed. The national accounts still count them. Confidence that this is the explanation: medium (the size fits; no source was found that states it for Romania).
- Outside agriculture the census has more self-employed (0.50 million employers and own-account workers) than the national accounts (0.35 million). So the 115 billion lei of mixed income that is not farming would be about 27,000 lei a month per self-employed person of the national accounts. That is not a plausible income of small traders and craftsmen. It is where the statistics office books production it estimates but cannot attribute to registered firms.

### 2. Undeclared work (confidence: medium)
European Labour Authority, study of 2023 with the labour input method, reference year 2019 ([study](https://www.ela.europa.eu/sites/default/files/2023-03/Study-on-the-extent-of-undeclared-work-in-the-EU.pdf), [Romania factsheet](https://www.ela.europa.eu/sites/default/files/2024-02/RO-UDW_factsheet-2023-fin.pdf), figures as quoted by the [OECD review of Romania, 2025](https://www.oecd.org/en/publications/oecd-reviews-of-labour-market-and-social-policies-romania-2025_f0532908-en/full-report/main-assessment_1acae3d2.html)):
- 21.7% of total labour input in the private sector was undeclared in 2019 (18.9% in 2013). EU average: 9.7%.
- Undeclared work was 27.1% of private-sector gross value added (26.2% in 2013). EU average: 14.6%.
- Romania is one of seven member states where it rose between 2013 and 2019.

The method compares the labour that households report with the labour that firms report. It therefore includes work that is fully undeclared and the undeclared part of under-declared work ("envelope wages"), by employees and by the self-employed. It does not say how much is which.

Applied to 2021: private-sector value added is about 910 billion lei (total less public administration, education and health). 27.1% of it is about 245 billion lei a year, 20 billion a month. This is larger than the households' mixed income, so part of the undeclared work is inside the wage bill and inside firms' surplus.

### 3. VAT (confidence: medium)
European Commission VAT gap report, as cited by [PwC through Romania Insider](https://www.romania-insider.com/business/pwc-romania-highest-vat-gap-eu-2021): the gap was 36.7% of the VAT total tax liability in 2021, about 9 billion euro (some 44 billion lei), against 37.3% in 2020. The primary report was not read. The gap covers fraud and evasion, but also insolvencies, errors and legal optimisation; evasion is the largest part in countries with a large gap.

### 4. Where it is (confidence: low to medium)
The sources agree on the industries (agriculture, construction, trade, accommodation and food, personal and household services) and on the forms:
- fully undeclared work, mostly casual and seasonal, and by people who are officially unemployed, inactive or retired;
- envelope wages: a contract at or near the minimum wage, the rest in cash. Typical of small firms;
- unreported sales of small firms and the self-employed, which is where the VAT gap and the undeclared profit come from;
- farming for own use, which is not illegal and not taxed, and is in GDP.

No source found here gives the split between these for Romania in a form the model can use directly.

### 5. What was not found
- The statistics office's own figure for the non-observed economy in 2021. A press calculation on older data gave about 22% of GDP ([profit.ro](https://profit.ro/stiri/economie/economia-subterana-a-contribuit-anul-trecut-cu-38-de-miliarde-de-euro-la-pib-ul-romaniei-17023001), for 2016).
- Declared income of the self-employed from tax records.
- The share of employees paid partly in cash, by industry (Eurobarometer surveys have it; not fetched).
- Output of household farms that is consumed by the household (the economic accounts for agriculture, `aact_eaa01`, and the farm structure survey would give it).

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Undeclared work, share of private-sector labour input, 2019 | 21.7% | ELA 2023 | medium |
| Undeclared work, share of private-sector value added, 2019 | 27.1% | ELA 2023 | medium |
| VAT gap, 2021 | 36.7%, about 9 billion euro | European Commission, by way of PwC | medium |
| Mixed income of households, 2021 | 160.0 billion lei (46.1 in the fourth quarter) | Eurostat `nasa_10_nf_tr`, `nasq_10_nf_tr` | high |
| Agriculture, value added less compensation of employees, 2021 | 44.7 billion lei | Eurostat `nama_10_a10` | high |
| Self-employed in the national accounts, 2021 | 1.95 million (1.59 in agriculture) | Eurostat `nama_10_a10_e` | high |
| Self-employed and family workers in the census, December 2021 | 1.14 million (0.55 in agriculture) | Eurostat `cens_21empn_r2` | high |
| People farming for own use, not employed in the census | about 1.0 million | difference of the two rows above | medium |
| Wages and salaries, fourth quarter of 2021 | 111.7 billion lei | Eurostat `namq_10_a10` | high |

## Implications for the game
For the [informal economy](../../02-design/economy/informal-economy.md) spec and the population generator:

1. **Totals stay as they are; each gets a declared and an undeclared part.** The wage of [Stage F](../../02-design/society/population-wages.md) is what an employee is really paid. A share of it is undeclared (the envelope, or all of it for a fully undeclared job). Income tax and contributions apply to the declared part only. The same for firms' sales and VAT.
2. **Mixed income is split in three** before it is given to people:
   - *farm income* (about 45 billion lei a year): to the census's agricultural self-employed and family workers in cash, and to rural households who farm for themselves in kind (food they do not have to buy);
   - *declared self-employment income*: to the non-farm employers and own-account workers of the census, at a level tied to the wages of their occupation;
   - *informal income* (what is left, the largest part): to people by a propensity that depends on industry group, occupation, education, activity and locality size. Recipients include employees with a second undeclared activity and people who are officially unemployed, inactive or retired.
3. **Calibration anchors for the split:** the undeclared share of private labour input (21.7%) and the VAT gap (36.7%). The model should reproduce both at the start, within a wide band, from the same person-level and firm-level shares.
4. **The player sees both economies.** Every indicator of output, employment and income exists as declared and as total. The difference is the map of the informal economy: by county, industry group and any group of people.
5. **Fighting it is a set of levers with costs and side effects,** not a slider for "less evasion": inspections and audits cost money and need administrative capacity; digital reporting takes years to build; penalties without detection do little; lower taxes on low wages cost revenue now. Pushing informal work into the open can also destroy the job, where it only existed because it was untaxed.
6. **People in the informal economy build no pension rights and have no unemployment insurance.** That makes the informal share of a career matter for the pension of new retirees decades later.

Data still to collect before the starting split can be specified with tolerances: the Eurobarometer shares of envelope wages by industry, the output of household farms for own use, and declared self-employment income.

## Sources
- European Labour Authority (2023), *Extent of undeclared work in the European Union*: https://www.ela.europa.eu/sites/default/files/2023-03/Study-on-the-extent-of-undeclared-work-in-the-EU.pdf
- European Labour Authority (2023), factsheet on undeclared work, Romania: https://www.ela.europa.eu/sites/default/files/2024-02/RO-UDW_factsheet-2023-fin.pdf
- OECD (2025), *Reviews of Labour Market and Social Policies: Romania*, main assessment: https://www.oecd.org/en/publications/oecd-reviews-of-labour-market-and-social-policies-romania-2025_f0532908-en/full-report/main-assessment_1acae3d2.html
- Romania Insider (2023), "PwC: Romania continues to have highest VAT gap in the EU": https://www.romania-insider.com/business/pwc-romania-highest-vat-gap-eu-2021
- profit.ro, on the underground economy in the national accounts of 2016: https://profit.ro/stiri/economie/economia-subterana-a-contribuit-anul-trecut-cu-38-de-miliarde-de-euro-la-pib-ul-romaniei-17023001
- Eurostat tables `nama_10_a10`, `nama_10_a10_e`, `namq_10_a10`, `nasa_10_nf_tr`, `nasq_10_nf_tr`, `cens_21empn_r2`, read through the dissemination API on 2026-10-10.
