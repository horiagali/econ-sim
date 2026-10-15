# Simulation changelog

Every change to a golden hash or snapshot needs an entry here with the reason
(ADR-0010). Newest first.

## 2026-10-15 — Population stage K: first golden hashes
- Added `tests/golden/popgen_ro_census2021_inactivity_enrolment.hashes` (AC-POPK-07): the inactivity and enrolment hash of the Census 2021 fixture population at 1:1000, 1:200, 1:100 and 1:10 (seed 42, default parameters, stages A to C run). Reason: initial baseline for the stage. The values are those of the spec, reproduced on the day with the Python reference (`python/reference/popgen_reference.py`); the Rust stage does not exist yet. The stage reads only stages A to C and its own margin file, so the hashes do not move with the firm-unit fixture or stages D to J.
- **Unchanged:** every other golden. The stage changes nothing before it (AC-POPK-02).

## 2026-10-13 — Scale world: VAT rates of 1 December 2021
- **Re-golden (owner approved, 2026-10-13):** the scale world's stand-in VAT schedule moves from 21% standard and 11% reduced (the 2025 rates) to Romania's rates on the start date, 19% standard and 9% and 5% reduced (spec `economy/vat`, AC-VAT-06; the spike's basket has no second-reduced group, so 5% is unused). Less VAT is carved out of household spending, so VAT, deposits and the state hash change; employment and the wage bill do not.
  - `golden_12_ticks_at_1_in_1000` (`crates/econ-core/src/scale_spike.rs`): hash 0x8557_a49a_52c1_e86a → 0xf540_af18_bcd5_dbab; consumption in tick 12 6,207,483,617,000 → 6,207,483,754,000 bani; VAT in tick 12 823,228,305,000 → 740,807,571,000; VAT over 12 ticks 9,769,553,421,000 → 8,791,436,167,000.
  - `golden_romania_12_ticks_at_1_in_1000` (`crates/econ-io/src/popgen.rs`): hash 0xc050_2b54_1f4a_bd0b → 0xdf92_e313_fa51_cb01; VAT in tick 12 979,111,507,848 → 881,083,907,188; employed and unemployed unchanged.
- **Unchanged:** `tests/golden/sim_200.hashes` and every population golden.

## 2026-10-11 — Population stages F to J: first golden hashes
- Added `tests/golden/popgen_ro_census2021_wages.hashes` (AC-POPW-07), `popgen_ro_census2021_pensions.hashes` (AC-POPP-07), `popgen_ro_census2021_wealth.hashes` (AC-POPH2-07), `popgen_ro_census2021_incomes.hashes` (AC-POPI-07) and `popgen_ro_census2021_employers.hashes` (AC-POPE-06): the wage, pension, wealth, income and employer hash of the Census 2021 fixture population at 1:1000, 1:200, 1:100 and 1:10 (seed 42, default parameters, every earlier stage run). Reason: initial baseline for the five stages. The values are those of the five specs, reproduced on the day with the Python reference (`python/reference/popgen_reference.py`); the Rust stages do not exist yet.
- **Unchanged:** every other golden. The stages change nothing before them (AC-POPW-02, AC-POPP-02, AC-POPH2-02, AC-POPI-02, AC-POPE-02).

## 2026-10-10 — Population stages C, D and E: first golden hashes
- Added `tests/golden/popgen_ro_census2021_attributes.hashes` (AC-POPA-06), `popgen_ro_census2021_jobs.hashes` (AC-POPJ-06) and `popgen_ro_census2021_housing.hashes` (AC-POPH-06): the attribute, jobs and housing hash of the Census 2021 fixture population at 1:1000, 1:200, 1:100 and 1:10 (seed 42, default parameters). Reason: initial baseline for the three stages. The values are those of the three specs, reproduced on the day with the Python reference (`python/reference/popgen_reference.py`); the Rust stages do not exist yet.
- **Unchanged:** `tests/golden/popgen_ro_census2021.hashes` and every other golden. The stages do not change the population (AC-POPA-02, AC-POPJ-02, AC-POPH-02).

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
