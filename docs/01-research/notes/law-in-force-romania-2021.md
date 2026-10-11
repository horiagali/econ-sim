---
id: research/law-in-force-romania-2021
title: The law in force in Romania on 1 December 2021 — starting values of the levers
status: draft
owner: horia
depends_on: [research/overview]
updated: 2026-10-11
---

# The law in force in Romania on 1 December 2021 — starting values of the levers

## Question
What were the tax rates, contribution rates, benefit amounts and central-bank settings in force on the scenario's start date? They are the starting values of the levers ([ADR-0018](../../03-architecture/decisions/0018-scenario-and-commands.md): the law in force is data, one value per lever).

## Short answer
- Romania taxed labour through contributions more than through income tax: a flat 10% income tax, but 35% employee contributions, on almost the whole gross wage.
- Capital was taxed lightly: 16% on profits (1% or 3% of turnover for small firms), 5% on dividends.
- VAT: 19%, with 9% and 5% reduced rates.
- Benefits were small flat amounts tied to a reference indicator of 500 lei; the state pension was a point system worth 1,442 lei a point.
- The central bank had just started to raise its rate from the low of the pandemic.

Most values below are from memory of the legislation, checked against a source where one is cited. They are good enough to start mechanics on (the owner's steer of 2026-10-11: plausible over exact); each is one number in a data file and is to be checked against the legal text when the lever registry is filled.

## Findings

### Taxes on labour
| Lever | Value on 1 December 2021 | Confidence |
|---|---|---|
| Personal income tax | flat 10% of taxable income (gross wage less employee contributions less personal deduction) | high |
| Personal deduction | 510 lei a month at a gross wage up to 1,950 lei with no dependants, rising with dependants to 1,310 lei with four or more; shrinking to zero as the gross wage rises to 3,600 lei | medium |
| Pension contribution of the employee (CAS) | 25% of the gross wage | high |
| Health contribution of the employee (CASS) | 10% of the gross wage | high |
| Employer's labour insurance contribution (CAM) | 2.25% of the gross wage | high |
| Share of CAS going to the private pension funds (Pillar II) | 3.75 points of the 25 | high |
| Construction, and from 2022 agriculture and food: gross minimum 3,000 lei; no income tax, no CASS and CAS of 21.25% on wages up to 30,000 lei | in force for construction since 2019 | medium |
| IT workers | no income tax on wages from software development | medium |
| Minimum gross wage | 2,300 lei a month (2,550 from January 2022); 2,350 lei for jobs that need higher education | high (Eurostat `earn_mw_cur` for the first) |

Net of a gross wage of 5,000 lei with no dependants: 5,000 − 1,250 − 500 = 3,250 taxable; tax 325; net 2,925 (58.5%).

### Taxes on capital, consumption and property
| Lever | Value | Confidence |
|---|---|---|
| Corporate profit tax | 16% | high |
| Micro-enterprise regime | turnover up to 1 million euro: 1% of turnover with at least one employee, 3% without | high |
| Tax on dividends | 5% | high |
| Tax on interest and other investment income | 10% | high |
| Health contribution on non-wage income | 10% of a fixed base of twelve minimum wages, if such income exceeds that base | medium |
| VAT | 19% standard; 9% and 5% reduced ([vat](../../02-design/economy/vat.md)) | high for the rates |
| Excise duties | fuel, tobacco, alcohol, by quantity; levels not collected here | — |
| Tax on buildings | set by each municipality: 0.08% to 0.2% of the taxable value for homes, 0.2% to 1.3% for business premises | medium |
| Wealth tax | none | high |

### Social transfers
| Lever | Value | Confidence |
|---|---|---|
| Value of a pension point | 1,442 lei (frozen through 2021; 1,586 from January 2022) | high ([source](https://www.economica.net/pensii-cu-cat-cresc-2021-punct-de-pensie-2021-cum-va-creste-in-2022-proiect-de-oug_197096.html)) |
| Standard retirement age | men 65; women about 61 years and 9 months, rising to 63 by 2030 | medium |
| Minimum pension (social indemnity for pensioners) | 800 lei (1,000 from January 2022) | high |
| Child allowance | 214 lei a month from age 2 to 18; 427 lei under 2 or with a disability (243 and 486 from January 2022) | high ([source](https://hotnews.ro/?p=156253)) |
| Reference social indicator (ISR) | 500 lei | high |
| Unemployment benefit | 75% of the ISR (375 lei), plus 3% to 10% of the past average wage by years of contribution; for 6, 9 or 12 months | medium |
| Guaranteed minimum income | 142 lei a month for a single person, more per household member, means-tested | medium |
| Child-raising leave benefit | 85% of past net income, between 1,250 and 8,500 lei a month, until the child is 2 | medium |

### Money
| Lever | Value | Confidence |
|---|---|---|
| Policy rate of the National Bank | 1.75% (raised from 1.25% in October and November 2021; 2.00% from January 2022) | medium (from memory; not confirmed by the search) |
| Standing facilities | deposit 0.75%, lending 2.50% | low |
| Minimum reserve requirement | 8% on lei liabilities, 5% on foreign-currency liabilities | medium |
| Inflation target | 2.5% a year, plus or minus one point | high |
| Exchange-rate regime | managed float; about 4.95 lei per euro in December 2021 | high (annual average 4.9215, Eurostat `ert_bil_eur_a`) |

## Numbers worth keeping
The tables above are the numbers. Derived, for calibration checks:

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Net wage as a share of gross, no dependants, above 3,600 lei | 58.5% | computed from the rates | high |
| Employer's cost over gross wage | 102.25% | computed | high |
| Tax and contributions as a share of labour cost at the mean wage | 42.8% | computed | high |

## Implications for the game
- **For the lever registry and `law.json`** ([ADR-0018](../../03-architecture/decisions/0018-scenario-and-commands.md)): one entry per row above. Rows the alpha's levers cover: the income tax rate and deduction, the three contributions, corporate tax, the micro regime as one switch and rate, dividend and interest tax, the VAT rates, building tax as one national rate, the pension point and retirement ages, the minimum pension, child allowance, unemployment benefit as replacement of the reference indicator, the guaranteed minimum income, the policy rate, the reserve requirement, the inflation target, the minimum wage.
- **For [taxation](../../02-design/economy/taxation.md):** the personal deduction is what makes the Romanian system progressive at the bottom; a lever "brackets" must be able to express "flat rate with a shrinking deduction". The sector exemptions (construction, IT) are special cases of "reduced rates for an industry", which the catalogue of levers has for corporate tax only; proposed: the same per-industry override for the income tax and the contributions.
- **For [social transfers](../../02-design/economy/social-transfers.md):** most benefits are multiples of one reference indicator, so "index the ISR" is one lever that moves many benefits.
- **For the [informal economy](../../02-design/economy/informal-economy.md):** the wedge is 42.8% of labour cost and almost flat, so it is as high at the minimum wage as at the top. That is the standard explanation for envelope wages in Romania, and it is the reason the lever "lower wedge on low wages" is there.
- **For [pensions](../../02-design/society/population-pensions.md):** a starting pension divided by 1,442 lei is the person's number of pension points; that is the state to keep, so that changing the point's value changes every pension.

## Sources
- economica.net, on the pension point in 2021 and 2022: https://www.economica.net/pensii-cu-cat-cresc-2021-punct-de-pensie-2021-cum-va-creste-in-2022-proiect-de-oug_197096.html
- contabilul.manager.ro, on the pension point staying at 1,442 lei: https://contabilul.manager.ro/a/25727/se-amana-cresterea-pensiilor-valoarea-punctului-de-pensie-se-va-mentine-la-nivelul-de-1442-lei.html
- hotnews.ro, on child allowances and the pension point from January 2022: https://hotnews.ro/?p=156253
- Eurostat `earn_mw_cur` (minimum wage), `ert_bil_eur_a` (exchange rate), read on 2026-10-10 and 2026-10-11.
- Everything else: the author's memory of the Fiscal Code (Law 227/2015), the pension law (Law 263/2010) and the central bank's decisions; to be checked against the legal texts.
