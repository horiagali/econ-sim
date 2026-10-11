#!/usr/bin/env python3
"""Normalise Eurostat's financial accounts into the opening financial balance
sheets of the model's sectors (spec `economy/accounting`).

Writes `data/derived/balance_sheets2021q4_ro.json`, in lei, end of 2021:

    assets[instrument][sector], liabilities[instrument][sector]
    gap[instrument]    all assets less all liabilities of the instrument (zero in a closed system)

Sectors are those of the accounting spec, plus one the spec does not have yet:
households, firms, banks, other_financial (insurers, investment funds, leasing
and other lenders), pension_funds, government, central_bank, rest_of_world.
Instruments are those of the financial accounts.

Everything here is measured. The statistics do not balance to the leu; the
scenario builder has to close each instrument's gap when it builds the ledger.
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
SHEETS_ID = "ro_balance_sheets_2021q4"
SECTORS = ["households", "firms", "banks", "other_financial", "pension_funds", "government", "central_bank",
           "rest_of_world"]
# Each model sector as published sectors added and subtracted.
SECTOR_OF = {
    "households": (["S14_S15"], []),
    "firms": (["S11"], []),
    "banks": (["S121_S122_S123"], ["S121"]),
    "other_financial": (["S12"], ["S121_S122_S123", "S129"]),
    "pension_funds": (["S129"], []),
    "government": (["S13"], []),
    "central_bank": (["S121"], []),
    "rest_of_world": (["S2"], []),
}
INSTRUMENTS = {
    "gold_and_sdr": "F1", "currency": "F21", "deposits": "F22_F29", "debt_securities": "F3", "loans": "F4",
    "equity": "F5", "insurance_and_pensions": "F6", "derivatives": "F7", "other_accounts": "F8",
}
MAX_GAP = 0.02      # an instrument's assets and liabilities may differ by this share of its size


def build(doc: dict) -> dict:
    cells = {(r["sector"], r["finpos"], r["na_item"]): float(r["value"]) for r in to_rows(doc)
             if r["unit"] == "MIO_NAC"}
    if not cells:
        raise CensusError("financial accounts: no values")
    lei = lambda million: round(million * 1_000_000)  # noqa: E731

    def amount(sector: str, side: str, code: str) -> int:
        plus, minus = SECTOR_OF[sector]
        return lei(sum(cells.get((s, side, code), 0.0) for s in plus) - sum(cells.get((s, side, code), 0.0) for s in minus))

    assets = {name: {s: amount(s, "ASS", code) for s in SECTORS} for name, code in INSTRUMENTS.items()}
    liabilities = {name: {s: amount(s, "LIAB", code) for s in SECTORS} for name, code in INSTRUMENTS.items()}
    gap = {}
    for name in INSTRUMENTS:
        a, l = sum(assets[name].values()), sum(liabilities[name].values())
        if min(min(assets[name].values()), min(liabilities[name].values())) < 0:
            raise CensusError(f"financial accounts: a negative position in {name}")
        if abs(a - l) > MAX_GAP * max(a, l, 1):
            raise CensusError(f"financial accounts: {name} has assets of {a} and liabilities of {l}")
        gap[name] = a - l
    net = {s: sum(assets[i][s] - liabilities[i][s] for i in INSTRUMENTS) for s in SECTORS}
    return {"date": "2021-12-31", "unit": "lei", "sectors": SECTORS, "instruments": list(INSTRUMENTS),
            "assets": assets, "liabilities": liabilities, "gap": gap, "financial_net_worth": net}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "balance_sheets2021q4_ro.json")
    a = ap.parse_args()
    raw = (a.raw / f"{SHEETS_ID}.json").read_bytes()
    side = a.raw / f"{SHEETS_ID}.provenance.json"
    prov = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": SHEETS_ID}
    prov["sha256"] = hashlib.sha256(raw).hexdigest()
    m = build(json.loads(raw))
    m["provenance"] = {"sources": [prov], "note": "Eurostat financial balance sheets, 2021-Q4, non-consolidated"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    shown = {s: round(v / 1e9, 1) for s, v in m["financial_net_worth"].items()}
    print(f"OK - financial net worth, billion lei: {shown} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
