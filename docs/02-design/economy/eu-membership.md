---
id: economy/eu-membership
title: EU Membership — Single Market, Schengen, Free Movement, Fiscal Rules
status: draft
owner: horia
depends_on: [economy/trade-fx, economy/fiscal-policy, economy/demographics]
research: []
updated: 2026-10-08
---

# EU Membership — Single Market, Schengen, Free Movement, Fiscal Rules

## Purpose
Romania is an EU member outside the euro area. Membership shapes what the head of state can and cannot do: no tariffs inside the EU, a common external tariff set in Brussels, free movement of workers and capital, state-aid rules, minimum VAT rules, a contribution to the EU budget, access to EU funds ([eu-funds](eu-funds.md)) and the EU fiscal rules. Breaking rules has consequences (infringement procedures, fines, suspended funds), so the player can push limits at a cost.

## Real-world basis
*(All to verify in research; rules change.)*
- **Customs union:** no tariffs or quotas on intra-EU trade; a **common external tariff** (CET) on non-EU imports, set by the EU. Trade defence (anti-dumping) is EU-level.
- **Free movement** of goods, services, capital and people: no capital controls; EU citizens can live and work anywhere in the EU, which is what enables Romania's large diaspora.
- **State aid:** subsidies to firms need to fit EU rules (block exemptions, de minimis, approval by the Commission).
- **VAT directive:** minimum standard rate 15%; limited reduced rates.
- **Fiscal rules:** deficit reference 3% of GDP, debt 60%, enforced via the excessive deficit procedure (EDP) with a net-expenditure path; non-compliance can lead to recommendations, then suspension of commitments of some EU funds. Romania has been under an EDP in recent years *(verify current status)*.
- **EU budget:** members pay contributions (mostly GNI-based plus VAT-based and customs revenue).
- **EU ETS:** carbon price for power and heavy industry, set by the EU market ([environment](environment.md)).
- **Euro:** Romania is legally committed to adopt the euro eventually but has no date. Joining and leaving are specified in [euro](euro.md).
- **Schengen:** Romania became a full Schengen member (land borders included) at the start of 2025 *(verify)*; internal border checks were removed, which cut waiting times for trucks at borders with Hungary and Bulgaria.

## Model
The EU is represented as a **partner bloc** inside the rest of the world ([trade-fx](trade-fx.md)) plus a set of **rules** and **flows**.

### Rules (constraints on levers)
| Rule | Effect on player levers |
|---|---|
| Customs union | Tariff lever is **locked to the CET** for non-EU goods and to 0 for EU goods while a member |
| Free movement of capital | Capital-controls lever disabled |
| Free movement of people | Immigration quota applies to **non-EU** immigrants only; EU citizens move freely (both ways) |
| State aid | Subsidies above de-minimis thresholds trigger a **state-aid review**; non-compliant aid → order to recover the aid + compliance risk |
| VAT directive | Standard VAT can't go below 15%; reduced rates only on allowed categories |
| Fiscal rules | Deficit and debt tracked against reference values; EDP status and required adjustment path shown |

### Compliance and infringement
- A **compliance score** per rule area. Breaking a rule (e.g. illegal subsidy, deficit far above the path) starts a procedure that escalates over months: warning → formal procedure → fines and/or **suspension of EU fund commitments**.
- Escalation is deterministic and visible in the UI with timelines, so the player can choose to break rules knowingly.

### Flows (booked in accounting, counterparty = EU institutions in the RoW sector)
- Contribution to the EU budget (monthly, GNI-based + VAT-based + share of customs duties collected).
- EU funds received ([eu-funds](eu-funds.md)).
- Customs duties collected on non-EU imports, mostly passed to the EU.

### Schengen
State: `schengen_member` (true at start), `border_wait[route]` (hours for freight at internal borders), `external_border_spend`.

| | In Schengen | Out of Schengen |
|---|---|---|
| Internal borders (Hungary, Bulgaria) | No checks; low freight wait | Checks; freight wait time rises (hours) |
| Trade cost with EU | Lower | Higher → applies as an extra cost on road freight and a small iceberg cost on EU trade ([trade-fx](trade-fx.md)) |
| Tourism | Easier travel → more EU tourists | Fewer |
| External border (Ukraine, Moldova, Serbia, Black Sea) | Romania must guard the EU external border: border police spending, standards checked by EU (compliance) | National standards only |
| Irregular migration pressure | Shared EU system | National |

Levers: **leave Schengen** (reintroduce border checks; EU relations cost), **rejoin** (requires meeting external-border standards for a period and an EU decision), external border budget.

### Free movement in practice
- Emigration and return flows to EU countries face no barriers ([demographics](demographics.md)); the wage gap with EU destinations drives them.
- EU firms can invest freely ([foreign-ownership](foreign-ownership.md)).

## Player levers
- Comply or deliberately break specific rules (via normal levers; the game flags a breach).
- Request EU flexibility (e.g. a longer fiscal adjustment path) — granted based on reforms and investment plans (simple rule).
- Leave or rejoin Schengen (above).
- Join or leave the euro ([euro](euro.md)).
- *Later:* leaving the EU entirely.

## Outputs
Compliance status per rule, EDP status and required path, net EU balance (funds received − contribution), infringement timeline.

## Acceptance tests
- [ ] Leaving Schengen raises freight waiting times and road-freight costs on EU trade, and lowers EU tourism.
- [ ] While in the EU, the player cannot set a tariff on imports from EU partners.
- [ ] A deficit persistently above the required path escalates the procedure and eventually suspends part of the EU fund commitments.
- [ ] An illegal subsidy is flagged and, if not withdrawn, leads to recovery orders.
- [ ] EU contribution and funds received net to the "net EU balance" shown in the budget view.

## Open questions
- [ ] Should "leave the EU" be a v1 lever too? (Euro and Schengen are in v1.)
- [ ] How detailed should state-aid review be: simple threshold rule (proposed) or per-scheme approval?
