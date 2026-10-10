#!/usr/bin/env python3
"""Normalise pension data into the margin file of the population generator's
pension stage (spec `society/population-pensions`).

Writes `data/derived/pensions2021_ro.json`:

    pension_bill                         lei per month: expenditure on pensions in 2021 / 12
    minimum_pension                      lei per month (the social indemnity for pensioners)
    rel_sex[sex]                         mean pension of the sex, as proportions, millionths
    rel_education[sex][edu group]        mean earnings of the education group / mean of the sex, millionths
    quantile_curve                       pension at a rank / mean pension, millionths, as a line through knots
    replacement_ratio                    median pension of 65-74 over median earnings of 50-59 (for checking)

What is published about Romanian pensions through Eurostat is the total
spent, the gap between women's and men's pensions and one replacement ratio.
The rest is built:

- The quantile curve is flat at the minimum pension up to the share of
  pensioners who receive it, and above that has the shape of the earnings
  curve, stretched so that the curve averages 1.
- Pensions follow career earnings. No table gives pensions by education, so
  the stage starts from the earnings proportions by education and weakens
  them with a parameter (see the spec).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import SEXES, CensusError  # noqa: E402
from normalise_census_jobs import EDU_GROUP_CODES, EDU_GROUP_OF_LEVEL, EDU_GROUPS  # noqa: E402
from normalise_earnings import ALL, TABLES, survey_cells  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
UNIT = 1_000_000
EXPENDITURE_ID = "ro_pensions_expenditure_2021"
GAP_ID = "ro_pension_gender_gap"
REPLACEMENT_ID = "ro_pension_replacement_ratio"
EDUCATION_ID = TABLES["rel_education"][0]
YEAR = "2021"
# Not in any Eurostat table; from Romanian law and the pension house's monthly figures.
MANUAL = [
    {"id": "ro_minimum_pension_2021", "value": 800, "unit": "lei per month",
     "what": "social indemnity for pensioners (the guaranteed minimum pension), in force from September 2020 "
             "to December 2021",
     "source": "https://www.gandul.ro/social/unul-din-cinci-pensionari-traieste-la-limita-extrema-a-saraciei-ce-pensii-primesc-lunar-19663243",
     "confidence": "high"},
    {"id": "ro_minimum_pension_beneficiaries_2021", "value": 925_814, "unit": "persons",
     "what": "pensioners receiving the social indemnity in June 2021 (National House of Public Pensions, as reported)",
     "source": "https://economedia.ro/?p=2129",
     "confidence": "medium"},
]


def manual(dataset_id: str) -> int:
    return next(m["value"] for m in MANUAL if m["id"] == dataset_id)


def pension_bill(doc: dict) -> int:
    """Lei per month: the year's expenditure on all pensions (million lei) / 12."""
    cells = {r["spdepb"]: float(r["value"]) for r in to_rows(doc)
             if r["time"] == YEAR and r["spdepm"] == "TOTAL" and r["unit"] == "MIO_NAC"}
    if "TOTAL" not in cells:
        raise CensusError(f"pension expenditure: no total for {YEAR}")
    return round(cells["TOTAL"] * 1_000_000 / 12)


def one_value(doc: dict, what: str, **where: str) -> float:
    rows = [r for r in to_rows(doc) if all(r.get(k) == v for k, v in where.items())]
    if len(rows) != 1:
        raise CensusError(f"{what}: expected one value, found {len(rows)}")
    return float(rows[0]["value"])


def rel_education(doc: dict) -> list:
    """[sex][education group]: mean earnings of the group over the mean of the sex, whole survey, millionths."""
    cells = survey_cells(doc, "isced11")
    table = []
    for sex in SEXES:
        total = cells.get((ALL, sex, "TOTAL"))
        row = [cells.get((ALL, sex, code)) for code in EDU_GROUP_CODES]
        if not total or any(v is None for v in row):
            raise CensusError(f"earnings by education: the whole-survey cells of {sex} are missing")
        table.append([round(v / total * UNIT) for v in row])
    return table


