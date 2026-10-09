---
id: adr/0012-data-pipeline-and-licensing
title: "ADR-0012: Offline data pipeline and data licensing"
status: accepted
owner: horia
depends_on: [research/tech-stack-deep-research, adr/0003-people-representation, glossary]
updated: 2026-10-09
---

# ADR-0012: Offline data pipeline and data licensing

## Context
Romania is recreated from real data: census, national and sector accounts, input-output tables, financial accounts, budgets, BNR data, firm data. Reconciling these into one consistent opening balance sheet is a large work package. The data also have licences.

**Owner decision (2026-10-09): this is a personal, non-commercial project, not a product for sale.** The owner may use any data source whose terms allow personal, non-commercial use. Licences are therefore recorded for reference, not enforced by the build.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (sections "The Python side", "IPUMS cannot feed the shipped population" and "Data integration and licensing") found:
- **IPUMS-International bans commercial use and redistribution** of its microdata. That ruled it out for a game for sale; it does not rule out a personal, non-commercial project, provided the owner registers for access and never redistributes the raw microdata.
- Eurostat allows reuse with attribution, a statement of modifications and a disclaimer. Data on countries outside the EU, EFTA and candidate countries must be removed only before *commercial* reuse (this affects FIGARO's inter-country tables).
- EU-SILC and LFS microdata need institutional (research-entity) access, which an individual cannot get; their published tables are freely usable.
- Eurostat keeps only the latest version of each dataset, so raw downloads must be archived to rebuild a past starting state.
- INS's TEMPO-Online appears to be an undocumented single-page app (unverified).
- INS, BNR, data.gov.ro and Ministry of Finance terms could not be fetched.
- DVC hashes file contents and rebuilds only the stages whose parameters changed, so changing `sample_scale` rebuilds only the population stages.

## Options considered
**Pipeline tooling**
1. **Python: uv, Polars, DuckDB; just as the command surface; DVC for the DAG and cache.** Best data libraries; runs on Windows.
2. **Make.** Timestamp-based, awkward on Windows.
3. **Snakemake.** Capable; a second DSL for agents to learn.

**Population source** (see [ADR-0003](0003-people-representation.md))
1. **IPUMS-International 2011 Romania 10% sample as seed**, reweighted to 2021 census totals. Rich observed joint distributions; free non-commercial registration; raw microdata must stay private.
2. **Synthesise from public aggregates only** (2021 census cross-tabs, EU-SILC tables). No access restrictions; thinner joint distributions.

**Licence handling**
1. **Provenance tags kept for reference.** Every raw file records source, licence and retrieval date; nothing fails the build. Fits a personal, non-commercial project.
2. **Licence gate.** The build fails when a restricted source enters a shipped table's lineage. Needed only for a product that is distributed or sold.

**Schema**
1. **Glossary as the master schema, with codegen.** One source of truth for Rust, Python, TypeScript and docs.
2. **JSON Schema as master.** No column dtypes or units.
3. **Hand-kept parallel types.** Drift guaranteed.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1 in each group.** The licence-handling and population-source choices follow from the project being personal and non-commercial.

**Pipeline stages**
1. `fetch_*` per source
2. `normalise_*` to tidy Parquet with glossary codes
3. `reconcile` national, sector, financial and IO accounts (GRAS balancing), with explicit discrepancy accounts
4. `synth_population`, taking `sample_scale` as a parameter
5. `firms`: named firms plus size-class cohort firms per industry, with ownership vectors and power plants per generation unit ([ADR-0016](0016-firm-representation.md))
6. `assemble_scenario`
7. `validate` (pandera for tables, pydantic for config; note pandera checks only schema on lazy frames unless collected)
8. `report`: a diff of key aggregates against the previous build

`just` is the single command surface for the owner and agents. DVC is added once the graph passes about five stages. Raw data live in a private DVC remote or a gitignored local folder; small fixtures in Git LFS; the derived scenario is stored as a GitHub Release asset of the private repo. The MIT-licensed `eurostat` client is used for Eurostat. The TEMPO-Online fetcher is isolated, throttled and cached; the same series are taken from Eurostat or the ECB wherever possible.

**Raw archive with provenance tags.** Every raw download is stored immutably with URL, sha256 and a provenance tag: source, licence and retrieval date (for example `eurostat`, `ins`, `bnr`, `ogl-ro`, `ipums`). The tags are kept for reference and for the credits; **nothing fails the build because of a licence.** `report` lists the sources behind each scenario table. FIGARO may be used in full, including non-EU partners.

**Restricted microdata stay private.** IPUMS-International microdata may be used: the owner registers for free non-commercial access. Raw microdata are never published: they live only in the private DVC remote or a gitignored local folder (for example `data/raw/restricted/`), never in any public repository, release asset or shared file. Derived synthetic records are not raw microdata. EU-SILC and LFS microdata need institutional access, so their published tables are used instead.

**Population from the IPUMS seed.** The `synth_population` stage draws a stratified sample of households from the IPUMS-International 2011 Romania 10% sample at the configured `sample_scale`, reweights it with iterative proportional fitting to 2021 census county totals (INS and the Eurostat Census Hub), and imputes income, wealth and opinions from published survey tables (EU-SILC, HBS/ABF, opinion surveys). The seed source stays a swappable stage, so a public-aggregates-only seed remains possible. The synthetic population is validated against published aggregates (SILC deciles, poverty rates by household type, county employment).

**Glossary as master schema.** Each glossary entry records name, table, dtype, unit, kind (stock, flow or ratio), enum, range, source and `since_version`. A Jinja generator emits Rust enums and column structs, Arrow schemas, Polars and pandera models, TypeScript types and the docs page. CI reruns `just gen` and fails on any diff. Enum and flow codes are append-only.

**Config and mod files.** Hand-edited parameters in TOML; per-industry tables in CSV; matrices in Parquet. No YAML for player-edited files (older resolvers turn codes like `NO` into booleans). Rust deserialises with `deny_unknown_fields`. Mods are data-only ordered overrides, hashed into the save's content hash ([ADR-0011](0011-storage-saves-history.md)); no scripting mods.

## Consequences
- Easier: a starting state can be rebuilt from archived raw files; every table's sources are listed for the credits.
- Easier: the IPUMS seed gives observed joint distributions (household composition × education × occupation × housing) instead of assumptions.
- Easier: one schema change propagates to every language; agents cannot invent parallel names.
- Harder: income, wealth and opinions are still imputed from published tables (the census sample carries little or no income data; to verify), so the validation suite has to show the population behaves believably.
- Harder: the 2011 seed is ten years older than the 2021 targets; reweighting fixes margins but not new household types.
- Harder: reconciliation is a large work package; every residual must land in a visible discrepancy account ([ADR-0007](0007-money-and-ledger.md)).
- Revisit if the project ever becomes commercial (below).

### If the project ever becomes commercial
These choices would need revisiting:
- **IPUMS:** its terms forbid commercial use; switch the seed to public aggregates only, or get written approval from INS Romania.
- **GADM** map boundaries ([ADR-0013](0013-ui-stack.md)): no commercial use; switch to geoBoundaries.
- **Eurostat non-EU data:** remove non-EU, non-EFTA, non-candidate partner data (FIGARO) before commercial reuse.
- **Real firm names** ([ADR-0015](0015-distribution-and-licensing.md), [ADR-0016](0016-firm-representation.md)): trademark and defamation review, or fictional names.
- **Provenance tags** would become a licence gate that fails the build.

## Open questions / to verify
- [ ] Register for IPUMS-International access (free, non-commercial) and request the Romania 2011 sample.
- [ ] Which Romanian 2021 census tables exist at county level with the needed cross-tabs (not verified).
- [ ] Whether TEMPO-Online has a documented API.
- [ ] Which IPUMS Romania 2011 variables support the stratified draw and IPF to 2021 county totals.
- [ ] Spike 7 exit criterion: a scenario builds end to end with provenance tags listed in the report, and no raw restricted microdata are tracked by Git.
