"""Independent reference implementation of Godley & Lavoie model SIM.

Source: Godley & Lavoie (2007), *Monetary Economics*, ch. 3. Written from the
textbook equations, not from the Rust code (ADR-0010 differential testing).

Shared conventions with the Rust core (documented in docs/03-architecture/
spikes/spike-2-ledger-sim.md):
  * money is an integer number of bani;
  * Y is solved from the closed form and rounded half away from zero;
  * T = round(theta * Y), half away from zero;
  * C = Y - G (goods market clears exactly), so the behavioural
    C = a1*YD + a2*H_prev holds up to one bano of rounding.
"""
from __future__ import annotations

import math
from dataclasses import dataclass


def round_half_away(x: float) -> int:
    """Round to the nearest integer, ties away from zero (IEEE-exact)."""
    if not math.isfinite(x):
        raise ValueError(f"non-finite {x}")
    t = math.trunc(x)
    frac = x - t  # exact for |x| < 2**52
    if abs(frac) >= 0.5:
        t += 1 if x > 0 else -1
    return int(t)


@dataclass(frozen=True)
class Params:
    g: int = 20_000_000 * 100  # 20 million lei per tick, in bani
    theta: float = 0.2
    alpha1: float = 0.6
    alpha2: float = 0.4


def run(ticks: int, p: Params = Params()) -> list[dict[str, int]]:
    rows = []
    h = 0  # household money stock, bani
    gov = 0  # government money liability (negative), bani
    for tick in range(ticks):
        y = round_half_away((float(p.g) + p.alpha2 * float(h)) / (1.0 - p.alpha1 * (1.0 - p.theta)))
        t = round_half_away(float(y) * p.theta)
        yd = y - t
        c = y - p.g
        # Accounting: households receive wages Y, pay taxes T and buy C.
        h = h + y - t - c
        gov = gov - p.g + t
        assert h + gov == 0, "money stock must equal the government's liability"
        assert abs(c - (p.alpha1 * yd + p.alpha2 * (h - yd + c))) <= 1.0
        rows.append({"tick": tick, "Y": y, "T": t, "YD": yd, "C": c, "G": p.g, "H": h, "GOV_DEFICIT": p.g - t})
    return rows


if __name__ == "__main__":
    for r in run(5):
        print(r)
