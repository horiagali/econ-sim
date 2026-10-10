# Data pipeline (ADR-0012) — Spike 7 slice

Builds the starting scenario from public data. Standard library only for now
(Polars/DuckDB come in when tables get big).

| Step | Script | Output |
|---|---|---|
| 1. Fetch | `fetch_eurostat.py` — downloads datasets listed in `sources.toml` via the Eurostat API (JSON-stat) | `data/raw/<id>.json` + `data/raw/<id>.provenance.json` (url, retrieval time, sha256, licence) |
| 2. Synthesise | `synth_counties.py` — draws a seed sample and fits it to county marginals with IPF | `data/derived/population_<counties>.csv` |
| Codegen | `scripts/codegen_glossary.py` (repo root) — glossary → `crates/econ-types/src/glossary.rs` | checked in CI |

`data/` is git-ignored. `dvc.yaml` describes the same stages for DVC
(`uv tool install dvc`, then `dvc repro`), so raw downloads can be cached
and versioned outside git.

```powershell
python python\pipeline\fetch_eurostat.py            # needs internet
python python\pipeline\synth_counties.py --fixture  # works offline
python -m unittest discover -s python\pipeline\tests
```

Placeholder marginals in `fixtures/` are **illustrative, not Romanian data**.
Real marginals come from the Census 2021 tables (INS) and Eurostat; the base
sample from the IPUMS 2011 Romania 10% sample (private, never committed).
