---
id: economy/eu-funds
title: EU Funds & Absorption
status: draft
owner: horia
depends_on: [economy/eu-membership, economy/infrastructure, economy/state-capacity, economy/fiscal-policy]
research: []
updated: 2026-10-08
---

# EU Funds & Absorption

## Purpose
EU funds are a large source of investment money for Romania, but only if the state can **absorb** them: prepare projects, co-finance, procure, build and meet milestones. Absorption depends on [state capacity](state-capacity.md). This is a core strategic layer: money is available, the challenge is spending it well.

## Real-world basis
*(Verify programme names, envelopes and deadlines.)*
- **Cohesion policy** (ERDF, Cohesion Fund, ESF+, Just Transition Fund): multi-year envelopes, project-based, co-financing required, "n+3" decommitment rule (unspent money is lost after a deadline).
- **Common Agricultural Policy (CAP):** direct payments to farmers per hectare + rural development programmes.
- **Recovery and Resilience Facility (PNRR in Romania):** grants and loans paid against milestones and targets (reforms and investments), with a hard end date around 2026.
- Romania has historically had low-to-medium absorption rates compared with other member states.

## Model
### Envelopes
Each programme has: total envelope, period, eligible project types (infrastructure, energy, education, health, digital, social, agriculture, business support), co-financing rate, deadline rule. Data file per programme.

### Project pipeline (uses the [project system](infrastructure.md))
1. **Preparation:** the player (or ministries automatically, at a default pace) prepares projects for eligible types. Preparation takes months and administrative capacity.
2. **Approval:** probability depends on project quality (state capacity) and compliance status ([eu-membership](eu-membership.md)).
3. **Execution:** a normal project in the project system; spending is pre-financed by the government and reimbursed by the EU after verification, so cash-flow and co-financing hit the budget.
4. **Reimbursement:** EU pays its share after checks; irregularities found (more likely with high corruption) → **financial corrections** (money withheld or clawed back).
5. **Decommitment:** unspent money past the deadline is lost.

### CAP
- Direct payments per hectare to households and firms with farm land → farm income ([households](households.md), `agri_*` industries).
- Rural development grants via the project pipeline.

### Recovery facility (milestones)
- Payments are tied to milestones: reforms (e.g. pension, tax administration) and investments. Missing milestones cuts the payment.
- Milestones are represented as lever conditions ("tax administration digitalised", "coal plants closed by X") checked by the game.

### Absorption rate
```math
\text{absorption}_t = \frac{\text{funds reimbursed (cumulative)}}{\text{funds available (cumulative)}}
```
Driven by state capacity (admin quality, procurement speed), co-financing room in the budget, construction capacity, compliance.

## Accounting
Funds are a capital transfer (investment grants) or current transfer (CAP, ESF) from the EU (RoW) to the government or directly to beneficiaries; co-financing is government spending; corrections are reverse transfers.

## Player levers
- Prioritise programmes and project types; prepare projects (choose region and type).
- Co-financing budget allocation.
- Invest in absorption capacity (EU-funds management agencies; see [state capacity](state-capacity.md)).
- Meet or ignore recovery-facility milestones.

## Outputs
Envelope vs committed vs paid by programme, absorption rate, money at risk of decommitment, corrections, contribution to GDP and investment.

## Acceptance tests
- [ ] With low state capacity, absorption stays low and money is lost to decommitment.
- [ ] Raising administrative capacity raises absorption over 1–3 years.
- [ ] A suspended commitment (fiscal-rule breach) reduces new fund flows.
- [ ] EU-funded projects raise construction output and public capital like other projects, but cost the budget only the co-financing share (after reimbursement).

## Open questions
- [ ] Do ministries prepare projects automatically by default (proposed, to avoid micromanagement), with the player able to steer?
- [ ] Model the next EU budget period (after 2027) with an uncertain envelope?
