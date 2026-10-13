#!/usr/bin/env python3
"""Independent reference of the opening balance sheet (spec `economy/opening-balance-sheet`).

    python python/reference/opening_balance_reference.py [--sheets FILE] [--details FILE]

Maps the financial accounts of the end of 2021 (nine statistical instruments,
eight sectors; `python/pipeline/fixtures/balance_sheets2021q4_ro.json`) onto
the ledger's instruments and sectors, in bani, so that every ledger row sums to
exactly zero. Prints JSON: the matrix `balances[instrument][sector]`, what was
left out (derivatives and other accounts, by sector), the closed statistical
gaps, the opening hash, and the government debt and central bank reserves of
`sector_details2021_ro.json`.

Integer arithmetic only. Where the statistics give holders and issuers but not
who holds whose claim, the spec's rules decide; every split that is not one of
those rules is proportional, by largest remainder with ties to the lower index
(`apportion`), so the Rust builder can reproduce it to the ban.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIXTURES = HERE.parent / "pipeline" / "fixtures"
BANI_PER_LEU = 100
M64 = (1 << 64) - 1

# The ledger's sectors, in matrix column order (accounting spec, with the
# owner's decision of 2026-10-13: other financial firms are a sector of their own).
SECTORS = ["households", "firms", "banks", "pension_funds", "other_financial", "government", "central_bank",
           "rest_of_world"]
RESIDENTS = SECTORS[:-1]
ROW = "rest_of_world"
# The ledger's instruments, in matrix row order (accounting spec, section 1).
INSTRUMENTS = ["cash", "deposits", "gov_deposits_at_cb", "reserves", "bonds", "loans", "cb_advances", "equity",
               "pension_claims", "foreign_assets"]
# Statistical instruments the ledger does not hold in v1: their net position goes to net worth.
LEFT_OUT = ["derivatives", "other_accounts"]
# Residents that can hold a claim on, or owe one to, the rest of the world, per statistical
# instrument. Missing: every resident. Households neither borrow from nor lend abroad.
CROSS_BORDER = {
    "loans": {"holders": ["firms", "banks", "pension_funds", "other_financial", "government", "central_bank"],
              "issuers": ["firms", "banks", "pension_funds", "other_financial", "government"]},
}
# Where the domestic part of a statistical instrument goes in the ledger (rules 4 to 7 of the spec).
DOMESTIC = {"loans": "loans", "equity": "equity", "insurance_and_pensions": "pension_claims",
            "gold_and_sdr": "foreign_assets"}


class OpeningError(ValueError):
    """The statistics cannot be mapped (a sector or instrument is missing, or a rule does not close)."""


def apportion(total: int, targets: list[int]) -> list[int]:
    """Split `total` in proportion to `targets` by largest remainder, ties to the lower index."""
    tsum = sum(targets)
    if tsum == 0:
        out = [0] * len(targets)
        if out:
            out[0] = total
        return out
    parts = [total * t // tsum for t in targets]
    order = sorted(range(len(targets)), key=lambda i: (-(total * targets[i] % tsum), i))
    for i in order[: total - sum(parts)]:
        parts[i] += 1
    return parts


def closed(sheets: dict) -> tuple[dict, dict, dict]:
    """Assets and liabilities in bani with each instrument's statistical gap closed on the rest
    of the world (rule 1): a surplus of assets is added to its liabilities, a deficit to its assets."""
    a = {k: {s: v * BANI_PER_LEU for s, v in row.items()} for k, row in sheets["assets"].items()}
    l = {k: {s: v * BANI_PER_LEU for s, v in row.items()} for k, row in sheets["liabilities"].items()}
    gaps = {}
    for k in sheets["instruments"]:
        gap = sum(a[k].values()) - sum(l[k].values())
        gaps[k] = gap
        if gap > 0:
            l[k][ROW] += gap
        elif gap < 0:
            a[k][ROW] -= gap
    return a, l, gaps


def cross_border(a: dict, l: dict, k: str) -> tuple[dict, dict]:
    """The cross-border part of instrument `k` per resident (rule 3): the rest of the world's
    holdings are claims on residents in proportion to what they owe, and what the rest of the
    world owes is held by residents in proportion to what they hold. Returns (owed abroad,
    held abroad) per resident."""
    who = CROSS_BORDER.get(k, {})
    issuers, holders = who.get("issuers", RESIDENTS), who.get("holders", RESIDENTS)
    owed = dict(zip(issuers, apportion(a[k][ROW], [l[k][s] for s in issuers])))
    held = dict(zip(holders, apportion(l[k][ROW], [a[k][s] for s in holders])))
    for s in RESIDENTS:
        owed.setdefault(s, 0)
        held.setdefault(s, 0)
        if owed[s] > l[k][s] or held[s] > a[k][s]:
            raise OpeningError(f"{k}: the rest of the world's part of {s} exceeds what it holds or owes")
    return owed, held


def opening_balances(sheets: dict) -> dict:
    """The ledger's opening balances, `[instrument][sector]` in bani; every row sums to zero."""
    for k in ("gold_and_sdr", "currency", "deposits", "debt_securities", "loans", "equity",
              "insurance_and_pensions", *LEFT_OUT):
        if k not in sheets["instruments"]:
            raise OpeningError(f"instrument {k} is missing")
    for s in SECTORS:
        if s not in sheets["sectors"]:
            raise OpeningError(f"sector {s} is missing")
    a, l, gaps = closed(sheets)
    b = {k: {s: 0 for s in SECTORS} for k in INSTRUMENTS}

    def put(instrument: str, sector: str, amount: int) -> None:
        b[instrument][sector] += amount

    # Rule 2. Currency: the central bank's notes are cash. The rest of the world's (foreign notes)
    # are a foreign asset of their holders: the central bank's own holding, then the banks' vaults.
    k = "currency"
    foreign = l[k][ROW]
    cb_foreign = min(a[k]["central_bank"], foreign)
    bank_foreign = foreign - cb_foreign
    if bank_foreign > a[k]["banks"] or any(l[k][s] for s in RESIDENTS if s != "central_bank"):
        raise OpeningError("currency: foreign notes exceed the banks' vault cash, or a resident other than "
                           "the central bank issues currency")
    for s in RESIDENTS:
        held_foreign = {"central_bank": cb_foreign, "banks": bank_foreign}.get(s, 0)
        put("cash", s, a[k][s] - held_foreign)
        put("foreign_assets", s, held_foreign)
    put("cash", "central_bank", -l[k]["central_bank"])
    put("foreign_assets", ROW, -foreign + a[k][ROW])
    put("cash", ROW, 0)

    # Rule 4. Deposits.
    k = "deposits"
    gov_own = min(a[k]["government"], l[k]["government"])          # 4a: inside government, netted
    gov_at_cb = a[k]["government"] - gov_own                          # 4b: the rest at the central bank
    reserves = l[k]["central_bank"] - gov_at_cb                       # 4c: the rest of the CB's are reserves
    if reserves < 0 or l[k]["government"] > a[k]["government"]:
        raise OpeningError("deposits: government deposits exceed what the central bank owes")
    at_banks = {s: a[k][s] for s in ("households", "firms", "pension_funds", "other_financial", ROW)}
    interbank = l[k]["banks"] - sum(at_banks.values())                # 4d: the banks' own share of bank deposits
    bank_abroad = a[k]["banks"] - reserves - interbank                # 4e: the banks' rest is abroad
    if interbank < 0 or bank_abroad < 0:
        raise OpeningError("deposits: bank deposits do not cover the non-banks' holdings")
    if a[k]["central_bank"] + bank_abroad != l[k][ROW] or a[k][ROW] != at_banks[ROW]:
        raise OpeningError("deposits: deposits abroad do not equal what the rest of the world owes")
    if any(l[k][s] for s in ("households", "firms", "pension_funds", "other_financial")):
        raise OpeningError("deposits: a non-bank issues deposits")
    for s, v in at_banks.items():
        put("deposits", s, v)
    put("deposits", "banks", -l[k]["banks"] + interbank)
    put("gov_deposits_at_cb", "government", gov_at_cb)
    put("gov_deposits_at_cb", "central_bank", -gov_at_cb)
    put("reserves", "banks", reserves)
    put("reserves", "central_bank", -reserves)
    put("foreign_assets", "central_bank", a[k]["central_bank"])
    put("foreign_assets", "banks", bank_abroad)
    put("foreign_assets", ROW, -l[k][ROW])

    # Rule 5. Debt securities.
    k = "debt_securities"
    gov_own = min(a[k]["government"], l[k]["government"])          # 5a: inside government, netted
    cb_foreign = a[k]["central_bank"]                                 # 5b: the CB's are foreign (reserves)
    other_foreign = l[k][ROW] - cb_foreign
    row_gov = a[k][ROW]                                               # 5c: the rest of the world holds government bonds
    if other_foreign < 0 or row_gov > l[k]["government"] - gov_own:
        raise OpeningError("debt securities: the central bank's holdings exceed foreign issues, or the rest "
                           "of the world holds more government bonds than there are")
    private_issuers = ["firms", "banks", "pension_funds", "other_financial", "central_bank"]
    private = sum(l[k][s] for s in private_issuers)
    holders = ["households", "firms", "banks", "pension_funds", "other_financial"]
    foreign_holders = ["banks", "pension_funds", "other_financial"]
    held_foreign = dict(zip(foreign_holders, apportion(other_foreign, [a[k][s] for s in foreign_holders])))
    rest = {s: a[k][s] - held_foreign.get(s, 0) for s in holders}
    held_private = dict(zip(holders, apportion(private, [rest[s] for s in holders])))  # 5d
    for s in holders:
        put("foreign_assets", s, held_foreign.get(s, 0))
        put("loans", s, held_private[s])
        put("bonds", s, rest[s] - held_private[s])
    for s in private_issuers:
        put("loans", s, -l[k][s])
    put("bonds", ROW, row_gov)
    put("bonds", "government", -(l[k]["government"] - gov_own))
    put("foreign_assets", "central_bank", cb_foreign)
    put("foreign_assets", ROW, -l[k][ROW])
    if sum(b["bonds"].values()) != 0:
        raise OpeningError("debt securities: government bonds do not close")

    # Rules 3, 6 and 7. Loans, equity, insurance and pensions, gold and SDR.
    for k, target in DOMESTIC.items():
        if k == "gold_and_sdr":
            for s in SECTORS:
                put(target, s, a[k][s] - l[k][s])
            continue
        if k == "equity":
            # The rest of the world's shares in residents stay in the equity row; residents' shares abroad
            # are foreign assets.
            held = dict(zip(RESIDENTS, apportion(l[k][ROW], [a[k][s] for s in RESIDENTS])))
            for s in RESIDENTS:
                put("equity", s, a[k][s] - held[s] - l[k][s])
                put("foreign_assets", s, held[s])
            put("equity", ROW, a[k][ROW])
            put("foreign_assets", ROW, -l[k][ROW])
            continue
        owed, held = cross_border(a, l, k)
        for s in RESIDENTS:
            put(target, s, (a[k][s] - held[s]) - (l[k][s] - owed[s]))
            put("foreign_assets", s, held[s] - owed[s])
        put("foreign_assets", ROW, a[k][ROW] - l[k][ROW])

    for k in INSTRUMENTS:
        if sum(b[k].values()) != 0:
            raise OpeningError(f"ledger row {k} does not sum to zero: {sum(b[k].values())}")
    left_out = {k: {s: a[k][s] - l[k][s] for s in SECTORS} for k in LEFT_OUT}
    return {"balances": b, "left_out": left_out, "gaps": gaps}


