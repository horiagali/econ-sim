#!/usr/bin/env python3
"""Normalise the Census 2021 education and activity tables into the margin file
of the population generator's second stage (spec `society/population-attributes`).

Reads two Eurostat downloads listed in sources.toml (JSON-stat), both at
development-region level (NUTS 2), and writes
`data/derived/census2021_edu_activity_ro.json`:

    activity[region][sex][age in years, 0..100][activity]      (100 = 100 or over)
    education[region][sex][age band][labour status][edu_level]

Counts are integers of real persons, all residents (private households or not).

Suppressed (confidential) cells are filled here, so the generator only sees
complete tables: within each published total (region x sex x age), what the
known cells leave over is spread evenly over the suppressed cells by largest
remainder. Education levels are then grouped into the five levels of the
population model.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import AGE_BANDS, AGE_CODES, SEXES, CensusError, spread  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
ACTIVITY_ID = "ro_census2021_activity_age"
EDUCATION_ID = "ro_census2021_activity_education"
PERSONS_ID = "ro_census2021_persons_hhstatus"

# Single years of age as published; the last one is open (100 or over).
YEAR_CODES = ["Y_LT1"] + [f"Y{i}" for i in range(1, 100)] + ["Y_GE100"]
# Activity categories of the margin file, with the census code of each.
ACTIVITIES = ["child", "in_education", "employed", "unemployed", "retired", "inactive_other"]
ACTIVITY_CODES = ["B_AGE_MIN", "EDUC", "EMP", "UNE", "INC", "INAC_OTH"]
# Labour status in the education table.
STATUSES = ["employed", "unemployed", "inactive"]
STATUS_CODES = ["EMP", "UNE", "INAC"]
# The five education levels of the population model (ISCED 2011 groups).
# "Not applicable" is everyone under 15; they are in the lowest level.
EDU_LEVELS = ["isced_0_2", "isced_3", "isced_4", "isced_5_6", "isced_7_8"]
EDU_GROUPS = [["ED0", "ED1", "ED2", "NAP"], ["ED3"], ["ED4"], ["ED5", "ED6"], ["ED7", "ED8"]]
EDU_CODES = [code for group in EDU_GROUPS for code in group]
MIN_WORKING_AGE = 15


def fill(cells: list, total: int | None, what: str) -> list[int]:
    """Replace suppressed (None) cells so that the cells sum to the published total."""
    if total is None:
        raise CensusError(f"{what}: the total is missing")
    holes = [i for i, v in enumerate(cells) if v is None]
    left = total - sum(v for v in cells if v is not None)
    if left and not holes:
        raise CensusError(f"{what}: cells sum to {total - left}, total says {total}")
    out = list(cells)
    for i, v in zip(holes, spread(left, [1] * len(holes))):
        out[i] = v
    return out


def no_unknowns(cells: dict, key_of, what: str) -> None:
    if any(v for k, v in cells.items() if key_of(k) == "UNK"):
        raise CensusError(f"{what}: the table has persons of unknown category; they are not handled")


def activity_table(doc: dict) -> tuple[list[str], list]:
    """(regions, activity[region][sex][year][activity]) from the activity-by-age table."""
    cells = {(r["geo"], r["sex"], r["age"], r["wstatus"]): int(r["value"]) for r in to_rows(doc)}
    no_unknowns(cells, lambda k: k[3], "activity table")
    regions = sorted({k[0] for k in cells})
    table = []
    for g in regions:
        table.append([])
        for s in SEXES:
            years = []
            for age, y in enumerate(YEAR_CODES):
                row = [cells.get((g, s, y, code)) for code in ACTIVITY_CODES]
                # Suppression never hides a structural zero: nobody under 15 has
                # an activity, nobody of 15 or more is below the minimum age.
                for i, code in enumerate(ACTIVITY_CODES):
                    if row[i] is None and (code == "B_AGE_MIN") != (age < MIN_WORKING_AGE):
                        row[i] = 0
                years.append(fill(row, cells.get((g, s, y, "TOTAL")), f"activity {g} {s} {y}"))
            table[-1].append(years)
    return regions, table


def education_table(doc: dict, regions: list[str]) -> list:
    """education[region][sex][band][status][edu_level] from the activity-by-education table."""
    cells = {(r["geo"], r["sex"], r["age"], r["isced11"], r["wstatus"]): int(r["value"]) for r in to_rows(doc)}
    no_unknowns(cells, lambda k: k[3], "education table (education)")
    no_unknowns(cells, lambda k: k[4], "education table (status)")
    if sorted({k[0] for k in cells}) != regions:
        raise CensusError("the education and activity tables cover different regions")
    table = []
    for g in regions:
        table.append([])
        for s in SEXES:
            bands = []
            for band, a in enumerate(AGE_CODES):
                adult = AGE_BANDS[band][0] >= MIN_WORKING_AGE
                flat = []
                for st in STATUS_CODES:
                    for code in EDU_CODES:
                        v = cells.get((g, s, a, code, st))
                        # Structural zeros: under 15 everyone is inactive with
                        # education "not applicable"; from 15 on nobody is.
                        if v is None and ((code == "NAP") == adult or (not adult and st != "INAC")):
                            v = 0
                        flat.append(v)
                flat = fill(flat, cells.get((g, s, a, "TOTAL", "TOTAL")), f"education {g} {s} {a}")
                by_status = []
                for i in range(len(STATUS_CODES)):
                    raw = dict(zip(EDU_CODES, flat[i * len(EDU_CODES):(i + 1) * len(EDU_CODES)]))
                    by_status.append([sum(raw[c] for c in group) for group in EDU_GROUPS])
                bands.append(by_status)
            table[-1].append(bands)
    return table


def county_regions(persons_doc: dict, regions: list[str]) -> dict[str, str]:
    """Each county's region: the NUTS 2 code is the first four characters of the NUTS 3 code."""
    idx = persons_doc["dimension"]["geo"]["category"]["index"]
    out = {}
    for county in sorted(idx):
        if county[:4] not in regions:
            raise CensusError(f"county {county} is in no region of the education and activity tables")
        out[county] = county[:4]
    return out


