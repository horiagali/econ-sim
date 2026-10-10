#!/usr/bin/env python3
"""Normalise the Census 2021 employment tables and two Labour Force Survey
cross-tables into the margin file of the population generator's jobs stage
(spec `society/population-jobs`).

Writes `data/derived/census2021_jobs_ro.json`:

    by_industry_group[region][sex][age band][kind][industry group]  census, real persons
    by_occupation[region][sex][age band][kind][occupation]   census, real persons
    pattern_edu_occupation[sex][edu group][occupation]       LFS 2021, EU-27, persons
    pattern_occupation_industry_group[sex][occupation][industry group]   LFS 2021, EU-27, persons

The census tables are margins: the generator reproduces them. The LFS tables
are patterns: only their proportions are used, to decide which occupations go
with which education and which industry group. They are EU-27 totals because most
cells of the Romanian tables are not published (too few respondents).

Suppressed census cells are filled as in normalise_census_attributes.py: what
the known cells of a (region, sex, age band) leave over of its employed total
is spread evenly over the suppressed cells. LFS cells that are not published
(too few respondents) are zero.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import AGE_BANDS, AGE_CODES, SEXES, CensusError  # noqa: E402
from normalise_census_attributes import EDU_LEVELS, MIN_WORKING_AGE, fill, no_unknowns  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
INDUSTRY_ID = "ro_census2021_employment_industry"
OCCUPATION_ID = "ro_census2021_employment_occupation"
LFS_INDUSTRY_ID = "eu_lfs2021_occupation_industry"
LFS_EDU_ID = "eu_lfs2021_occupation_education"

# Status in employment (ICSE), with the census code of each.
KINDS = ["employee", "employer", "own_account", "family_worker"]
KIND_CODES = ["SAL", "SELF_S", "SELF_NS", "CFAM_COOP"]
# Occupations: ISCO-08 major groups; the index is the major group's digit.
OCCUPATIONS = ["armed_forces", "managers", "professionals", "technicians", "clerks", "service_sales",
               "skilled_agricultural", "craft", "operators", "elementary"]
OCCUPATION_CODES = [f"OC{i}" for i in range(10)]
# Industry groups: the ten NACE Rev. 2 groups of the census table, and the sections in each.
INDUSTRY_GROUPS = ["agriculture", "industry", "construction", "trade_transport_hospitality", "information_communication",
           "finance", "real_estate", "professional_administrative", "public_education_health", "other_services"]
GROUP_CODES = ["A", "B-E", "F", "G-I", "J", "K", "L", "M_N", "O-Q", "R-U"]
GROUP_SECTIONS = ["A", "BCDE", "F", "GHI", "J", "K", "L", "MN", "OPQ", "RSTU"]
# Education groups of the LFS table, and the group of each of the model's five levels.
EDU_GROUPS = ["isced_0_2", "isced_3_4", "isced_5_8"]
EDU_GROUP_CODES = ["ED0-2", "ED3_4", "ED5-8"]
EDU_GROUP_OF_LEVEL = [0, 1, 1, 2, 2]
LFS_SEX = ["F", "M"]
LFS_AGE = "Y_GE15"
# The two census tables suppress different cells, so their employed totals may
# differ by a few persons per region, sex and age band.
MAX_TABLE_GAP = 25


def census_table(doc: dict, dim: str, codes: list[str], what: str) -> tuple[list[str], list]:
    """(regions, table[region][sex][band][kind][category]) from one census employment table."""
    cells = {(r["geo"], r["sex"], r["age"], r[dim], r["wstatus"]): int(r["value"]) for r in to_rows(doc)}
    no_unknowns(cells, lambda k: k[3], f"{what} table (category)")
    no_unknowns(cells, lambda k: k[4], f"{what} table (kind)")
    regions = sorted({k[0] for k in cells})
    table = []
    for g in regions:
        table.append([])
        for s in SEXES:
            bands = []
            for band, a in enumerate(AGE_CODES):
                if AGE_BANDS[band][0] < MIN_WORKING_AGE:
                    bands.append([[0] * len(codes) for _ in KIND_CODES])
                    continue
                total = cells.get((g, s, a, "TOTAL", "TOTAL"))
                not_employed = cells.get((g, s, a, "NAP", "TOTAL"))
                if total is None or not_employed is None:
                    raise CensusError(f"{what} {g} {s} {a}: the employed total is missing")
                flat = [cells.get((g, s, a, code, kind)) for kind in KIND_CODES for code in codes]
                flat = fill(flat, total - not_employed, f"{what} {g} {s} {a}")
                bands.append([flat[i * len(codes):(i + 1) * len(codes)] for i in range(len(KIND_CODES))])
            table[-1].append(bands)
    return regions, table


def lfs_cells(doc: dict, row_dim: str, col_dim: str) -> dict:
    """{(sex, row code, column code): persons} from an LFS table in thousands of persons."""
    out = {}
    for r in to_rows(doc):
        if r["age"] == LFS_AGE and r["sex"] in LFS_SEX:
            out[(r["sex"], r[row_dim], r[col_dim])] = round(float(r["value"]) * 1000)
    return out


def pattern_edu_occupation(doc: dict) -> list:
    cells = lfs_cells(doc, "isced11", "isco08")
    table = [[[cells.get((s, e, o), 0) for o in OCCUPATION_CODES] for e in EDU_GROUP_CODES] for s in SEXES]
    for s, by_edu in zip(SEXES, table):
        if any(sum(row) == 0 for row in by_edu) or any(sum(row[o] for row in by_edu) == 0 for o in (1, 2, 5, 9)):
            raise CensusError(f"LFS education x occupation table is empty for {s}")
    return table


def pattern_occupation_industry_group(doc: dict) -> list:
    cells = lfs_cells(doc, "isco08", "nace_r2")
    table = [[[sum(cells.get((s, o, section), 0) for section in sections) for sections in GROUP_SECTIONS]
              for o in OCCUPATION_CODES] for s in SEXES]
    for s, by_occ in zip(SEXES, table):
        if any(sum(row[k] for row in by_occ) == 0 for k in range(len(INDUSTRY_GROUPS))):
            raise CensusError(f"LFS occupation x industry group table has an empty industry group for {s}")
    return table


def build(industry_doc: dict, occupation_doc: dict, lfs_industry_doc: dict, lfs_edu_doc: dict) -> dict:
    regions, by_industry_group = census_table(industry_doc, "nace_r2", GROUP_CODES, "industry_group")
    regions_o, by_occupation = census_table(occupation_doc, "isco08", OCCUPATION_CODES, "occupation")
    if regions != regions_o:
        raise CensusError("the industry and occupation tables cover different regions")
    return {
        "regions": regions,
        "sexes": SEXES,
        "age_bands": AGE_BANDS,
        "kinds": KINDS,
        "occupations": OCCUPATIONS,
        "industry_groups": INDUSTRY_GROUPS,
        "edu_levels": EDU_LEVELS,
        "edu_groups": EDU_GROUPS,
        "edu_group_of_level": EDU_GROUP_OF_LEVEL,
        "by_industry_group": by_industry_group,
        "by_occupation": by_occupation,
        "pattern_edu_occupation": pattern_edu_occupation(lfs_edu_doc),
        "pattern_occupation_industry_group": pattern_occupation_industry_group(lfs_industry_doc),
    }


def check_against_activity(m: dict, activity_margins: dict) -> None:
    """Both tables must hold the employed of the activity table, per region, sex and age band,
    up to what the filling of suppressed cells moves."""
    if m["regions"] != activity_margins["regions"]:
        raise CensusError("the jobs and activity tables cover different regions")
    employed = activity_margins["activities"].index("employed")
    for ri, g in enumerate(m["regions"]):
        for si, s in enumerate(SEXES):
            for band, (lo, hi) in enumerate(AGE_BANDS):
                years = range(lo, hi) if hi is not None else [lo]
                want = sum(activity_margins["activity"][ri][si][y][employed] for y in years)
                for name in ("by_industry_group", "by_occupation"):
                    got = sum(sum(row) for row in m[name][ri][si][band])
                    if abs(got - want) > MAX_TABLE_GAP:
                        raise CensusError(f"{g} {s} band {lo}: activity table has {want} employed, {name} {got}")


def totals(m: dict) -> dict:
    """National employed persons by kind, industry group and occupation."""
    def national(table: list, n: int, by_kind: bool) -> list[int]:
        out = [0] * n
        for region in table:
            for sex in region:
                for band in sex:
                    for k, row in enumerate(band):
                        if by_kind:
                            out[k] += sum(row)
                        else:
                            out = [a + b for a, b in zip(out, row)]
        return out
    return {
        "kind": dict(zip(KINDS, national(m["by_industry_group"], len(KINDS), True))),
        "industry_group": dict(zip(INDUSTRY_GROUPS, national(m["by_industry_group"], len(INDUSTRY_GROUPS), False))),
        "occupation": dict(zip(OCCUPATIONS, national(m["by_occupation"], len(OCCUPATIONS), False))),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--activity-margins", type=Path,
                    default=HERE / "data" / "derived" / "census2021_edu_activity_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "census2021_jobs_ro.json")
    a = ap.parse_args()
    docs, prov = [], []
    for ds in (INDUSTRY_ID, OCCUPATION_ID, LFS_INDUSTRY_ID, LFS_EDU_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs.append(json.loads(raw))
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m = build(*docs)
    check_against_activity(m, json.loads(a.activity_margins.read_text(encoding="utf-8")))
    m["totals"] = totals(m)
    m["provenance"] = {"sources": prov,
                       "note": "Eurostat Census 2021 round (margins) and EU Labour Force Survey 2021, EU-27 (patterns); "
                               "suppressed census cells filled by normalise_census_jobs.py"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {len(m['regions'])} regions, {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
