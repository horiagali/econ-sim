#!/usr/bin/env python3
"""Normalise the demographic tables of the starting state (spec `economy/demographics`).

Writes `data/derived/demography2019_ro.json`:

    mortality.yearly_ppb[sex][age]      probability of dying within a year, single years 0 to 109,
                                        the average of 2017, 2018 and 2019 (parts per billion)
    mortality.monthly_ppb[sex][age]     the same as a monthly probability, m = 1 - (1 - q)^(1/12)
    fertility.rates_ppm[age - 15]       births per woman a year, single years 15 to 49, 2019 (millionths)
    sex_ratio_at_birth_ppm              share of boys among births (2021)
    retirement_age_schedule[sex]        the law's dated standard retirement age, months of age

**Pre-pandemic on purpose** (owner, 2026-10-26: the simulation runs as if COVID
never happened): mortality is the 2017-2019 table, not 2020-2021's, and
fertility the 2019 rates (2021's were not depressed by the pandemic).

Measured: Eurostat `demo_mlifetable` (PROBDEATH, single years 0 to 94; 95 and
over is closed at 1) and `demo_frate` (single years 15 to 49, with the groups
under 15 and 50 and over). Built: the probabilities from 95 to 109, extended
from ages 80 to 94 along a straight line in log-odds; the births of mothers
under 15 are folded into age 15 and of mothers 50 and over into 49, so the
total fertility rate is kept. Entered by hand: the sex ratio at birth and the
women's retirement schedule (Law 263/2010, Annex 5, as the research note
reads it; medium confidence).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import CensusError  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
MORTALITY_ID = "ro_mortality_2017_2019"
FERTILITY_ID = "ro_fertility_2019"
SEXES = ["F", "M"]
BASE_YEARS = ["2017", "2018", "2019"]
# The centre of the base years: the table of calendar year y is the base improved (y - 2018) times.
BASE_YEAR = 2018
FERTILITY_YEAR = "2019"
AGES = 110          # single years 0 to 109; at 110 the simulation sets the probability to 1
LAST_MEASURED = 94  # demo_mlifetable has single years to 94, then 95 and over (closed at 1)
FIT_FROM = 80       # the log-odds line is drawn through ages 80 and 94
FIRST_FERTILE, LAST_FERTILE = 15, 49
PPB, PPM = 1_000_000_000, 1_000_000
START_DATE = "2021-12-01"

MANUAL = [
    {"id": "ro_sex_ratio_at_birth_2021", "value": 514_000, "unit": "millionths (share of boys)",
     "what": "share of boys among live births, 2021: 99,294 of 193,191 (Eurostat demo_fasec, as read in the "
             "research note demographics-romania-2021)",
     "source": "docs/01-research/notes/demographics-romania-2021.md", "confidence": "high"},
    {"id": "ro_retirement_age_2021", "value": {"F": 741, "M": 780}, "unit": "months of age",
     "what": "standard retirement age on 1 December 2021: men 65 years, women 61 years 9 months, rising to 63 "
             "years on 1 January 2030 (Law 263/2010, Annex 5); taken as one month of age every 97/15 months, "
             "to check against the Annex when law.json is filled",
     "source": "docs/01-research/notes/demographics-romania-2021.md#6-retirement-confidence-medium",
     "confidence": "medium"},
]
WOMEN_FINAL = (2030, 1, 756)  # (year, month, months of age): 63 years from January 2030


def age_of(code: str) -> int | None:
    """Single year of a Eurostat age code; None for groups."""
    if code == "Y_LT1":
        return 0
    if code.startswith("Y") and code[1:].isdigit():
        return int(code[1:])
    return None


def mortality(doc: dict) -> list[list[float]]:
    """[sex][age 0..109]: yearly probability of dying, the base years' average, extended above 94."""
    cells = {}
    for r in to_rows(doc):
        a = age_of(r["age"])
        if r.get("indic_de", "PROBDEATH") == "PROBDEATH" and a is not None and r["time"] in BASE_YEARS:
            cells[(r["sex"], a, r["time"])] = float(r["value"])
    table = []
    for sex in SEXES:
        row = []
        for a in range(LAST_MEASURED + 1):
            vals = [cells.get((sex, a, t)) for t in BASE_YEARS]
            if any(v is None for v in vals):
                raise CensusError(f"mortality: {sex} aged {a} is missing for one of {BASE_YEARS}")
            row.append(sum(vals) / len(vals))
        logit = [math.log(q / (1 - q)) for q in row]
        slope = (logit[LAST_MEASURED] - logit[FIT_FROM]) / (LAST_MEASURED - FIT_FROM)
        for a in range(LAST_MEASURED + 1, AGES):
            z = logit[LAST_MEASURED] + slope * (a - LAST_MEASURED)
            row.append(1 / (1 + math.exp(-z)))
        table.append(row)
    return table


