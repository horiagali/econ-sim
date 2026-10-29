#!/usr/bin/env python3
"""Normalise the margin file of the population generator's linked-emigrant stage (Stage L; spec
`economy/migration-remittances`, "Starting state").

Writes `data/derived/linked_emigrants2021_ro.json`:

    linked_households                   real households with one member abroad (about 15% of households)
    diaspora_total                      people born in Romania living abroad (the linked ones and the stock)
    start_year                          the year of the start date (departures of that year are under 12 months)
    departure_age_weights[sex][age]     emigrants of 2021 by sex and single year of age, 18 to 69
    departure_years[[year, weight]]     relative number of today's emigrants who left in each year
    edu_shares_ppm[edu level]           education of the emigrants (ISCED 0-2, 3, 4, 5-6, 7-8), millionths
    region_factor_ppm[[region, ppm]]    how likely a household of the region is to have a member abroad
    rural_factor_ppm                    the same for a rural household, relative to an urban one
    head_age_factor[[from, to, ppm]]    the same by the head's age in years (the last band open)

Measured: emigrants of 2021 by age and sex (Eurostat `migr_emi2`, the age profile at departure). The
rest are the educated guesses of the spec and its research note (owner's delegation, 2026-10-26):
about 1.2 million households, skewed to rural Moldavia, Maramures and the south and to older heads,
fewer in Bucharest; departures centred on 2007-2014; a diaspora of about 4 million. Low confidence;
the total remittance flow is exact whatever the count (it is fitted at load).
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
EMIGRANTS_ID = "ro_emigrants_age_2021"
SEXES = ["F", "M"]
FIRST_AGE, LAST_AGE = 18, 69
# The year of the start date (1 December 2021): a departure in it is 0 to 11 months before the start.
START_YEAR = 2021
PPM = 1_000_000

GUESSES = {
    # 15% of 7.7 million private households (spec "Starting state"; low).
    "linked_households": 1_200_000,
    # About 4 million born in Romania abroad in 2020 (research note, section 2; medium).
    "diaspora_total": 4_000_000,
    # Departures centred on 2007-2014 (EU accession and the opening of labour markets); fewer before,
    # a steady flow after (low).
    "departure_years": ([[y, 500] for y in range(2002, 2007)] + [[y, 1500] for y in range(2007, 2015)]
                        + [[y, 800] for y in range(2015, 2022)]),
    # Romanian-born adults abroad by education (ISCED 0-2, 3, 4, 5-6, 7-8): about 30% low, 50% medium,
    # 20% high; no source read, an educated guess (low).
    "edu_shares_ppm": [300_000, 430_000, 70_000, 160_000, 40_000],
    # More in Moldavia (RO21), the south (RO22, RO31, RO41) and the north-west (RO11, Maramures);
    # fewer in Bucharest-Ilfov (RO32) and the west (low).
    "region_factor_ppm": [["RO11", 1_100_000], ["RO12", 1_000_000], ["RO21", 1_500_000], ["RO22", 1_200_000],
                          ["RO31", 1_200_000], ["RO32", 400_000], ["RO41", 1_200_000], ["RO42", 900_000]],
    "rural_factor_ppm": 1_500_000,
    # Older heads: a parent whose child left (low).
    "head_age_factor": [[0, 30, 300_000], [30, 45, 800_000], [45, 70, 1_600_000], [70, None, 1_000_000]],
}


def departure_ages(doc: dict) -> list[list[int]]:
    """[sex][age - 18]: emigrants of 2021 by single year of age."""
    cells = {(r["sex"], r["age"]): int(r["value"]) for r in to_rows(doc) if r.get("time", "2021") == "2021"}
    table = []
    for sex in SEXES:
        row = []
        for a in range(FIRST_AGE, LAST_AGE + 1):
            if (sex, f"Y{a}") not in cells:
                raise CensusError(f"emigrants: no {sex} aged {a}")
            row.append(cells[(sex, f"Y{a}")])
        table.append(row)
    return table


def build(docs: dict) -> dict:
    g = GUESSES
    if sum(g["edu_shares_ppm"]) != PPM:
        raise CensusError("emigrants: education shares must sum to one million")
    return {
        "sexes": SEXES,
        "start_year": START_YEAR,
        "linked_households": g["linked_households"],
        "diaspora_total": g["diaspora_total"],
        "first_departure_age": FIRST_AGE,
        "departure_age_weights": departure_ages(docs[EMIGRANTS_ID]),
        "departure_years": g["departure_years"],
        "edu_shares_ppm": g["edu_shares_ppm"],
        "region_factor_ppm": g["region_factor_ppm"],
        "rural_factor_ppm": g["rural_factor_ppm"],
        "head_age_factor": g["head_age_factor"],
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "linked_emigrants2021_ro.json")
    a = ap.parse_args()
    raw = (a.raw / f"{EMIGRANTS_ID}.json").read_bytes()
    side = a.raw / f"{EMIGRANTS_ID}.provenance.json"
    p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": EMIGRANTS_ID}
    p["sha256"] = hashlib.sha256(raw).hexdigest()
    m = build({EMIGRANTS_ID: json.loads(raw)})
    m["provenance"] = {"sources": [p], "guesses": "see GUESSES in normalise_linked_emigrants.py (low confidence)",
                       "note": "Eurostat migr_emi2 (2021 emigrants by age and sex: the age at departure); "
                               "the count, skew, departure years, education and diaspora total are the spec's "
                               "educated guesses (economy/migration-remittances, owner's delegation 2026-10-26)"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {m['linked_households']:,} linked households -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
