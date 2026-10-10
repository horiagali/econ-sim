#!/usr/bin/env python3
"""Normalise the Census 2021 tables into the population generator's margin file.

Reads the two Eurostat downloads listed in sources.toml (JSON-stat) and writes
`data/derived/census2021_margins_ro.json`: integer counts of real persons and
households per county (spec `society/population-generator`, "Inputs").

Suppressed (confidential) cells are filled here, so the generator only sees
complete integer tables: not-private = total - private per cell; where the
total is suppressed too, the county-and-sex residual is spread over the
suppressed cells by largest remainder.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
PERSONS_ID = "ro_census2021_persons_hhstatus"
HOUSEHOLDS_ID = "ro_census2021_households_size"

SEXES = ["F", "M"]
# Five-year bands as published; the last one is open (100+).
AGE_CODES = ["Y_LT5", "Y5-9", "Y10-14", "Y15-19", "Y20-24", "Y25-29", "Y30-34", "Y35-39", "Y40-44", "Y45-49",
             "Y50-54", "Y55-59", "Y60-64", "Y65-69", "Y70-74", "Y75-79", "Y80-84", "Y85-89", "Y90-94", "Y95-99",
             "Y_GE100"]
AGE_BANDS = [[5 * i, 5 * i + 5] for i in range(20)] + [[100, None]]
SIZE_CODES = ["1", "2", "3", "4", "5", "6-10", "GE11"]
SIZE_CLASSES = [[1, 1], [2, 2], [3, 3], [4, 4], [5, 5], [6, 10], [11, None]]


class CensusError(ValueError):
    """The census tables cannot be turned into consistent margins."""


def spread(total: int, weights: list[int]) -> list[int]:
    """Split `total` over cells in proportion to `weights` (largest remainder, ties to the lower index)."""
    if total < 0:
        raise CensusError(f"negative residual {total}")
    if not weights:
        if total:
            raise CensusError(f"residual {total} with no cell to put it in")
        return []
    wsum = sum(weights)
    if wsum == 0:
        weights, wsum = [1] * len(weights), len(weights)
    parts = [total * w // wsum for w in weights]
    order = sorted(range(len(weights)), key=lambda i: (-(total * weights[i] % wsum), i))
    for i in order[: total - sum(parts)]:
        parts[i] += 1
    return parts


def person_tables(doc: dict) -> tuple[list[str], dict, dict]:
    """(counties, private[county][sex][age], collective[county][sex][age]) from the persons table."""
    cells: dict = {}
    for r in to_rows(doc):
        cells[(r["geo"], r["hhstatus"], r["sex"], r["age"])] = int(r["value"])
    counties = sorted({k[0] for k in cells})
    private: dict = {}
    collective: dict = {}
    for c in counties:
        private[c], collective[c] = {}, {}
        for s in SEXES:
            prv_total = cells.get((c, "PRV", s, "TOTAL"))
            all_total = cells.get((c, "TOTAL", s, "TOTAL"))
            if prv_total is None or all_total is None:
                raise CensusError(f"{c} {s}: county totals are missing")
            prv = [cells.get((c, "PRV", s, a)) for a in AGE_CODES]
            if any(v is None for v in prv):
                known = sum(v for v in prv if v is not None)
                holes = [i for i, v in enumerate(prv) if v is None]
                tot = [cells.get((c, "TOTAL", s, AGE_CODES[i])) or 0 for i in holes]
                for i, v in zip(holes, spread(prv_total - known, tot)):
                    prv[i] = v
            if sum(prv) != prv_total:
                raise CensusError(f"{c} {s}: private age bands sum to {sum(prv)}, total says {prv_total}")
            col: list = []
            for i, a in enumerate(AGE_CODES):
                tot = cells.get((c, "TOTAL", s, a))
                npr = cells.get((c, "NPRV", s, a))
                col.append(tot - prv[i] if tot is not None else npr)
            if any(v is not None and v < 0 for v in col):
                raise CensusError(f"{c} {s}: a private cell exceeds its total")
            known = sum(v for v in col if v is not None)
            holes = [i for i, v in enumerate(col) if v is None]
            for i, v in zip(holes, spread(all_total - prv_total - known, [1] * len(holes))):
                col[i] = v
            if sum(col) != all_total - prv_total:
                raise CensusError(f"{c} {s}: not-private age bands do not sum to the county total")
            private[c][s], collective[c][s] = prv, col
    return counties, private, collective


def household_table(doc: dict, counties: list[str]) -> dict:
    cells = {(r["geo"], r["n_person"]): int(r["value"]) for r in to_rows(doc) if r.get("hhcomp", "TOTAL") == "TOTAL"}
    out = {}
    for c in counties:
        row = [cells.get((c, k)) for k in SIZE_CODES]
        total = cells.get((c, "TOTAL"))
        if total is None or any(v is None for v in row):
            raise CensusError(f"{c}: household size table is incomplete")
        if sum(row) != total:
            raise CensusError(f"{c}: size classes sum to {sum(row)}, total says {total}")
        out[c] = row
    return out


def check_consistent(counties: list[str], private: dict, households: dict, max_household_size: int) -> None:
    """Private persons must fit the households of each county."""
    for c in counties:
        persons = sum(sum(private[c][s]) for s in SEXES)
        lo = sum(n * cls[0] for n, cls in zip(households[c], SIZE_CLASSES))
        hi = sum(n * (cls[1] or max_household_size) for n, cls in zip(households[c], SIZE_CLASSES))
        if not lo <= persons <= hi:
            raise CensusError(f"{c}: {persons} private persons cannot fit households holding {lo} to {hi}")


def build(persons_doc: dict, households_doc: dict, max_household_size: int = 15) -> dict:
    counties, private, collective = person_tables(persons_doc)
    households = household_table(households_doc, counties)
    check_consistent(counties, private, households, max_household_size)
    return {
        "counties": counties,
        "sexes": SEXES,
        "age_bands": AGE_BANDS,
        "size_classes": SIZE_CLASSES,
        "persons_private": [[private[c][s] for s in SEXES] for c in counties],
        "persons_collective": [[collective[c][s] for s in SEXES] for c in counties],
        "households": [households[c] for c in counties],
    }


def totals(m: dict) -> dict:
    flat = lambda t: sum(v for county in t for sex in county for v in sex)  # noqa: E731
    return {
        "persons_private": flat(m["persons_private"]),
        "persons_collective": flat(m["persons_collective"]),
        "households": sum(v for county in m["households"] for v in county),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "census2021_margins_ro.json")
    a = ap.parse_args()
    docs, prov = [], []
    for ds in (PERSONS_ID, HOUSEHOLDS_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs.append(json.loads(raw))
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m = build(*docs)
    m["totals"] = totals(m)
    m["provenance"] = {"sources": prov, "note": "Eurostat Census 2021 round; suppressed cells filled by normalise_census.py"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {len(m['counties'])} counties, {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
