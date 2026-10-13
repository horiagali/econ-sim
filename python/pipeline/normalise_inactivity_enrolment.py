#!/usr/bin/env python3
"""Normalise the data for the kinds of inactivity and for enrolment into the
margin file of the population generator's Stage K (spec
`society/population-inactivity-enrolment`).

Writes `data/derived/inactivity_enrolment2021_ro.json`:

    inactive_kind[sex][age band][kind]      persons "other inactive" in Census 2021, by kind
                                            (homemaker, disabled, other); every row sums to the census
    enrolment[sex][age group][level]        pupils and students of the school year 2021/22 by level
    population[sex][age group]              Census 2021 residents of each age group (for enrolment rates)

Measured: the census count of persons "other inactive" by sex and age band
(from the margin file of Stage C), the census count of homemakers (INS,
entered by hand below), and enrolment by level, sex and age (Eurostat,
`educ_uoe_enra02`). Guessed: how many of the other inactive cannot work for
reasons of health, how homemakers split by sex, and how each kind is spread
over the ages (research note `inactivity-enrolment-romania`). The guesses are
the constants below, each with its reason and confidence.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from jsonstat import to_rows  # noqa: E402
from normalise_census import CensusError  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
ENROLMENT_ID = "ro_enrolment_level_age_2021_22"
SEXES = ["F", "M"]
KINDS = ["homemaker", "disabled", "other"]
LEVELS = ["pre_school", "primary", "lower_secondary", "upper_secondary", "post_secondary", "tertiary",
          "master_doctorate"]
# ISCED 2011 levels of the Eurostat table that make up each enrolment level.
LEVEL_CODES = [["ED0"], ["ED1"], ["ED2"], ["ED3"], ["ED4"], ["ED5", "ED6"], ["ED7", "ED8"]]
# Age groups of the enrolment table, [first year, first year after] (None: open). Single years up to 29,
# then the five-year groups that Eurostat publishes for every age.
ENROLMENT_AGES = ([[0, 2]] + [[y, y + 1] for y in range(2, 30)] + [[y, y + 5] for y in range(30, 65, 5)]
                  + [[65, None]])
ENROLMENT_AGE_CODES = (["Y_LT2"] + [f"Y{y}" for y in range(2, 30)] + [f"Y{y}-{y + 4}" for y in range(30, 65, 5)]
                       + ["Y_GE65"])

# --- entered by hand --------------------------------------------------------------------------
MANUAL = [
    {"id": "ro_census2021_homemakers", "value": 1_169_829, "unit": "persons",
     "what": "Census 2021 (INS, final results): economically inactive persons who are homemakers (casnice). "
             "The other inactive categories of the same table: pupils and students 3,477,027; pensioners "
             "4,288,077; supported by other persons 1,049,167; supported by the state or other organisations "
             "112,636; supported from other sources 273,215; other economic situation 498,815 "
             "(10,868,766 inactive in all)",
     "source": "https://cursdeguvernare.ro/capitalul-uman-populatia-activa-s-a-redus-cu-1-milion-in-ultimii-10-ani"
               "-cea-inactiva-se-pastreaza-a-crescut-numarul-pensionarilor-si-cel-al-elevilor-si-studentilor.html",
     "confidence": "high"},
    {"id": "ro_disabled_adults_18_64_2021", "value": 414_773, "unit": "persons",
     "what": "adults aged 18 to 64 with a disability certificate on 31 March 2021 (Ministry of Labour, as "
             "reported); 856,069 persons with disabilities in all",
     "source": "https://economedia.ro/ministerul-muncii-856-069-persoane-cu-dizabilitati-erau-inregistrate-la-31"
               "-martie-2021.html",
     "confidence": "medium"},
]

# --- educated guesses -------------------------------------------------------------------------
GUESSES = {
    # Share of homemakers who are women. The census report says "over 80%" (medium).
    "homemaker_female_share": 0.82,
    # Persons "other inactive" who cannot work for reasons of health: of the ~415,000 certified disabled
    # adults of 18 to 64, most draw an invalidity pension (counted as retired) or work; about 200,000 are
    # left, with some who have no certificate (low).
    "disabled": 200_000,
    "disabled_male_share": 0.55,  # men more often unable to work (low)
    # Relative weight of each kind (homemaker, disabled, other) in each age range, by sex, before the
    # tables are balanced to the totals: homemakers are women of working age and older women without a
    # pension; the disabled rise with age; "other" (supported by others, discouraged, other situations) is
    # mostly the young and men (low).
    "age_pattern": {
        "F": [[15, 0.05, 0.05, 0.90], [20, 0.35, 0.05, 0.60], [25, 0.60, 0.06, 0.34], [30, 0.75, 0.07, 0.18],
              [50, 0.75, 0.10, 0.15], [65, 0.75, 0.12, 0.13], [75, 0.60, 0.25, 0.15]],
        "M": [[15, 0.01, 0.06, 0.93], [20, 0.05, 0.08, 0.87], [25, 0.12, 0.10, 0.78], [30, 0.25, 0.13, 0.62],
              [50, 0.30, 0.18, 0.52], [65, 0.35, 0.20, 0.45], [75, 0.30, 0.30, 0.40]],
    },
}
IPF_SWEEPS = 200


def enrolment_table(doc: dict) -> list:
    """enrolment[sex][age group][level], from the Eurostat table of the school year 2021/22."""
    cells: dict[tuple, int] = {}
    for r in to_rows(doc):
        cells[(r["sex"], r["age"], r["isced11"])] = int(r["value"])
    if any(v for (s, a, lv), v in cells.items() if a == "UNK"):
        raise CensusError("enrolment table: pupils of unknown age; they are not handled")
    out = []
    for s in SEXES:
        rows = []
        for code in ENROLMENT_AGE_CODES:
            if (s, code, "ED0-8") not in cells:
                raise CensusError(f"enrolment table: no total for {s} {code}")
            row = [sum(cells.get((s, code, c), 0) for c in codes) for codes in LEVEL_CODES]
            if sum(row) != cells[(s, code, "ED0-8")]:
                raise CensusError(f"enrolment table: levels of {s} {code} sum to {sum(row)}, "
                                  f"total says {cells[(s, code, 'ED0-8')]}")
            rows.append(row)
        out.append(rows)
    return out


def census_tables(ea: dict) -> tuple[list, list, list]:
    """(other inactive[sex][age band], population[sex][enrolment age group], age bands) from Stage C's file."""
    if ea["sexes"] != SEXES:
        raise CensusError("education-and-activity file: sexes are not F, M")
    other = ea["activities"].index("inactive_other")
    n_years = len(ea["activity"][0][0])
    by_year = [[[sum(ea["activity"][r][s][y][k] for r in range(len(ea["regions"])))
                 for k in range(len(ea["activities"]))] for y in range(n_years)] for s in range(2)]

    def years(lo: int, hi: int | None) -> range:
        return range(lo, min(hi, n_years) if hi is not None else n_years)

    bands = ea["age_bands"]
    inactive = [[sum(by_year[s][y][other] for y in years(*b)) for b in bands] for s in range(2)]
    population = [[sum(sum(by_year[s][y]) for y in years(*g)) for g in ENROLMENT_AGES] for s in range(2)]
    return inactive, population, bands


