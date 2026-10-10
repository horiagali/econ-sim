---
id: architecture/spike-10-daily-clock
title: "Spike results: 10 (daily clock)"
status: draft
owner: horia
depends_on: [adr/0017-time-base, architecture/spikes-0-4-results]
updated: 2026-10-10
---

# Spike results: 10 — the daily clock

Built 2026-10-10 on the owner's Windows laptop, Rust 1.97.0, release build, one thread. It answers the question [ADR-0017](../decisions/0017-time-base.md) left open: what does a simulated year cost when one tick is one day?

## What was built
The scale world of Spike 4 ([results](spikes-0-4-results.md)) got a second way of stepping, `ScaleWorld::step_day`, next to its monthly `step`. Same population, same rules, each part at its own period:

| Period | Part |
|---|---|
| Staggered (each household on its own day, 1 to 28) | separations and job search of its members, the household's spending with VAT |
| Daily | the goods market (demand to gross output through the 90-industry input-output system), the ledger posting and its checks, including the comparison of the ledger with every household's deposits |
| Month end | wages for everyone employed through the clearing accounts, the statistics cube |

`econ-cli bench-days` (`just bench-days`) runs three years both ways and compares.

## Result ✅

| Scale | Persons | Year, monthly tick | Year, daily tick | Ordinary day | Month-end day | 50 years, daily |
|---|---|---|---|---|---|---|
| 1:1000 | 19,000 | 14 ms | 47 ms | 0.12 ms | 0.35 ms | 2 s |
| 1:100 | 190,000 | 131 ms | 210 ms | 0.50 ms | 2.9 ms | 11 s |
| 1:10 | 1,900,000 | 1.42 s | 2.43 s | 5.7 ms | 34 ms | 122 s |

- The budget of ADR-0017 was about 1 ms for an ordinary day and half a second for a year at 1:100. Measured: 0.5 ms and 0.21 s.
- A year of daily ticks costs 1.6 times a year of monthly ticks at 1:100, not 30 times. Staggering is why: the per-person work is done once a month either way.
- The fixed daily cost (the input-output solve and the ledger checks) shows at 1:1000, where the daily year is 3.4 times the monthly one. It does not grow with the population.
- The two ways of stepping give the same economy: wage bill, consumption and employment of the second year agree within 3% at 1:1000 (test `daily_clock_agrees_with_the_monthly_tick_in_aggregate`), and the third year's wage bill within 1% at every scale.

## What it taught
- **Rounding to whole persons needs a carry when groups are small.** The first version hired `floor(rate × job seekers)` per region and skill each day. A day's cohort has a handful of seekers per cell, so the floor was zero and nobody was hired: employment fell by a fifth. Carrying the fraction owed to the next day fixed it. This is the same device as the carry of the population generator ([population-attributes](../../02-design/society/population-attributes.md)). Every staggered rule that selects whole persons from a rate (hires, separations by alignment, births, deaths if they are ever staggered) must carry its remainder per cell.
- **Income and spending are no longer in the same step.** Wages arrive at month end; a household spends on its own day out of what it has. The spending rule therefore reads last pay day's income. Rules written for the monthly tick as "this tick's income" need a look when they are reworded.
- **Per-person loops belong to the month end or to the stagger.** The ordinary day touches a 28th of the persons. A daily phase that loops over everyone would cost about as much as the month-end day (2.9 ms at 1:100) every day, which is 1 s a year: still affordable once, not for every mechanic.

## Limits
- The scale world has a small part of the mechanics of v1 (no firms as agents, no banks, no prices, no demographics). The real cost per year will be several times higher; the ratio between daily and monthly stepping is what carries over.
- Daily market phases (prices, exchange rate, yields) are represented only by the input-output solve.
- Weekends are ignored, as ADR-0017 decides.