def quantile_curve(earnings_curve: dict, floor_over_mean: float, share_at_floor: float) -> dict:
    """Pension at a rank over the mean pension, as a line through knots (both in millionths).

    Flat at the minimum pension up to `share_at_floor`; above it the shape of
    the earnings curve, stretched so that the whole curve averages 1.
    """
    if not 0 < floor_over_mean < 1 or not 0 <= share_at_floor < 1:
        raise CensusError("pensions: the minimum pension must be below the mean, and its share below 1")
    ranks = [r / UNIT for r in earnings_curve["rank"]]
    rises = [(v - earnings_curve["value"][0]) / UNIT for v in earnings_curve["value"]]
    area = sum((b - a) * (ra + rb) / 2 for a, b, ra, rb in zip(ranks, ranks[1:], rises, rises[1:]))
    stretch = (1 - floor_over_mean) / ((1 - share_at_floor) * area)
    out_ranks, out_values = [0], [floor_over_mean]
    for rank, rise in zip(ranks, rises):
        at = round((share_at_floor + (1 - share_at_floor) * rank) * UNIT)
        if at > out_ranks[-1]:
            out_ranks.append(at)
            out_values.append(floor_over_mean + stretch * rise)
    out_ranks[-1] = UNIT
    return {"rank": out_ranks, "value": [round(v * UNIT) for v in out_values]}


def build(docs: dict, earnings: dict, retired: int) -> dict:
    """`earnings` is the earnings margin file (for the shape of its curve); `retired` the census's retired persons."""
    if retired <= 0:
        raise CensusError("pensions: no retired persons")
    bill = pension_bill(docs[EXPENDITURE_ID])
    minimum = manual("ro_minimum_pension_2021")
    share = manual("ro_minimum_pension_beneficiaries_2021") / retired
    gap = one_value(docs[GAP_ID], "gender pension gap", time=YEAR, age="Y_GE65")
    if not 0 <= gap < 100:
        raise CensusError("gender pension gap: not a percentage")
    return {
        "sexes": SEXES,
        "edu_groups": EDU_GROUPS,
        "edu_group_of_level": EDU_GROUP_OF_LEVEL,
        "pension_bill": bill,
        "minimum_pension": minimum,
        "rel_sex": [round((100 - gap) / 100 * UNIT), UNIT],
        "rel_education": rel_education(docs[EDUCATION_ID]),
        "quantile_curve": quantile_curve(earnings["quantile_curve"], minimum * retired / bill, share),
        "replacement_ratio": round(one_value(docs[REPLACEMENT_ID], "replacement ratio", time=YEAR, sex="T") * UNIT),
        "totals": {"retired": retired, "mean_pension": round(bill / retired, 2),
                   "share_at_minimum": round(share, 4)},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--earnings", type=Path, default=HERE / "data" / "derived" / "earnings2021_ro.json")
    ap.add_argument("--activity-margins", type=Path,
                    default=HERE / "data" / "derived" / "census2021_edu_activity_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "pensions2021_ro.json")
    a = ap.parse_args()
    docs, prov = {}, []
    for ds in (EXPENDITURE_ID, GAP_ID, REPLACEMENT_ID, EDUCATION_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs[ds] = json.loads(raw)
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    earnings = json.loads(a.earnings.read_text(encoding="utf-8"))
    retired = json.loads(a.activity_margins.read_text(encoding="utf-8"))["totals"]["activity"]["retired"]
    m = build(docs, earnings, retired)
    m["provenance"] = {"sources": prov, "manual": MANUAL,
                       "note": "Eurostat: social protection expenditure 2021 (level), EU-SILC (gender pension gap, "
                               "replacement ratio), Structure of Earnings Survey 2022 (shape and education "
                               "proportions, by way of earnings); minimum pension and its beneficiaries entered by hand"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - pension bill {m['pension_bill']:,} lei a month, {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
