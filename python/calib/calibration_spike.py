"""Spike 5: calibration loop from Python (ADR-0009).

1. Measures runs per hour at several population scales.
2. Morris elementary-effects screening (SALib) of the spike world's
   behaviour parameters.
3. One history-matching wave against two illustrative targets.
4. A 100-year quiet baseline run to check for drift or explosions.

Run:  uv run python calibration_spike.py      (from python/calib)
Targets here are illustrative placeholders, not Romanian data.
"""
from __future__ import annotations

import json
import sys
import time

import numpy as np
from SALib.analyze import morris as morris_analyze
from SALib.sample import morris as morris_sample

import econ_py

TICKS = 120  # 10 years
SCALE = 1000  # calibration scale (1:1000)
SEED = 42

PROBLEM = {
    "num_vars": 5,
    "names": ["matching_efficiency", "vacancy_ratio", "mpc_income", "mpc_wealth", "separation_rate"],
    "bounds": [[0.10, 0.50], [0.4, 1.4], [0.70, 0.95], [0.001, 0.02], [0.005, 0.03]],
}
# Illustrative targets (to be replaced by Romanian statistics):
TARGETS = {"unemployment": (0.055, 0.01), "consumption_to_wages": (0.95, 0.05)}


def outputs(params: dict, scale: int = SCALE, ticks: int = TICKS) -> dict:
    r = econ_py.run_scale(scale, SEED, ticks, params)
    u = float(np.mean(r["unemployment_rate"][ticks // 2:]))
    c = np.asarray(r["consumption_bani"][ticks // 2:], dtype=float)
    w = np.asarray(r["wage_bill_bani"][ticks // 2:], dtype=float)
    return {"unemployment": u, "consumption_to_wages": float(c.sum() / max(w.sum(), 1.0))}


def runs_per_hour() -> dict:
    res = {}
    for scale, n in ((1000, 20), (100, 3)):
        t0 = time.perf_counter()
        for _ in range(n):
            outputs({}, scale=scale)
        dt = (time.perf_counter() - t0) / n
        res[f"1:{scale}"] = {"seconds_per_120_tick_run": round(dt, 3), "runs_per_hour": int(3600 / dt)}
    return res


def morris() -> dict:
    X = morris_sample.sample(PROBLEM, N=10, num_levels=4, seed=SEED)
    Y = {k: [] for k in TARGETS}
    for row in X:
        out = outputs(dict(zip(PROBLEM["names"], map(float, row))))
        for k in Y:
            Y[k].append(out[k])
    result = {"runs": int(len(X))}
    for k, ys in Y.items():
        si = morris_analyze.analyze(PROBLEM, X, np.asarray(ys), num_levels=4, seed=SEED)
        result[k] = {n: round(float(m), 4) for n, m in zip(PROBLEM["names"], si["mu_star"])}
    return result


def history_matching_wave(n: int = 200) -> dict:
    rng = np.random.default_rng(SEED)
    lo = np.array([b[0] for b in PROBLEM["bounds"]])
    hi = np.array([b[1] for b in PROBLEM["bounds"]])
    X = lo + (hi - lo) * rng.random((n, len(lo)))
    keep = []
    for row in X:
        out = outputs(dict(zip(PROBLEM["names"], map(float, row))))
        imp = max(abs(out[k] - t) / s for k, (t, s) in TARGETS.items())
        if imp < 3.0:
            keep.append(row)
    keep = np.asarray(keep)
    ranges = {}
    if len(keep):
        for i, name in enumerate(PROBLEM["names"]):
            ranges[name] = [round(float(keep[:, i].min()), 4), round(float(keep[:, i].max()), 4)]
    return {"runs": n, "not_ruled_out": int(len(keep)), "fraction": round(len(keep) / n, 3), "remaining_ranges": ranges}


def quiet_baseline() -> dict:
    ticks = 1200
    r = econ_py.run_scale(SCALE, SEED, ticks, {})
    u = np.asarray(r["unemployment_rate"])
    c = np.asarray(r["consumption_bani"], dtype=float)
    return {
        "ticks": ticks,
        "unemployment_first_10y_mean": round(float(u[:120].mean()), 4),
        "unemployment_last_10y_mean": round(float(u[-120:].mean()), 4),
        "unemployment_min_max": [round(float(u.min()), 4), round(float(u.max()), 4)],
        "consumption_last_over_first_10y": round(float(c[-120:].mean() / c[:120].mean()), 4),
        "all_finite": bool(np.isfinite(c).all() and np.isfinite(u).all()),
    }


def main() -> int:
    report = {}
    t0 = time.perf_counter()
    report["runs_per_hour"] = runs_per_hour()
    report["morris_mu_star"] = morris()
    report["history_matching_wave_1"] = history_matching_wave()
    report["quiet_baseline_100y"] = quiet_baseline()
    report["total_seconds"] = round(time.perf_counter() - t0, 1)
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
