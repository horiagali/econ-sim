#!/usr/bin/env python3
"""Fail if clippy ACCEPTS crates/lint-canary (it must reject it).

The canary deliberately uses HashMap and f64::exp. If clippy passes, the
determinism deny-lists in clippy.toml are not active (ADR-0006).
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
canary = ROOT / "crates" / "lint-canary"
r = subprocess.run(["cargo", "clippy", "-q"], cwd=canary, capture_output=True, text=True)
out = r.stdout + r.stderr
needed = ["disallowed type `std::collections::HashMap`", "disallowed method `f64::exp`"]
missing = [n for n in needed if n not in out]
if r.returncode == 0 or missing:
    print("LINT CANARY FAILED: clippy did not reject the canary as expected.")
    print("missing diagnostics:", missing)
    print(out[-2000:])
    sys.exit(1)
print("OK — clippy rejects HashMap and f64::exp (determinism deny-lists active).")