def check_against_counties(m: dict, county_margins: dict) -> None:
    """Both tables must hold exactly the persons of the county margins, per region, sex and age band."""
    region_of = m["region_of_county"]
    for ri, g in enumerate(m["regions"]):
        members = [ci for ci, c in enumerate(county_margins["counties"]) if region_of.get(c) == g]
        for si, s in enumerate(SEXES):
            for band, (lo, hi) in enumerate(AGE_BANDS):
                want = sum(county_margins[t][ci][si][band] for ci in members
                           for t in ("persons_private", "persons_collective"))
                years = range(lo, hi) if hi is not None else [lo]
                got_a = sum(sum(m["activity"][ri][si][y]) for y in years)
                got_e = sum(sum(row) for row in m["education"][ri][si][band])
                if not want == got_a == got_e:
                    raise CensusError(f"{g} {s} band {lo}: county margins have {want} persons, "
                                      f"activity table {got_a}, education table {got_e}")


def build(activity_doc: dict, education_doc: dict, persons_doc: dict) -> dict:
    regions, activity = activity_table(activity_doc)
    return {
        "regions": regions,
        "region_of_county": county_regions(persons_doc, regions),
        "sexes": SEXES,
        "age_bands": AGE_BANDS,
        "activities": ACTIVITIES,
        "statuses": STATUSES,
        "edu_levels": EDU_LEVELS,
        "activity": activity,
        "education": education_table(education_doc, regions),
    }


def totals(m: dict) -> dict:
    """National persons by activity and by education level."""
    by_activity = [0] * len(ACTIVITIES)
    by_edu = [0] * len(EDU_LEVELS)
    for region in m["activity"]:
        for sex in region:
            for year in sex:
                by_activity = [a + b for a, b in zip(by_activity, year)]
    for region in m["education"]:
        for sex in region:
            for band in sex:
                for status in band:
                    by_edu = [a + b for a, b in zip(by_edu, status)]
    return {"activity": dict(zip(ACTIVITIES, by_activity)), "edu_level": dict(zip(EDU_LEVELS, by_edu))}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--county-margins", type=Path, default=HERE / "data" / "derived" / "census2021_margins_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "census2021_edu_activity_ro.json")
    a = ap.parse_args()
    docs, prov = [], []
    for ds in (ACTIVITY_ID, EDUCATION_ID, PERSONS_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs.append(json.loads(raw))
        if ds == PERSONS_ID:
            continue  # only its county list is used; its provenance is in the county margin file
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m = build(*docs)
    check_against_counties(m, json.loads(a.county_margins.read_text(encoding="utf-8")))
    m["totals"] = totals(m)
    m["provenance"] = {"sources": prov,
                       "note": "Eurostat Census 2021 round; suppressed cells filled by normalise_census_attributes.py"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {len(m['regions'])} regions, {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
