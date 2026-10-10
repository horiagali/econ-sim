# Data pipeline (ADR-0012) — Spike 7 slice

Builds the starting scenario from public data. Standard library only for now
(Polars/DuckDB come in when tables get big).

| Step | Script | Output |
|---|---|---|
| 1. Fetch | `fetch_eurostat.py` — downloads datasets listed in `sources.toml` via the Eurostat API (JSON-stat) | `data/raw/<id>.json` + `data/raw/<id>.provenance.json` (url, retrieval time, sha256, licence) |
| 1b. Normalise census | `normalise_census.py` — turns the two Census 2021 downloads into the population generator's margin file, filling suppressed cells | `data/derived/census2021_margins_ro.json` (a copy is committed as `fixtures/census2021_margins_ro.json`) |
| 2. Synthesise population | `econ-cli synth-population --margins FILE --sample-scale N --seed S --out DIR` (`just synth-population 100`) — the population generator, Rust crate `econ-popgen` | `households.arrow`, `persons.arrow`, `fit_report.json` in `DIR` (default `data/population/`) |
| Spike 7 slice | `synth_counties.py` — draws a seed sample and fits it to county marginals with IPF | `data/derived/population_<counties>.csv` |
| Codegen | `scripts/codegen_glossary.py` (repo root) — glossary → `crates/econ-types/src/glossary.rs` | checked in CI |

`data/` is git-ignored. `dvc.yaml` describes the same stages for DVC
(`uv tool install dvc`, then `dvc repro` from this folder; offline, skip the download with `dvc repro -s normalise_census synth_population`), so raw downloads can be cached
and versioned outside git.

```powershell
python python\pipeline\fetch_eurostat.py            # needs internet
python python\pipeline\synth_counties.py --fixture  # works offline
python -m unittest discover -s python\pipeline\tests
```

`fixtures/marginals_illustrative.json` is **illustrative, not Romanian data**. `fixtures/census2021_margins_ro.json` is real: Eurostat Census 2021 round (tables `cens_21cobhs_r3` and `cens_21hhcs_r3`, reuse allowed with attribution), all 42 counties. The population generator that reads it is specified in `docs/02-design/society/population-generator.md` and implemented in `crates/econ-popgen`; its Python reference is `python/reference/popgen_reference.py`, and the two are compared record for record by AC-POP-07.
Real marginals come from the Census 2021 tables (INS) and Eurostat; the base
sample from the IPUMS 2011 Romania 10% sample (private, never committed).
