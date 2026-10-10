#!/usr/bin/env python3
"""Normalise the Census 2021 locality-size and tenure tables into the margin
file of the population generator's housing stage (spec `society/population-housing`).

Writes `data/derived/census2021_housing_ro.json`, both tables by county (NUTS 3):

    persons_by_locality[county][age group][locality class]      all residents
    households_by_tenure[county][size group][tenure]            private households

Locality classes group the census classes of locality size (see LOCALITY_CLASSES).
Size groups are one-person and larger households. Tenure is owner, tenant, other.

Suppressed cells of the locality table are filled here: a locality class the
county has nobody in stays zero; inside each county and age group, what the
known cells leave over of the published total is spread evenly over the
suppressed cells. Households of unknown tenure (0.7%) are spread over the three
tenures in proportion.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import SEXES, CensusError, spread  # noqa: E402
from normalise_census_attributes import fill  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
LOCALITY_ID = "ro_census2021_locality_size"
TENURE_ID = "ro_census2021_households_tenure"

# Census classes of locality size (persons), and the model's six classes.
SIZE_CODES = ["LT200", "200-499", "500-999", "1000-1999", "2000-4999", "5000-9999", "10000-19999", "20000-49999",
              "50000-99999", "100000-199999", "200000-499999", "500000-999999", "GE1000000"]
LOCALITY_CLASSES = ["under_2000", "2000_4999", "5000_9999", "10000_49999", "50000_199999", "200000_or_more"]
LOCALITY_MIN = [0, 2000, 5000, 10000, 50000, 200000]
CLASS_OF_SIZE_CODE = [0, 0, 0, 0, 1, 2, 3, 3, 4, 4, 5, 5, 5]
# A locality of this class or above counts as urban (10,000 inhabitants or more).
URBAN_FROM_CLASS = 3
# Broad age groups as published, and the first age (years) of each.
AGE_GROUP_CODES = ["Y_LT15", "Y15-29", "Y30-49", "Y50-64", "Y65-84", "Y_GE85"]
AGE_GROUP_FROM = [0, 15, 30, 50, 65, 85]
SIZE_GROUPS = ["one_person", "larger"]
TENURES = ["owner", "tenant", "other"]
TENURE_CODES = ["OWN", "RENT", "OTH"]


def locality_table(doc: dict) -> tuple[list[str], list]:
    """(counties, persons[county][age group][locality class]), both sexes together."""
    cells = {(r["geo"], r["sex"], r["age"], r["n_person"]): int(r["value"]) for r in to_rows(doc)}
    counties = sorted({k[0] for k in cells})
    table = []
    for g in counties:
        county_by_code = []
        for code in SIZE_CODES:
            parts = [cells.get((g, s, "TOTAL", code)) for s in SEXES]
            if any(v is None for v in parts):
                raise CensusError(f"locality {g} {code}: the county total is missing")
            county_by_code.append(sum(parts))
        groups = []
        for a in AGE_GROUP_CODES:
            by_code = [0] * len(SIZE_CODES)
            for s in SEXES:
                row = [cells.get((g, s, a, code)) for code in SIZE_CODES]
                # Nobody of any age lives in a class the county does not have.
                row = [0 if v is None and county_by_code[i] == 0 else v for i, v in enumerate(row)]
                row = fill(row, cells.get((g, s, a, "TOTAL")), f"locality {g} {s} {a}")
                by_code = [x + y for x, y in zip(by_code, row)]
            by_class = [0] * len(LOCALITY_CLASSES)
            for code_index, v in enumerate(by_code):
                by_class[CLASS_OF_SIZE_CODE[code_index]] += v
            groups.append(by_class)
        table.append(groups)
    return counties, table


def tenure_table(doc: dict, counties: list[str]) -> list:
    """households[county][size group][tenure], unknown tenure spread in proportion."""
    cells = {(r["geo"], r["hhcomp"], r["tenure"]): int(r["value"]) for r in to_rows(doc)}
    table = []
    for g in counties:
        rows = []
        for group in SIZE_GROUPS:
            def cell(tenure: str) -> int:
                total = cells.get((g, "TOTAL", tenure))
                single = cells.get((g, "P1", tenure))
                if total is None or single is None:
                    raise CensusError(f"tenure {g} {tenure}: a cell is missing")
                return single if group == "one_person" else total - single
            known = [cell(t) for t in TENURE_CODES]
            everyone = cell("TOTAL")
            unknown = everyone - sum(known)
            if unknown < 0 or any(v < 0 for v in known):
                raise CensusError(f"tenure {g} {group}: tenures exceed the total")
            rows.append([k + extra for k, extra in zip(known, spread(unknown, known))])
        table.append(rows)
    return table


def build(locality_doc: dict, tenure_doc: dict) -> dict:
    counties, persons = locality_table(locality_doc)
    return {
        "counties": counties,
        "locality_classes": LOCALITY_CLASSES,
        "locality_min": LOCALITY_MIN,
        "urban_from_class": URBAN_FROM_CLASS,
        "age_group_from": AGE_GROUP_FROM,
        "size_groups": SIZE_GROUPS,
        "tenures": TENURES,
        "persons_by_locality": persons,
        "households_by_tenure": tenure_table(tenure_doc, counties),
    }


def check_against_counties(m: dict, county_margins: dict) -> None:
    """Persons and households must be exactly those of the county margins."""
    if m["counties"] != county_margins["counties"]:
        raise CensusError("the housing tables and the county margins cover different counties")
    for ci, g in enumerate(m["counties"]):
        persons = sum(sum(sex) for t in ("persons_private", "persons_collective") for sex in county_margins[t][ci])
        got = sum(sum(row) for row in m["persons_by_locality"][ci])
        if got != persons:
            raise CensusError(f"{g}: county margins have {persons} persons, the locality table {got}")
        households = county_margins["households"][ci]
        single, larger = (sum(row) for row in m["households_by_tenure"][ci])
        if single != households[0] or larger != sum(households[1:]):
            raise CensusError(f"{g}: the tenure table does not hold the households of the county margins")


def totals(m: dict) -> dict:
    by_class = [sum(row[k] for county in m["persons_by_locality"] for row in county) for k in range(len(LOCALITY_CLASSES))]
    by_tenure = [sum(row[k] for county in m["households_by_tenure"] for row in county) for k in range(len(TENURES))]
    return {"persons_by_locality": dict(zip(LOCALITY_CLASSES, by_class)),
            "persons_urban": sum(by_class[URBAN_FROM_CLASS:]),
            "households_by_tenure": dict(zip(TENURES, by_tenure))}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--county-margins", type=Path, default=HERE / "data" / "derived" / "census2021_margins_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "census2021_housing_ro.json")
    a = ap.parse_args()
    docs, prov = [], []
    for ds in (LOCALITY_ID, TENURE_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs.append(json.loads(raw))
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m = build(*docs)
    check_against_counties(m, json.loads(a.county_margins.read_text(encoding="utf-8")))
    m["totals"] = totals(m)
    m["provenance"] = {"sources": prov,
                       "note": "Eurostat Census 2021 round; suppressed cells filled by normalise_census_housing.py"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {len(m['counties'])} counties, {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
