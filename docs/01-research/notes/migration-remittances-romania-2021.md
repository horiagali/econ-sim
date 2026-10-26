---
id: research/migration-remittances-romania-2021
title: Migration, diaspora and remittances, Romania 2021
status: draft
owner: horia
depends_on: []
updated: 2026-10-26
---

# Migration, diaspora and remittances, Romania 2021

## Question
How many people left Romania, came back, came in and moved between counties in 2021; how large is the diaspora; how much do its members send home, and where does it sit in the balance of payments? What do these numbers imply for the starting state and the rates of [migration and remittances](../../02-design/economy/migration-remittances.md)?

## Short answer
- **Emigration 216,861 and immigration 194,642 in 2021** (Eurostat, long-term, 12 months or more). Of the immigrants, **149,513 were Romanian citizens** (returns, 77%), 9,878 other EU citizens and 35,165 non-EU citizens. Of the emigrants, 160,931 were Romanian citizens. Net migration about −22,000 (INS's own release: "a little over −16,000"). Confidence high.
- **Emigrants are young:** 29% are 25–34, 21% 35–44, 18% 15–24; 9% are children under 15 (they leave with their parents), 3% are 65 or over. Confidence high.
- **Personal transfers (remittances, secondary income) were 19,056 million lei received and 1,808 million paid in 2021: 17,248 million net** (€3,872m and €3,505m). This is the roadmap's "about 17 billion". Compensation of employees received (short-term workers abroad, primary income) was a further €3,823m (about 18.8 billion lei). Confidence high.
- **The diaspora is about 3.5 to 4 million people** born in Romania (about 4 million in 25 destination countries in 2020: Italy 0.98m, Germany 0.81m, Spain 0.58m, UK 0.43m, Hungary 0.21m). Population by domicile (22.1m) exceeded the usual-resident population (19.3m) by 2.9 million in mid-2020, a lower bound for long-term absentees still registered at home. Confidence medium.
- **Internal migration: 374,526 changes of domicile between localities in 2021**, 2.0% of the population, mostly young adults and children. Net gains by county are concentrated: Ilfov +13,602, Cluj +4,198, Timiș +3,581, Bucharest +2,972, Brașov +2,247, Iași +1,870, Sibiu +1,498; the poorer counties of Moldavia and the south lose 1,000 to 1,900 each. Confidence high for the totals; the share of moves that cross a county border is not published in the table read (educated guess 35–45%, low).

## Findings

### 1. International migration flows, 2021 (confidence: high)
Eurostat `migr_emi2`, `migr_emi1ctz` and `migr_imm1ctz`, Romania, read through the API on 2026-10-26. The definition is the UN one (a change of usual residence for 12 months or more); INS supplies the figures, estimated from its own sources ("temporary long-term" migrants) and not from registrations.

| | 2019 | 2020 | 2021 | 2022 | 2023 |
|---|---|---|---|---|---|
| Emigrants | 233,736 | 186,818 | **216,861** | 202,311 | 239,244 |
| Immigrants | 202,422 | 145,519 | **194,642** | 293,024 | 324,091 |
| of which Romanian citizens (returns) | 161,286 | 114,469 | **149,513** | 190,205 | 171,588 |

- 2021 immigrants by citizenship: Romanian 149,513; other EU 9,878; non-EU 35,165.
- 2021 emigrants by citizenship: Romanian 160,931; others 55,930 (foreigners leaving, and Romanians who hold another citizenship are counted by their declared one).
- Emigration has been 160,000 to 250,000 a year since 2009 (246,626 in 2009, the low 161,755 in 2013). The large exodus came earlier, around and after EU accession in 2007 (stock figures below). From 2022 the balance turned positive, mainly through returns and non-EU work immigration.
- INS's 2021 release gives a balance of "a little over −16,000" (men 50.2% of emigrants, 54.7% of immigrants); the difference from Eurostat's −22,219 is a revision or definition detail. Confidence high for the order of magnitude.
- OECD: inflows of Romanian citizens to OECD countries were about 215,000 in 2021 (+2%), 43% to Germany, 13% to Italy, 8% to Austria. Confidence high.

**Emigrants by age, 2021** (Eurostat `migr_emi2`, single years summed):

| Age | Emigrants | Share |
|---|---|---|
| 0–14 | 19,028 | 8.8% |
| 15–24 | 39,259 | 18.1% |
| 25–34 | 62,148 | 28.7% |
| 35–44 | 45,638 | 21.0% |
| 45–54 | 31,050 | 14.3% |
| 55–64 | 13,111 | 6.0% |
| 65+ | 6,624 | 3.1% |

**Returning Romanians by age, 2021** (`migr_imm1ctz`, citizens): under 15: 18,473; 15–64: 125,609 (peak 30–34: 20,867); 65+: 5,431. Returns are slightly older than departures and include children born abroad.

### 2. The diaspora stock (confidence: medium)
- About **4 million** people born in Romania lived in 25 countries in 2020, at least 18% of the 22.6 million born and living in Romania, and nearly a quarter of those aged 20–59 (Erdelystat's comparison of national sources, reported by Economedia). Main destinations: Italy 979,000; Germany 813,000; Spain 579,000; United Kingdom 427,000; Hungary 210,000 (many of them ethnic Hungarians). Confidence medium (secondary source; the UN DESA migrant-stock table gives a similar order, not read).
- INS: population by domicile 22,142,153 against usual residents 19,261,714 in mid-2020 (Gheţău, HotNews). The 2.9 million gap is people registered at home who live abroad for a year or more, a lower bound of the diaspora (those who changed domicile abroad are no longer counted). Confidence high for the figures, medium for the reading.
- The 2021 census counted 19.05 million residents; 39 of 42 counties shrank over 2011–2021; Ilfov grew strongly. Confidence high.
- **Not every member of the diaspora sends money.** How many Romanian households receive remittances is not published in a source read; an educated guess from the flow (19 billion lei a year) and typical amounts (150–300 euro a month per sender) is **0.9 to 1.5 million senders**, and as many receiving households (12–20% of 7.7 million). Confidence low.

### 3. Remittances in the balance of payments, 2021 (confidence: high)
Eurostat `bop_rem6`, Romania, partner rest of the world, read through the API on 2026-10-26 (the BNR's balance of payments, BPM6):

| Item (BPM6) | Credit | Debit | Net |
|---|---|---|---|
| Secondary income: personal transfers (D752), million lei | **19,056** | 1,808 | **17,248** |
| same, million euro | 3,872.0 | 367.4 | 3,504.6 |
| Primary income: compensation of employees (D1), million euro | 3,823.0 | 175.1 | 3,647.9 |
| Personal remittances (R1, supplementary item), million lei | 27,666 | 2,152 | 25,514 |
| Workers' remittances and compensation of employees (R1WR), million lei | 37,871 | 2,670 | 35,201 |

- Personal transfers in million euro: 3,682 (2019), 3,422 (2020, −7%), 3,872 (2021), 4,426 (2022). They dipped in the pandemic, less than feared, and recovered. Confidence high.
- **Personal transfers** are current transfers between resident and non-resident households: what emigrants (resident abroad for a year or more) send home. They are **secondary income**. **Compensation of employees** is the pay of Romanian residents working abroad for less than a year (seasonal and short-term workers): **primary income**, and those workers are residents, part of the domestic population.
- The World Bank's "personal remittances received" for Romania in 2021 is about 9.1 billion US dollars (3.2% of GDP): it is R1WR, personal transfers plus compensation of employees. Confidence high.
- BNR (via Curs de guvernare): Germany, the United Kingdom, Italy and Spain sent nearly three quarters of 2020's remittances; Germany sent 1.27 billion euro in 2021 and the UK 1.18 billion. In 2020 remittances were 116% of net FDI. Confidence medium.

### 4. Internal migration, 2021 (confidence: high for totals)
INS TEMPO, read through its API on 2026-10-26.

`POP301A` (settlements of domicile, by age, 2021): **374,526** in total. Under 15: 84,717 (moving with parents); 15–19: 23,642; 20–24: 32,302; 25–29: 47,721; 30–34: 44,791; 35–39: 33,750; 40–44: 29,222; 45–49: 20,907; 50–54: 18,938; 55–59: 10,681; 60+: 27,855. The rate peaks at 25–34 (about 3.3% a year of that age group, against 2.0% overall).

`POP303B` (net balance of changes of domicile by county, 2021): gains in Ilfov +13,602, Cluj +4,198, Timiș +3,581, Bucharest +2,972, Brașov +2,247, Iași +1,870, Sibiu +1,498, Bihor +903, Arad +429, Constanța +254; the largest losses in Teleorman −1,876, Hunedoara −1,680, Bacău −1,673, Olt −1,629, Vaslui −1,617, Arges −1,409, Prahova −1,409. The positive balances sum to about 31,500: the net reallocation between counties is under 0.2% of the population a year. Confidence high.

- INS counts a change of domicile recorded on the identity card (police registers), between localities, including moves within a county; people who move without changing their card (students, many renters in Bucharest and Cluj) are missed, so real internal mobility is higher. Confidence medium.
- **Share crossing a county border:** not in the tables read. Educated guess 35–45% (moves within a county, from a town to its suburbs, are a large share: Ilfov's gain is mostly from Bucharest). Confidence low.

### 5. Immigration of foreigners (confidence: medium)
- 2021: 45,043 foreign immigrants (9,878 EU, 35,165 non-EU), under 0.25% of the population.
- Non-EU workers need a work permit within a yearly government quota: 25,000 for 2021, raised to 100,000 for 2022 (government decisions; from memory, confidence medium). From 2022 the main origins were Nepal, Sri Lanka, Turkey, India, Bangladesh (Eurostat 2023 via Romania Insider).

### 6. Behaviour (confidence: medium to low)
- **Emigration** rises with the wage gap to destinations and with unemployment at home, falls with age, and is much higher where family is already abroad (network effects; Mayda 2010; Beine, Docquier and Özden 2011). Elasticities of emigration rates to the destination/origin income ratio in cross-country studies are around 1 to 2. Confidence low for a number to use; medium for the direction.
- Romania after 2007: EU accession and the opening of labour markets (Spain and Italy first, Germany and the UK fully from 2014) drove the exodus; the 2008–2009 crisis in Spain and Italy brought a wave of returns and a fall in remittances.
- **Remittances** rise with the sender's income and fall with years abroad as ties weaken and families reunite abroad (Dustmann and Mestres 2010); they are steady or countercyclical with the home economy (insurance motive). Confidence medium.
- **Return migration** rises when home wages catch up and with age; returnees bring savings and often start firms. Romania's returns rose strongly in 2022–2023 (above). Confidence medium.

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Emigrants, 2021 | 216,861 (Romanian citizens 160,931) | Eurostat `migr_emi2`, `migr_emi1ctz` | high |
| Immigrants, 2021 | 194,642: Romanian 149,513; EU 9,878; non-EU 35,165 | Eurostat `migr_imm1ctz` | high |
| Emigrants by age, 2021 | 0–14 8.8%; 15–24 18.1%; 25–34 28.7%; 35–44 21.0%; 45–54 14.3%; 55–64 6.0%; 65+ 3.1% | Eurostat `migr_emi2` | high |
| Personal transfers, 2021 | credit 19,056m lei; debit 1,808m; net 17,248m | Eurostat `bop_rem6` (BNR) | high |
| Compensation of employees received, 2021 | €3,823m (about 18.8 billion lei) | Eurostat `bop_rem6` | high |
| Romanian-born living abroad, 2020 | about 4 million | Erdelystat via Economedia | medium |
| Domicile minus usual residents, mid-2020 | 22.14m − 19.26m = 2.88m | INS via HotNews | high |
| Households receiving remittances | 0.9–1.5 million (12–20%) | educated guess from the flow | low |
| Changes of domicile, 2021 | 374,526 (2.0% of population) | INS TEMPO `POP301A` | high |
| Net county gains, 2021 | Ilfov +13,602; Cluj +4,198; Timiș +3,581; Bucharest +2,972 | INS TEMPO `POP303B` | high |
| Share of domicile changes between counties | 35–45% | educated guess | low |
| Non-EU work-permit quota | 25,000 (2021); 100,000 (2022) | government decisions, from memory | medium |
| EUR/RON, 2021 average | 4.9215 | input-output table (trade-fx) | high |

## Implications for the game
- [Migration and remittances](../../02-design/economy/migration-remittances.md): the monthly emigration and return rates at the start are fitted so that the first year gives about 217,000 departures and 150,000 returns at 1:1 (net −22,000 with 45,000 foreign immigrants), with departures by age as in the table above.
- The starting remittance flow is **17.25 billion lei a year net** (BPM6 personal transfers), paid by the rest of the world to the households linked to a sending emigrant. In the roadmap's M7 gap (households receive about 650 billion lei against 720 in the input-output table) it closes about a quarter of the 70 billion. Compensation of employees (another 18.8 billion, primary income) belongs to residents working abroad for less than a year: a second candidate for the gap, outside the first version.
- Remittances are a current-account credit (secondary income) of about 1.5% of GDP: their steady flow supports the leu ([trade and FX](../../02-design/economy/trade-fx.md)).
- Internal migration is a slow, concentrated reallocation (under 0.2% of the population a year in net terms); the first version needs only a base rate by age and a destination rule that sends people toward Ilfov, Cluj, Timiș, Bucharest and Brașov.

## Sources
- [Eurostat `migr_emi2`: emigration by age and sex](https://ec.europa.eu/eurostat/databrowser/view/migr_emi2/default/table) — emigrants by single year of age; read through the dissemination API.
- [Eurostat `migr_imm1ctz`: immigration by citizenship](https://ec.europa.eu/eurostat/databrowser/view/migr_imm1ctz/default/table) — immigrants by citizenship and age; returns of nationals.
- [Eurostat `migr_emi1ctz`: emigration by citizenship](https://ec.europa.eu/eurostat/databrowser/view/migr_emi1ctz/default/table) — emigrants who are nationals.
- [Eurostat `bop_rem6`: personal remittances statistics](https://ec.europa.eu/eurostat/databrowser/view/bop_rem6/default/table) — personal transfers and compensation of employees in BPM6, million euro and lei.
- [INS TEMPO `POP301A`, `POP303B`](http://statistici.insse.ro:8077/tempo-online/) — changes of domicile by age; net balance by county.
- [OECD International Migration Outlook 2023, Romania](https://www.oecd.org/en/publications/international-migration-outlook-2023_b0f40584-en/full-report/component-42.html) — inflows of Romanian citizens to OECD countries, 2021.
- [Economedia: over 18% of those born in Romania lived abroad in 2020](https://economedia.ro/portal-de-statistica-in-2020-peste-18-dintre-cei-nascuti-in-romania-locuiau-in-strainatate-cele-mai-populare-cinci-tari-de-destinatie-sunt-italia-germania-spania-marea-britanie-si-ungaria.html) — diaspora stock by destination (Erdelystat).
- [e-juridic: INS on the population at 1 January 2022](https://e-juridic.manager.ro/articole/romanii-mai-putini-si-mai-batrani-datele-ins-29068.html) — INS's 2021 migration balance and sex shares.
- [HotNews, V. Ghețău on population by domicile and residence](https://hotnews.ro/?p=158152) — domicile 22.1m vs residents 19.3m; how internal migration is recorded.
- [Curs de guvernare: remittances above FDI in 2020](https://cursdeguvernare.ro/dependenta-de-munca-in-strainatate-remiterile-romanilor-peste-isd-in-anul-pandemiei.html) — BNR remittances 2019–2020, source countries.
- [Romania Insider: remittances at a 30-year high in 2021](https://romania-insider.com/romanians-personal-remittances-record-level-30-years) — World Bank total, about 9 billion dollars, 3.2% of GDP.
- [Romania Insider: migrants, most of them nationals returning](https://www.romania-insider.com/society/romania-migrants-destination-sept-2025) — returns and non-EU origins, 2022–2024.
- Mayda, A. M. (2010), "International migration: a panel data analysis of the determinants of bilateral flows", *Journal of Population Economics* 23 — wage gaps and emigration.
- Beine, M., Docquier, F., Özden, Ç. (2011), "Diasporas", *Journal of Development Economics* 95 — network effects.
- Dustmann, C., Mestres, J. (2010), "Remittances and temporary migration", *Journal of Development Economics* 92 — remittances fall with time abroad and family reunion.