def monthly_ppb(q: float) -> int:
    """The pipeline's rule: a yearly probability as the monthly one, m = 1 - (1 - q)^(1/12), in ppb."""
    return round((1 - (1 - q) ** (1 / 12)) * PPB)


def fertility(doc: dict) -> list[float]:
    """Births per woman a year at single ages 15..49; the tails folded into the end ages."""
    cells = {r["age"]: float(r["value"]) for r in to_rows(doc)
             if r.get("time", FERTILITY_YEAR) == FERTILITY_YEAR}
    rates = []
    for a in range(FIRST_FERTILE, LAST_FERTILE + 1):
        if f"Y{a}" not in cells:
            raise CensusError(f"fertility: no rate at {a}")
        rates.append(cells[f"Y{a}"])
    # A five-year group's rate is per woman of the group a year: five single years' worth of births.
    rates[0] += 5 * cells.get("Y10-14", 0.0)
    rates[-1] += cells.get("Y_GE50", 0.0)
    return rates


def month_plus(year: int, month: int, k: int) -> tuple[int, int]:
    n = year * 12 + (month - 1) + k
    return n // 12, n % 12 + 1


def retirement_schedule() -> dict:
    """Dated values: women from 741 months (December 2021) by one month of age to 756 (January 2030)."""
    start = MANUAL[1]["value"]
    y0, m0 = 2021, 12
    y1, mo1, final = WOMEN_FINAL
    span = (y1 * 12 + mo1) - (y0 * 12 + m0)
    steps = final - start["F"]
    women = [[START_DATE, start["F"]]]
    for s in range(1, steps + 1):
        y, m = month_plus(y0, m0, -(-span * s // steps))
        women.append([f"{y:04d}-{m:02d}-01", start["F"] + s])
    return {"F": women, "M": [[START_DATE, start["M"]]]}


def build(docs: dict) -> dict:
    q = mortality(docs[MORTALITY_ID])
    f = fertility(docs[FERTILITY_ID])
    return {
        "sexes": SEXES,
        "start_date": START_DATE,
        "mortality": {
            "base_years": [int(y) for y in BASE_YEARS],
            "base_year": BASE_YEAR,
            "ages": AGES,
            "extended_from": LAST_MEASURED + 1,
            "yearly_ppb": [[round(v * PPB) for v in row] for row in q],
            "monthly_ppb": [[monthly_ppb(v) for v in row] for row in q],
        },
        "fertility": {
            "year": int(FERTILITY_YEAR),
            "first_age": FIRST_FERTILE,
            "rates_ppm": [round(v * PPM) for v in f],
        },
        "sex_ratio_at_birth_ppm": MANUAL[0]["value"],
        "retirement_age_schedule": retirement_schedule(),
        "totals": {"tfr": round(sum(f), 4),
                   "life_expectancy_at_birth": [round(life_expectancy(row), 2) for row in q]},
    }


def life_expectancy(q: list[float]) -> float:
    """Period life expectancy at birth of a table (deaths at mid-year; everyone left dies at 110)."""
    alive, years = 1.0, 0.0
    for qa in q:
        years += alive * (1 - qa / 2)
        alive *= 1 - qa
    return years + alive / 2


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "demography2019_ro.json")
    a = ap.parse_args()
    docs, prov = {}, []
    for ds in (MORTALITY_ID, FERTILITY_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs[ds] = json.loads(raw)
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m = build(docs)
    m["provenance"] = {"sources": prov, "manual": MANUAL,
                       "note": "Pre-pandemic on purpose (owner, 2026-10-26): Eurostat life tables 2017-2019 "
                               "(averaged; ages 95 to 109 extended in log-odds from 80-94) and fertility rates "
                               "2019; sex ratio at birth and the retirement schedule entered by hand"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - TFR {m['totals']['tfr']}, e0 {m['totals']['life_expectancy_at_birth']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