# Rule 8: the issuer that absorbs the difference when households' own totals replace the statistics'.
RECONCILE = {"cash": "central_bank", "deposits": "banks"}


def reconcile(opening: dict, households: dict, tolerance_ppm: int = 10) -> dict:
    """Rule 8. The household column is the weighted sum of the households of the population
    (`households[instrument]`, bani, for cash and deposits). Each replaces the statistics' figure;
    the issuer absorbs the difference, which must be within `tolerance_ppm` of the figure."""
    b = {k: dict(row) for k, row in opening["balances"].items()}
    for k, issuer in RECONCILE.items():
        diff = households[k] - b[k]["households"]
        if abs(diff) * 1_000_000 > tolerance_ppm * abs(b[k]["households"]):
            raise OpeningError(f"households' {k} differs from the statistics by more than {tolerance_ppm} ppm")
        b[k]["households"] += diff
        b[k][issuer] -= diff
    return {**opening, "balances": b}


def opening_hash(balances: dict) -> int:
    """FNV-1a 64 over the number of instruments and of sectors (u32 each, little-endian), then
    every balance in bani (i64, little-endian), instrument by instrument in matrix order."""
    h = 0xCBF29CE484222325
    data = len(INSTRUMENTS).to_bytes(4, "little") + len(SECTORS).to_bytes(4, "little") + b"".join(
        balances[k][s].to_bytes(8, "little", signed=True) for k in INSTRUMENTS for s in SECTORS)
    for byte in data:
        h = ((h ^ byte) * 0x100000001B3) & M64
    return h


