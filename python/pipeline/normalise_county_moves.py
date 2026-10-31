#!/usr/bin/env python3
"""Normalise the inputs of moves between counties (spec `economy/migration-remittances`, step 6).

Writes `data/derived/county_moves2021_ro.json`:

    counties[i]                 NUTS 3 codes, in the census margins' order (the scenario's counties)
    seats[i]                    each county's seat
    seat_lat_milli[i]           the seat's latitude, thousandths of a degree
    seat_lon_milli[i]           the seat's longitude, thousandths of a degree
    net_2021[i]                 the county's net balance of changes of domicile in 2021 (persons)
    settlements_2021            every change of domicile between localities in 2021 (persons)
    between_county_share_ppm    the share of those that cross a county border (millionths)

Measured: INS TEMPO POP303B (net balance of changes of domicile by county, 2021) and POP301B
(settlements of domicile, 2021). A move inside a county nets to zero in its balance, so the
balances are those of moves between counties; they sum to zero. The seats' coordinates are public
geography (rounded to about a kilometre); the share crossing a border is the spec's educated guess.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
BALANCE_ID = "ro_domicile_balance_2021"
SETTLEMENTS_ID = "ro_domicile_settlements_2021"
YEAR = "Anul 2021"

# NUTS 3 code: (INS TEMPO label, county seat, latitude, longitude). Seats' coordinates rounded to
# 0.01 degree (public geography; Ilfov's seat is Buftea).
COUNTIES = {
    "RO111": ("Bihor", "Oradea", 47.07, 21.92),
    "RO112": ("Bistrita-Nasaud", "Bistrita", 47.13, 24.49),
    "RO113": ("Cluj", "Cluj-Napoca", 46.77, 23.59),
    "RO114": ("Maramures", "Baia Mare", 47.66, 23.58),
    "RO115": ("Satu Mare", "Satu Mare", 47.79, 22.89),
    "RO116": ("Salaj", "Zalau", 47.19, 23.06),
    "RO121": ("Alba", "Alba Iulia", 46.07, 23.58),
    "RO122": ("Brasov", "Brasov", 45.65, 25.61),
    "RO123": ("Covasna", "Sfantu Gheorghe", 45.86, 25.79),
    "RO124": ("Harghita", "Miercurea Ciuc", 46.36, 25.80),
    "RO125": ("Mures", "Targu Mures", 46.54, 24.56),
    "RO126": ("Sibiu", "Sibiu", 45.79, 24.15),
    "RO211": ("Bacau", "Bacau", 46.57, 26.91),
    "RO212": ("Botosani", "Botosani", 47.75, 26.67),
    "RO213": ("Iasi", "Iasi", 47.16, 27.59),
    "RO214": ("Neamt", "Piatra Neamt", 46.93, 26.37),
    "RO215": ("Suceava", "Suceava", 47.65, 26.26),
    "RO216": ("Vaslui", "Vaslui", 46.64, 27.73),
    "RO221": ("Braila", "Braila", 45.27, 27.96),
    "RO222": ("Buzau", "Buzau", 45.15, 26.82),
    "RO223": ("Constanta", "Constanta", 44.18, 28.63),
    "RO224": ("Galati", "Galati", 45.44, 28.05),
    "RO225": ("Tulcea", "Tulcea", 45.18, 28.80),
    "RO226": ("Vrancea", "Focsani", 45.70, 27.18),
    "RO311": ("Arges", "Pitesti", 44.86, 24.87),
    "RO312": ("Calarasi", "Calarasi", 44.20, 27.33),
    "RO313": ("Dambovita", "Targoviste", 44.93, 25.46),
    "RO314": ("Giurgiu", "Giurgiu", 43.90, 25.97),
    "RO315": ("Ialomita", "Slobozia", 44.56, 27.37),
    "RO316": ("Prahova", "Ploiesti", 44.94, 26.02),
    "RO317": ("Teleorman", "Alexandria", 43.97, 25.33),
    "RO321": ("Municipiul Bucuresti", "Bucuresti", 44.43, 26.10),
    "RO322": ("Ilfov", "Buftea", 44.56, 25.95),
    "RO411": ("Dolj", "Craiova", 44.32, 23.80),
    "RO412": ("Gorj", "Targu Jiu", 45.04, 23.27),
    "RO413": ("Mehedinti", "Drobeta-Turnu Severin", 44.63, 22.66),
    "RO414": ("Olt", "Slatina", 44.43, 24.36),
    "RO415": ("Valcea", "Ramnicu Valcea", 45.10, 24.37),
    "RO421": ("Arad", "Arad", 46.18, 21.31),
    "RO422": ("Caras-Severin", "Resita", 45.30, 21.89),
    "RO423": ("Hunedoara", "Deva", 45.88, 22.90),
    "RO424": ("Timis", "Timisoara", 45.76, 21.23),
}

GUESSES = {
    # Share of changes of domicile that cross a county border (spec, step 6; educated guess, low).
    "between_county_share_ppm": 400_000,
}


class CountyMovesError(Exception):
    """The raw tables do not have what the normaliser needs."""


def rows(csv_text: str) -> dict[str, int]:
    """Area label -> value, for the year 2021, from a TEMPO pivot's CSV text."""
    out = {}
    for line in csv_text.splitlines()[1:]:
        parts = [p.strip() for p in line.split(",")]
        if len(parts) < 5 or parts[2] != YEAR:
            continue
        out[parts[1]] = int(parts[-1])
    return out


