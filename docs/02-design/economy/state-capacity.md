---
id: economy/state-capacity
title: State Capacity & Corruption
status: draft
owner: horia
depends_on: [economy/fiscal-policy, economy/infrastructure]
research: []
updated: 2026-10-08
---

# State Capacity & Corruption

## Purpose
How well the state turns money into results. The same budget builds more road, absorbs more EU funds, collects more tax and delivers better hospitals when administrative capacity is high and corruption is low. This is a slow-moving, high-impact layer, and the bridge to the politics layer later.

## Real-world basis
- Public investment efficiency varies widely across countries; a substantial share of potential value is lost to inefficiency in many countries (IMF public investment management assessments).
- Corruption raises procurement costs, distorts project choice, lowers FDI and tax compliance, and erodes trust.
- Administrative capacity depends on civil-service pay and merit, digitalisation, management systems and stability of staff.
- Romania's indicators (corruption perception, government effectiveness) are below the EU average but improved since EU accession *(verify with Worldwide Governance Indicators, Transparency International)*.

## State
| code_name | Meaning | Range |
|---|---|---|
| `admin_capacity[m]` | Administrative capacity per ministry/agency (incl. tax administration, EU-funds agencies) | 0–1 |
| `corruption` | Prevalence of corruption in public procurement and services | 0–1 |
| `rule_of_law` | Judicial effectiveness and contract enforcement | 0–1 |
| `digitalisation` | Digital public services and e-government | 0–1 |
| `policy_stability` | Inverse of how often key rules (taxes) change | 0–1 |

## Effects
| Area | How capacity and corruption act |
|---|---|
| Public investment | Effective capital added = spending × efficiency; efficiency = f(capacity, corruption); delays and cost overruns rise as capacity falls ([infrastructure](infrastructure.md)) |
| Corruption leakage | A share of procurement spending is diverted: booked as transfer to connected households (top income classes) instead of producing output |
| Public services | Quality per leu spent in education, health and policing scales with capacity |
| Tax collection | Enforcement effectiveness ([informal economy](informal-economy.md)) |
| EU funds | Project quality, approval and absorption ([eu-funds](eu-funds.md)); irregularities → financial corrections |
| FDI and investment | Institutions index in [foreign-ownership](foreign-ownership.md) and risk premium in [investment](investment-capital.md) |
| SOEs | Management quality and efficiency gap ([state-enterprises](state-enterprises.md)) |
| Trust | Feeds tax compliance and (later) politics |

## Update rule (slow, months to years)
```math
\text{admin\_capacity}_{m,t+1} = \text{admin\_capacity}_{m,t} + \lambda \Big(f(\text{relative pay}_m, \text{merit reform}, \text{digitalisation}, \text{training budget}_m) - \text{admin\_capacity}_{m,t}\Big) - \text{shock}(\text{mass layoffs, purges})
```
```math
\text{corruption}_{t+1} = \text{corruption}_t + \lambda_c \Big(g(\text{anti-corruption funding}, \text{rule\_of\_law}, \text{digitalisation}, \text{procurement transparency}, \text{relative public pay}) - \text{corruption}_t\Big)
```
Improvements are slow; deterioration (e.g. cutting anti-corruption funding) is faster.

## Player levers
Civil service pay and merit-based hiring reform, training budgets, digitalisation of public services (a project), anti-corruption agency and justice funding, procurement transparency (open contracting), policy stability (consequence of how often the player changes tax rules), EU-funds management agency staffing.

## Outputs
Capacity per ministry, corruption index, estimated money lost to inefficiency and corruption, rule of law, public-investment efficiency, international-style governance scores.

## Acceptance tests
- [ ] With the same budget, a high-capacity state adds more public capital than a low-capacity one.
- [ ] Raising anti-corruption funding lowers corruption over several years, and lowers leakage in procurement.
- [ ] Changing tax rates every few months lowers policy stability and FDI.
- [ ] Corruption leakage is fully booked as transfers (no money disappears).

## Open questions
- [ ] Should corruption scandals appear as events (flavour) when leakage is high, or stay purely statistical in v1?
