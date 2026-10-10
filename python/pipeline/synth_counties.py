#!/usr/bin/env python3
"""Spike 7: synthesise a weighted population for a few counties with IPF.

Offline mode (--fixture) uses illustrative marginals from fixtures/ and a
generated seed sample. The real pipeline replaces the seed sample with the
IPUMS 2011 Romania 10% sample and the marginals with Census 2021 tables.
The sample scale is a parameter (--scale), never a constant.
"""
from __future__ import annotations

import argparse
import csv
import json
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ipf import integerise, ipf  # noqa: E402

HERE = Path(__file__).resolve().parent


def seed_sample(n: int, counties: list[str], seed: int) -> list[dict]:
    rnd = random.Random(seed)
    ages = ["0-14", "15-24", "25-49", "50-64", "65+"]
    return [{"county": rnd.choice(counties), "age": rnd.choice(ages), "sex": rnd.choice("FM")} for _ in range(n)]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--fixture", action="store_true", help="use illustrative offline marginals")
    ap.add_argument("--scale", type=int, default=100, help="real people per synthetic person")
    ap.add_argument("--seed", type=int, default=42)
    a = ap.parse_args()
    if not a.fixture:
        print("Only --fixture mode is implemented in Spike 7 (real marginals need the INS tables).")
        return 2
    m = json.loads((HERE / "fixtures" / "marginals_illustrative.json").read_text(encoding="utf-8"))
    total = sum(m["county"].values())
    n_records = max(1, total // a.scale)
    recs = seed_sample(n_records, list(m["county"]), a.seed)
    w0 = [total / n_records] * n_records
    w, iters = ipf(recs, w0, {"county": m["county"], "age": m["age"], "sex": m["sex"]})
    wi = integerise(w)
    out_dir = HERE / "data" / "derived"
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / f"population_fixture_1to{a.scale}.csv"
    with out.open("w", newline="", encoding="utf-8") as f:
        wr = csv.writer(f)
        wr.writerow(["person_id", "county", "age", "sex", "weight"])
        for i, (r, wt) in enumerate(zip(recs, wi)):
            wr.writerow([i, r["county"], r["age"], r["sex"], wt])
    prov = {"marginals": "fixtures/marginals_illustrative.json (ILLUSTRATIVE)", "seed": a.seed, "scale": a.scale,
            "records": n_records, "ipf_iterations": iters, "total_weight": sum(wi)}
    (out_dir / (out.stem + ".provenance.json")).write_text(json.dumps(prov, indent=2), encoding="utf-8")
    print(f"OK — {n_records} records, total weight {sum(wi)}, IPF converged in {iters} iterations -> {out.relative_to(HERE)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