def build(balance_csv: str, settlements_csv: str, counties: list[str]) -> dict:
    bal = rows(balance_csv)
    sett = rows(settlements_csv)
    net = []
    for code in counties:
        if code not in COUNTIES:
            raise CountyMovesError(f"no INS label for county {code}")
        label = COUNTIES[code][0]
        if label not in bal:
            raise CountyMovesError(f"POP303B: no 2021 balance for {label}")
        net.append(bal[label])
    if sum(net) != 0:
        raise CountyMovesError(f"the counties' balances sum to {sum(net)}, not 0")
    if "TOTAL" not in sett:
        raise CountyMovesError("POP301B: no national total for 2021")
    return {
        "counties": counties,
        "seats": [COUNTIES[c][1] for c in counties],
        "seat_lat_milli": [round(COUNTIES[c][2] * 1000) for c in counties],
        "seat_lon_milli": [round(COUNTIES[c][3] * 1000) for c in counties],
        "net_2021": net,
        "settlements_2021": sett["TOTAL"],
        "between_county_share_ppm": GUESSES["between_county_share_ppm"],
    }


def provenance(raw: Path, ident: str, body: bytes) -> dict:
    side = raw / f"{ident}.provenance.json"
    p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ident}
    p["sha256"] = hashlib.sha256(body).hexdigest()
    return p


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--margins", type=Path, default=HERE / "data" / "derived" / "census2021_margins_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "county_moves2021_ro.json")
    a = ap.parse_args()
    bal = (a.raw / f"{BALANCE_ID}.csv").read_bytes()
    sett = (a.raw / f"{SETTLEMENTS_ID}.csv").read_bytes()
    counties = json.loads(a.margins.read_text(encoding="utf-8"))["counties"]
    m = build(bal.decode("utf-8"), sett.decode("utf-8"), counties)
    m["provenance"] = {
        "sources": [provenance(a.raw, BALANCE_ID, bal), provenance(a.raw, SETTLEMENTS_ID, sett)],
        "guesses": "see GUESSES in normalise_county_moves.py (low confidence)",
        "note": "INS TEMPO POP303B and POP301B (changes of domicile, 2021); county seats' coordinates "
                "are public geography rounded to 0.01 degree",
    }
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {len(counties)} counties, {m['settlements_2021']:,} changes of domicile -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
