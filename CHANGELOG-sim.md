# Simulation changelog

Every change to a golden hash or snapshot needs an entry here with the reason
(ADR-0010). Newest first.

## 2026-10-10 — Scale world on the Romanian population: new golden
- **New golden:** `golden_romania_12_ticks_at_1_in_1000` in `crates/econ-io/src/popgen.rs` pins the state hash, employment, unemployment and VAT after 12 ticks of the scale world built by `ScaleWorld::from_population` on the Census 2021 fixture (1:1000, seed 42). Reason: initial baseline for the first run of the simulation on generated households instead of invented ones.
- **Unchanged:** `golden_12_ticks_at_1_in_1000` (the invented population) and every other golden. `ScaleWorld::generate` draws the same numbers in the same order; the input-output matrix and consumption shares only moved into a helper.

## 2026-10-10 — Population generator: first golden hashes
- Added `tests/golden/popgen_ro_census2021.hashes`: state hash of the starting population generated from the Census 2021 margin fixture at 1:1000, 1:100 and 1:10 (seed 42, default parameters). Reason: initial baseline for AC-POP-06. The values come from the Python reference (`python/reference/popgen_reference.py`), written before the Rust generator.

## 2026-10-10 — Scale world: gamma mixer and VAT (owner decision)
Both change scale-world results on purpose. `tests/golden/sim_200.hashes` is **unchanged** (the SIM model uses neither the fast hash nor the scale world).
- **Fast-hash mixer:** `fast_u64` now ends with `mix64(st ^ entity·γ)` instead of `mix64(st ^ entity)`. Reason: the old mixer fails PractRand `BRank` at 16 GB when only the entity varies, and shows a rank deficiency of 32–35 in the first 2^20 consecutive entities (ADR-0006 Amendment 1, revised). `FAST_KAT` changed from 744824335102330087 to 16733015585576584871 (also in `python/reference/rng_quality.py`).
- **VAT in the scale world:** household spending now has VAT carved out per household and VAT category and posted to government through the ledger (`tax.vat`, 3002). Reason: first real mechanic wired into the scale world. Rounding can leave one ban per purchase unspent, so deposits and consumption change slightly; final demand for the input-output solve is now at basic prices (net of VAT).
- **New golden:** `golden_12_ticks_at_1_in_1000` in `crates/econ-core/src/scale_spike.rs` pins the state hash and aggregates after 12 ticks (1:1000, seed 42). Reason: the scale world had no golden before.
- Not a result change: `Bani::mul_ratio` has a 64-bit fast path, checked equal to the `i128` path by a property test. Save format 2 → 3 (`gov_vat`).

## 2026-10-09 — Spike 2: first golden run
- Added `tests/golden/sim_200.hashes`: per-tick state hashes of the Godley–Lavoie
  SIM model (200 ticks, default parameters). Reason: initial baseline.