def pattern_row(pattern: list, lo: int) -> list[float]:
    row = pattern[0][1:]
    for start, *weights in pattern:
        if lo >= start:
            row = weights
    return row


def round_table(table: list[list[float]], rows: list[int], cols: list[int]) -> list[list[int]]:
    """Integers whose rows and columns are exactly `rows` and `cols` (which sum to the same), close to `table`."""
    out = []
    for vals, total in zip(table, rows):
        floor = [int(v) for v in vals]
        left = total - sum(floor)
        for k in sorted(range(len(vals)), key=lambda k: (-(vals[k] - floor[k]), k))[:left]:
            floor[k] += 1
        out.append(floor)
    while True:
        got = [sum(row[k] for row in out) for k in range(len(cols))]
        over = [k for k in range(len(cols)) if got[k] > cols[k]]
        under = [k for k in range(len(cols)) if got[k] < cols[k]]
        if not over:
            return out
        a, b = over[0], under[0]
        r = max((r for r in range(len(out)) if out[r][a] > 0),
                key=lambda r: ((table[r][b] - out[r][b]) - (table[r][a] - out[r][a]), -r))
        out[r][a] -= 1
        out[r][b] += 1


def inactive_kind_table(inactive: list, bands: list) -> list:
    """inactive_kind[sex][band][kind]: the guessed pattern balanced to the census rows and the kind totals."""
    g = GUESSES
    homemakers = next(m["value"] for m in MANUAL if m["id"] == "ro_census2021_homemakers")
    hm_f = round(homemakers * g["homemaker_female_share"])
    dis_m = round(g["disabled"] * g["disabled_male_share"])
    kind_totals = [[hm_f, g["disabled"] - dis_m], [homemakers - hm_f, dis_m]]
    out = []
    for s, sex in enumerate(SEXES):
        rows = inactive[s]
        cols = kind_totals[s] + [sum(rows) - sum(kind_totals[s])]
        if cols[2] < 0:
            raise CensusError(f"inactive kinds: homemakers and disabled exceed the other inactive ({sex})")
        t = [[w * r for w in pattern_row(g["age_pattern"][sex], b[0])] if r else [0.0] * 3
             for b, r in zip(bands, rows)]
        for _ in range(IPF_SWEEPS):
            for k in range(3):
                c = sum(row[k] for row in t)
                if c:
                    for row in t:
                        row[k] *= cols[k] / c
            for row, r in zip(t, rows):
                rs = sum(row)
                if rs:
                    row[:] = [v * r / rs for v in row]
        out.append(round_table(t, rows, cols))
    return out


