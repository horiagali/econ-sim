#!/usr/bin/env python3
"""Differential test: Rust SIM mode vs the independent Python reference.

Passes only if every aggregate matches **exactly in bani** for 200 ticks, and
if deliberately mutated Rust builds are caught (proving the test has teeth).
Run via `just diff-sim` (builds econ-cli first).
"""
from __future__ import annotations

import csv
import io
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from sim_reference import run  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
TICKS = 200
COLS = ["Y", "T", "YD", "C", "G", "H", "GOV_DEFICIT"]


def binary() -> Path:
    for name in ("econ-cli.exe", "econ-cli"):
        p = ROOT / "target" / "debug" / name
        if p.exists():
            return p
    sys.exit("econ-cli not built: run `cargo build -p econ-cli` (or `just diff-sim`)")


def rust_rows(*extra: str) -> list[dict[str, int]]:
    out = subprocess.run([str(binary()), "sim", "--ticks", str(TICKS), *extra],
                         capture_output=True, text=True, check=False)
    rows = list(csv.DictReader(io.StringIO(out.stdout)))
    return [{k: int(v) for k, v in r.items() if k != "state_hash"} for r in rows]


def compare(rust: list[dict[str, int]], ref: list[dict[str, int]]) -> list[str]:
    errs = []
    if len(rust) != len(ref):
        errs.append(f"row count {len(rust)} vs {len(ref)}")
    for a, b in zip(rust, ref):
        for c in COLS:
            if a[c] != b[c]:
                errs.append(f"tick {a['tick']} {c}: rust={a[c]} ref={b[c]} diff={a[c] - b[c]}")
    return errs


def main() -> int:
    ref = run(TICKS)
    errs = compare(rust_rows(), ref)
    if errs:
        print("DIFFERENTIAL TEST FAILED (Rust SIM != Python reference):")
        print("\n".join(errs[:20]))
        return 1
    print(f"OK — Rust SIM matches the Python reference exactly in bani for {TICKS} ticks.")
    # The test must catch deliberate bugs.
    for mutation in ("tax-sign", "consume-gross"):
        if not compare(rust_rows("--mutate", mutation), ref):
            print(f"MUTATION NOT CAUGHT: --mutate {mutation} still matched the reference")
            return 1
        print(f"OK — mutation '{mutation}' is caught by the differential test.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
