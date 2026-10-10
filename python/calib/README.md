# econ-calib

Calibration tooling (ADR-0009). Uses the `econ_py` Python bindings, built from
`crates/econ-py` with maturin automatically by `uv`.

```powershell
cd python\calib
uv sync          # builds econ_py (needs Rust) and installs numpy + SALib
uv run python calibration_spike.py
```

`calibration_spike.py` is Spike 5: runs-per-hour, Morris screening, one
history-matching wave and a 100-year quiet baseline. Targets in it are
illustrative placeholders, not Romanian data.
