#!/usr/bin/env python3
"""Details of two sector balance sheets that the financial accounts do not give
(spec `economy/opening-balance-sheet`): government debt by residual maturity
and currency, and the central bank's reserves by kind.

Writes `data/derived/sector_details2021_ro.json`, in lei, end of 2021:

    government_debt      total, short_term, by_residual_maturity, by_currency
    central_bank_reserves  gold, sdr_holdings, fx_deposits, fx_securities, fx_reserves_official

There is no raw download yet: the measured figures are press releases of the
Ministry of Finance and of the National Bank of Romania, written below as
constants with their source and confidence (research note
`opening-balance-sheet-romania`). The maturity buckets above one year are an
educated guess (owner's rule of 2026-10-11): they keep the measured total and
short-term debt exactly and give a mean residual maturity of about seven years.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
EUR_RON_2021_12_31 = 4.9481  # BNR reference rate on 31 December 2021 (high)

# Government debt, EU methodology, 31 December 2021: 576.6 bn lei, of which 547.22 bn medium
# and long term (Ministry of Finance, reported by ACTMedia and Romania Insider; medium-high).
DEBT_TOTAL = 576_600_000_000
DEBT_MEDIUM_LONG = 547_220_000_000
# Residual maturity above one year, billion lei: a guess (low) with a mean of about seven years.
DEBT_BUCKETS_ABOVE_1Y = {"1_to_3y": 120_000_000_000, "3_to_5y": 110_000_000_000, "5_to_10y": 177_000_000_000}
# Currency of the debt, end of May 2021, billion lei (Ministry of Finance; medium), applied to the total.
DEBT_CURRENCY_MAY_2021 = {"lei": 249_200, "eur": 235_900, "usd": 40_800}

# Central bank, 31 December 2021 (BNR press release on international reserves; high):
# foreign currency reserves EUR 40,475 million, gold 103.6 tonnes worth EUR 5,356 million.
FX_RESERVES_EUR = 40_475_000_000
GOLD_EUR = 5_356_000_000


def apportion(total: int, targets: list[int]) -> list[int]:
    """Largest remainder, ties to the lower index."""
    tsum = sum(targets)
    parts = [total * t // tsum for t in targets]
    order = sorted(range(len(targets)), key=lambda i: (-(total * targets[i] % tsum), i))
    for i in order[: total - sum(parts)]:
        parts[i] += 1
    return parts


def build(sheets: dict) -> dict:
    short = DEBT_TOTAL - DEBT_MEDIUM_LONG
    buckets = {"up_to_1y": short, **DEBT_BUCKETS_ABOVE_1Y}
    buckets["over_10y"] = DEBT_TOTAL - sum(buckets.values())
    if buckets["over_10y"] <= 0:
        sys.exit("maturity buckets exceed the total")
    names = list(DEBT_CURRENCY_MAY_2021)
    by_currency = dict(zip(names, apportion(DEBT_TOTAL, [DEBT_CURRENCY_MAY_2021[c] for c in names])))
    gold = round(GOLD_EUR * EUR_RON_2021_12_31)
    gold_sdr = sheets["assets"]["gold_and_sdr"]["central_bank"]
    return {
        "date": "2021-12-31",
        "unit": "lei",
        "government_debt": {
            "total": DEBT_TOTAL,
            "short_term": short,
            "by_residual_maturity": buckets,
            "by_currency": by_currency,
        },
        "central_bank_reserves": {
            "gold": gold,
            "sdr_holdings": gold_sdr - gold,
            "fx_deposits": sheets["assets"]["deposits"]["central_bank"],
            "fx_securities": sheets["assets"]["debt_securities"]["central_bank"],
            "fx_reserves_official": round(FX_RESERVES_EUR * EUR_RON_2021_12_31),
        },
        "provenance": {
            "sources": [
                {"id": "ro_mof_public_debt_2021_12", "source": "Ministry of Finance (press reports)",
                 "url": "https://www.romania-insider.com/romania-public-debt-gdp-end-2021",
                 "confidence": "medium-high", "note": "total and medium/long-term debt, 31 December 2021"},
                {"id": "ro_mof_public_debt_2021_05", "source": "Ministry of Finance (press reports)",
                 "url": "https://economedia.ro/romanias-public-debt-reached-49-7-of-gdp-the-ministry-of-finance-announces.html",
                 "confidence": "medium", "note": "currency of the debt, end of May 2021"},
                {"id": "ro_bnr_reserves_2021_12", "source": "National Bank of Romania press release",
                 "url": "https://actmedia.eu/financial-and-banking/bnr-international-reserves-december-2021/95714",
                 "confidence": "high", "note": "FX reserves and gold, 31 December 2021"},
                {"id": "ro_balance_sheets_2021q4", "source": "eurostat",
                 "note": "central bank holdings of gold and SDR, deposits and debt securities (fixture "
                         "balance_sheets2021q4_ro.json)"},
            ],
            "note": "constants in normalise_sector_details.py; maturity buckets above one year are a guess (low)",
        },
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--sheets", type=Path, default=HERE / "data" / "derived" / "balance_sheets2021q4_ro.json")
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "sector_details2021_ro.json")
    a = ap.parse_args()
    out = build(json.loads(a.sheets.read_text(encoding="utf-8")))
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