def build(enrol_doc: dict, ea: dict) -> dict:
    inactive, population, bands = census_tables(ea)
    enrolment = enrolment_table(enrol_doc)
    kinds = inactive_kind_table(inactive, bands)
    return {
        "sexes": SEXES,
        "age_bands": bands,
        "inactive_kinds": KINDS,
        "inactive_kind": kinds,
        "enrolment_levels": LEVELS,
        "enrolment_ages": ENROLMENT_AGES,
        "enrolment": enrolment,
        "population": population,
        "totals": {
            "inactive_kind": {k: sum(kinds[s][b][i] for s in range(2) for b in range(len(bands)))
                              for i, k in enumerate(KINDS)},
            "inactive_kind_female": {k: sum(kinds[0][b][i] for b in range(len(bands))) for i, k in enumerate(KINDS)},
            "enrolment": {lv: sum(enrolment[s][a][i] for s in range(2) for a in range(len(ENROLMENT_AGES)))
                          for i, lv in enumerate(LEVELS)},
        },
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--attributes", type=Path, default=HERE / "data" / "derived" / "census2021_edu_activity_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "inactivity_enrolment2021_ro.json")
    a = ap.parse_args()
    raw = (a.raw / f"{ENROLMENT_ID}.json").read_bytes()
    side = a.raw / f"{ENROLMENT_ID}.provenance.json"
    p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ENROLMENT_ID}
    p["sha256"] = hashlib.sha256(raw).hexdigest()
    m = build(json.loads(raw), json.loads(a.attributes.read_text(encoding="utf-8")))
    m["provenance"] = {
        "sources": [p], "manual": MANUAL, "guesses": GUESSES,
        "derived_from": "census2021_edu_activity_ro.json (Census 2021: persons other inactive by sex and age band; "
                        "residents by sex and year of age)",
        "note": "Eurostat UOE enrolment of the school year 2021/22 (labelled 2022; ages on 1 January 2022); "
                "census homemakers and certified disabled adults entered by hand; the number unable to work, "
                "the split by sex and the age pattern of each kind are educated guesses listed under `guesses` "
                "(research note inactivity-enrolment-romania)"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
