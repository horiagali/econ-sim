---
id: research/return-immigration-drivers
title: What brings emigrants back and immigrants in, beyond wages
status: draft
owner: horia
depends_on: [research/migration-remittances-romania-2021]
updated: 2026-10-31
---

# What brings emigrants back and immigrants in, beyond wages

## Question
The first version of [migration](../../02-design/economy/migration-remittances.md) moves returns and foreign immigration with one thing only: net wages at home in euro against wages abroad. The owner asked (2026-10-31) whether a better economy and quality of life, and a narrower wage gap, should bring the diaspora home and draw immigrants, and expects that the quality of life people *expect* at home matters. What does the evidence say, and how strongly?

## Findings
- **Wages and the income gap are the best-established driver of flows.** Gravity models of OECD migration find that flows rise with the income gap between origin and destination (Ortega and Peri, 14 destinations, 1980–2005) and with relative incomes and the stock of earlier migrants (Lewer, Pacheco and Rossouw, 16 destinations, 1991–2000). The model already has this channel. Confidence high.
- **Public services push people out and keep them away.** Survey data for ten Central, Eastern and Southeastern European countries (OeNB Euro Survey 2018, Raggl 2022, OLS and instrumental variables) show that people dissatisfied with their country's public services are more likely to want to emigrate: social security, health, infrastructure and regional development matter for everyone; education, health and public safety more for parents of young children. Confidence medium (intentions, not moves; causal identification partial).
- **Romanians abroad name corruption, health care and bureaucracy as what keeps them away.** In the RePatriot / Open-I survey (1,810 Romanians abroad, 2019), 47% wanted to return (57% in 2017). Of those who did not, 83% named corruption and 72% political life as major reasons, and 40% the lack of jobs; respondents said better health care and education and less bureaucracy would make them consider returning. A 2026 diaspora study (MKOR) reports a competitive salary (44%) and better health care (46% in one group) as the main conditions for return. Confidence medium (opinion surveys, self-selected samples).
- **Family, jobs at home and plans matter for who returns.** A plan to start a business in Romania is the strongest predictor of return intentions among Romanians in Western Europe (Plopeanu and Homocianu, 2018 data); attachment to Romania, adaptation abroad and perceived discrimination follow. Jobs available in the home community, where the family lives, drive returns from the UK (2024). Occupation, education and ties to relatives predict permanent return from Spain and Italy (2009). Among returnees who opened firms, many find the business environment hostile and plan to leave again. Confidence medium.
- **Economic success abroad is not what decides it.** Among Polish, Romanian and Bulgarian labour migrants in the Netherlands, economic success or failure is unrelated to return intentions; strong ties to the host society lower them and transnational ties raise them (Snel, Faber and Engbersen 2015). Confidence medium.
- **Quality of life matters less for immigrants than for returnees.** Lewer, Pacheco and Rossouw find no significant effect of six non-economic quality-of-life indices on immigration flows into OECD countries once incomes and networks are in the model. Foreign immigrants to Romania (45,043 in 2021, mostly non-EU workers) come for jobs; the non-EU part is capped by work permits (a lever in M14). Confidence medium.
- **No study gives a usable elasticity for Romania.** Nothing found estimates how much returns move with health or education spending, or with corruption, in numbers a rule could take. Any strength the model uses is a guess to calibrate (M7).

## Implications for the model
- Keep wages in euro as the main channel (it is). Add **expected home quality** for the decisions of people who know Romania: returns of links and of the stock, and emigration (the other side of the same choice). Immigration takes it weakly or not at all.
- Make it **expected**, as the owner proposed: people abroad judge from what they hear over years, not from this month's budget, so the index should be slow (a moving memory of a few years).
- What the `World` can measure now: public services as real public spending per resident on health and education (the ministry budgets of the [lever system](../../02-design/game/lever-system.md)), and jobs as the region's unemployment. Corruption and governance have no variable before the institutions milestone; they enter the index when they exist.
- Jobs at home should raise returns, not only lower emigration: regional unemployment above its start makes returns less likely.
- Return incentives (Diaspora Start-Up) stay the planned lever (decided 2026-10-26); the evidence on the business plan motive supports it.

## Sources
- [OeNB, Focus on European Economic Integration Q3/22: the role of public services quality in shaping migration intentions in CESEE (Raggl)](https://www.oenb.at/en/Publications/Economics/Focus-on-European-Economic-Integration/2022/focus-on-european-economic-integration-q3-22/focus-on-european-economic-integration-q3-22-html.html) and [the paper's record](https://ideas.repec.Org/a/onb/oenbfi/y2022iq3-22b1.html)
- [Romania Insider: more than half of Romanians abroad would not come back (RePatriot / Open-I survey)](https://www.romania-insider.com/romanians-abroad-return-survey)
- [Romania Insider: most Romanian emigrants are homesick; what stops them from coming back?](https://www.romania-insider.com/news/most-romanian-emigrants-are-homesick-what-stops-them-from-coming-back)
- [MKOR study of the Romanian diaspora, 2026](https://mkor.eu/research/mkor-study-romanian-diaspora-2026/)
- [Plopeanu and Homocianu, Eastern Journal of European Studies (2020): return intentions of Romanians in Western Europe](https://doaj.org/article/215259e033e3426aaf015fc5333979ab)
- [Return intentions of Romanian migrants from the United Kingdom (Revista Română de Sociologie)](https://revistasociologieromaneasca.ro/sr/article/view/1803)
- [Ivan (2009), Revista Română de Sociologie: return of Romanians from Spain and Italy](https://revistasociologieromaneasca.ro/sr/article/view/2009_2_ivan)
- [Snel, Faber and Engbersen: to stay or return? Return intentions of Central and Eastern European labour migrants (CEEMR)](https://ceemr.uw.edu.pl/node/329)
- [Ortega and Peri: the causes and effects of international migrations, OECD 1980–2005 (NBER w14833)](https://www.nber.org/papers/w14833)
- [Lewer, Pacheco and Rossouw: do non-economic quality of life factors drive immigration? (IZA DP 4385)](https://repec.iza.org/dp4385.pdf)
- [Economedia: Diaspora Start-Up](https://economedia.ro/?p=74472)