def check_details(details: dict, sheets: dict) -> dict:
    """Government debt by residual maturity and currency, and central bank reserves by kind,
    against the balance sheets. Returns the checks, each with `ok`."""
    debt = details["government_debt"]
    total = debt["total"]
    buckets = sum(debt["by_residual_maturity"].values())
    currencies = sum(debt["by_currency"].values())
    res = details["central_bank_reserves"]
    gold_sdr = sheets["assets"]["gold_and_sdr"]["central_bank"]
    return {
        "debt_buckets_sum_to_total": {"ok": buckets == total and currencies == total, "total": total},
        "short_term_measured": {"ok": debt["by_residual_maturity"]["up_to_1y"] == debt["short_term"]},
        "gold_and_sdr_against_sheets": {"ok": res["gold"] + res["sdr_holdings"] == gold_sdr,
                                        "gold": res["gold"], "sdr_holdings": res["sdr_holdings"]},
        "fx_against_sheets": {
            "ok": res["fx_deposits"] == sheets["assets"]["deposits"]["central_bank"]
            and res["fx_securities"] == sheets["assets"]["debt_securities"]["central_bank"]},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--sheets", type=Path, default=FIXTURES / "balance_sheets2021q4_ro.json")
    ap.add_argument("--details", type=Path, default=FIXTURES / "sector_details2021_ro.json")
    args = ap.parse_args()
    sheets = json.loads(args.sheets.read_text(encoding="utf-8"))
    details = json.loads(args.details.read_text(encoding="utf-8"))
    out = opening_balances(sheets)
    out["opening_hash"] = f"{opening_hash(out['balances']):016x}"
    out["net_financial_assets"] = {s: sum(out["balances"][k][s] for k in INSTRUMENTS) for s in SECTORS}
    out["details"] = check_details(details, sheets)
    print(json.dumps(out, indent=2))
    return 0 if all(c["ok"] for c in out["details"].values()) else 1


if __name__ == "__main__":
    sys.exit(main())
