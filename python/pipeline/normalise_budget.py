#!/usr/bin/env python3
"""Normalise the starting budget of Romania, 2021 (spec `economy/budget-2021`).

Writes `data/derived/budget2021_ro.json` (a copy is committed as
`fixtures/budget2021_ro.json`):

    types                      the economic types of spending, in a fixed order (ESA 2010 transactions)
    lines[]                    the budget lines (one per ministry line of the game), each with
                               its COFOG subgroups and its spending by type, lei a year
    by_function[]              the same spending by COFOG division (GF01 to GF10), for checking
    programmes[]               social benefits by programme (cash and in kind), lei a year
    employment                 public posts by sector in December 2021 and the public firm unit of each sector
    public_units[]             per public firm unit: posts, wages and employer contributions, lei a year
    published                  the published national totals the table must add up to, lei a year
    cash_execution             the Ministry of Finance's cash execution of 2021 (a second view, not summed)

Measured (Eurostat, ESA 2010, general government S13, 2021): spending by COFOG
subgroup and economic transaction (`gov_10a_exp`), the national totals of each
transaction (`gov_10a_main`), GDP (`nama_10_gdp`). Entered by hand (listed under
`provenance.manual`): occupied public posts in December 2021 and the cash
execution of the consolidated budget, from the Ministry of Finance as reported
in the press. Guessed (listed under `provenance.guesses`): how employers'
social contributions split between the CAM and imputed contributions and over
functions, and the posts of health, social care and the smaller public-order
bodies, which the published figures do not separate.

Amounts are whole lei a year. Every published total is kept exactly: where the
published subgroups do not add up to the published total (rounding to 0.1
million lei), the difference goes to the largest cell of that transaction and
is listed under `provenance.rounding`.
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
YEAR = "2021"
START_DATE = "2021-12-01"
EXP_ID = "ro_gov_expenditure_cofog_2021"
MAIN_ID = "ro_gov_main_aggregates_2021"
GDP_ID = "ro_gdp_2021"
UNIT = 1_000_000

# Economic types of spending, in the order of every `spending` vector. `wages` (D11) and
# `employer_contributions` (D12) together are compensation of employees (D1), which is what
# the COFOG table publishes; the split is computed (see `split_compensation`).
TYPES = ["wages", "employer_contributions", "goods_services", "social_benefits_cash", "social_benefits_in_kind",
         "subsidies", "interest", "investment", "non_produced_assets", "current_transfers", "capital_transfers",
         "other_taxes"]
ESA = {"goods_services": "P2", "social_benefits_cash": "D62", "social_benefits_in_kind": "D632", "subsidies": "D3",
       "interest": "D4", "investment": "P5", "non_produced_assets": "NP", "current_transfers": "D7",
       "capital_transfers": "D9", "other_taxes": "D29_D5_D8"}
# The seven headings of the task and of the game's budget screen.
CATEGORIES = {
    "wages": ["wages", "employer_contributions"],
    "goods_services": ["goods_services"],
    "social_transfers": ["social_benefits_cash", "social_benefits_in_kind"],
    "subsidies": ["subsidies"],
    "interest": ["interest"],
    "investment": ["investment", "non_produced_assets"],
    "other": ["current_transfers", "capital_transfers", "other_taxes"],
}
# National totals read from gov_10a_main (code there -> key here).
PUBLISHED_MAIN = {"TE": "TE", "D1PAY": "D1", "D11PAY": "D11", "D12PAY": "D12", "P2": "P2", "D62PAY": "D62",
                  "D632PAY": "D632", "D3PAY": "D3", "D4PAY": "D4", "P5": "P5", "NP": "NP", "D7PAY": "D7",
                  "D76PAY": "D76", "D9PAY": "D9", "D29PAY": "D29_D5_D8", "D622PAY": "D622", "P3": "P3", "TR": "TR",
                  "B9": "B9"}

# Budget lines. Research and development of every function is one line (the lever "research");
# public debt transactions are their own line (interest has no ministry). Everything else
# follows its COFOG division.
RESEARCH = ["GF0104", "GF0105", "GF0204", "GF0305", "GF0408", "GF0505", "GF0605", "GF0705", "GF0805", "GF0907",
            "GF1008"]
LINES = [  # (id, name, rule): rule is a list of subgroups, or a division less the subgroups taken elsewhere
    ("administration", "General public services", "GF01"),
    ("public_debt", "Public debt transactions", ["GF0107"]),
    ("defence", "Defence", "GF02"),
    ("police_justice", "Public order, safety and justice", "GF03"),
    ("economic_affairs", "Economic affairs other than transport", "GF04"),
    ("transport", "Transport and communications", ["GF0405", "GF0406"]),
    ("environment", "Environmental protection", "GF05"),
    ("housing_community", "Housing and community amenities", "GF06"),
    ("health", "Health", "GF07"),
    ("culture", "Recreation, culture and religion", "GF08"),
    ("education", "Education", "GF09"),
    ("social_protection", "Social protection", "GF10"),
    ("research", "Research and development", RESEARCH),
]
# The public firm unit whose employees a subgroup's wages pay (firm_units2021_ro.json).
UNIT_OF_DIVISION = {"GF02": "defence_services.public", "GF07": "health_services.public",
                    "GF09": "education_services.public"}
UNIT_OF_SUBGROUP = {"GF1001": "social_care.public", "GF1004": "social_care.public",
                    "GF1006": "social_care.public", "GF1007": "social_care.public"}
DEFAULT_UNIT = "public_administration.public"
PUBLIC_UNITS = ["defence_services.public", "education_services.public", "health_services.public",
                "public_administration.public", "social_care.public"]
# Social benefits (D62 cash, D632 in kind) by programme: (id, subgroups or a division).
PROGRAMMES = [
    ("old_age", ["GF1002"]), ("survivors", ["GF1003"]), ("sickness_disability", ["GF1001"]),
    ("family_children", ["GF1004"]), ("unemployment", ["GF1005"]), ("housing", ["GF1006"]),
    ("social_exclusion", ["GF1007"]), ("social_protection_other", ["GF1009", "GF1008"]),
    ("health_in_kind", "GF07"), ("education_grants", "GF09"),
]

# Not in any Eurostat table: from the Ministry of Finance, as reported in the press.
MANUAL = [
    {"id": "ro_public_posts_2021_12", "unit": "posts occupied",
     "what": "occupied posts in public institutions and authorities, December 2021, Ministry of Finance monthly "
             "staffing statistics",
     "values": {"total": 1_262_318, "central": 807_562, "local": 454_756,
                "central_state_budget": 595_034, "central_social_insurance": 9_244,
                "central_state_and_unemployment_budget": 43_325, "central_own_revenue": 159_959,
                "local_budgets": 278_131, "local_own_revenue": 176_625,
                "ministry_education": 290_417, "ministry_internal_affairs": 121_622, "ministry_defence": 75_855,
                "ministry_finance": 24_384, "ministry_health": 18_421, "ministry_justice": 16_926,
                "january_2021_total": 1_249_283},
     "source": "https://www.wall-street.ro/articol/Economie/281922/cati-bugetari-are-romania-si-unde-lucreaza-cei-mai-multi-dintre-acestia.html",
     "also": "https://profit.ro/stiri/ministerul-finantelor-peste-1-262-milioane-posturi-erau-ocupate-in-institutiile-si-autoritatile-publice-in-decembrie-2021-20602900",
     "confidence": "high"},
    {"id": "ro_cash_execution_2021", "unit": "lei a year",
     "what": "execution of the consolidated general budget, January to December 2021 (cash), Ministry of Finance",
     "values": {"revenue": 379_610_000_000, "expenditure": 459_630_000_000, "personnel": 111_910_000_000,
                "goods_services": 64_040_000_000, "social_assistance": 147_250_000_000,
                "subsidies": 8_650_000_000, "eu_funded_projects": 41_570_000_000, "other": 8_320_000_000,
                "memo_investment_all_kinds": 59_270_000_000},
     "source": "https://economedia.ro/?p=42711",
     "also": "https://www.digi24.ro/stiri/economie/digi-economic/deficitul-bugetar-al-romaniei-a-fost-de-80-de-miliarde-de-lei-in-2021-la-67-din-pib-mai-mic-decat-in-2020-1819083",
     "confidence": "medium"},
]

# Educated guesses (owner's rule of 2026-10-11): each is a simplification of the spec.
GUESSES = {
    # Employers' contributions (D12): the labour insurance contribution CAM (2.25% of wages, paid by
    # every employer, intra-government for the state) plus imputed contributions, the counterpart
    # of the special pensions paid from the budgets of defence and of the interior (D622).
    "cam_rate_ppm": 22_500,
    "imputed_contributions_by_division_ppm": {"GF02": 400_000, "GF03": 600_000},
    # Posts the published figures do not separate.
    "posts_health_central_outside_ministry": 60_000,   # hospitals and institutes run by ministries, own revenue
    "posts_health_local": 165_000,                     # hospitals run by counties and towns, of 176,625 own-revenue posts
    "posts_social_care_local": 130_000,                # county social-assistance directorates and personal assistants
    "posts_order_justice_other": 40_000,               # courts, prosecutors, intelligence and guard services
}


def lei(million: float) -> int:
    """Million lei with one decimal, as published, to whole lei."""
    return round(float(million) * 10) * 100_000


def apportion(total: int, weights: list[int]) -> list[int]:
    """Split `total` in proportion to non-negative `weights`; largest remainder, ties to the lower index."""
    if total < 0 or any(w < 0 for w in weights):
        raise CensusError("apportion: negative total or weight")
    s = sum(weights)
    if s == 0:
        return [0] * len(weights)
    base = [total * w // s for w in weights]
    rest = sorted(range(len(weights)), key=lambda k: (-(total * weights[k] % s), k))
    for k in rest[: total - sum(base)]:
        base[k] += 1
    return base


def cofog_cells(doc: dict) -> tuple[dict, dict, dict]:
    """Subgroup cells {(subgroup, ESA code): lei}, division totals {division: lei} and names."""
    rows = [r for r in to_rows(doc) if r["time"] == YEAR and r["sector"] == "S13" and r["unit"] == "MIO_NAC"]
    if not rows:
        raise CensusError(f"COFOG table: no rows for {YEAR}")
    names = doc["dimension"]["cofog99"]["category"].get("label", {})
    cells, totals = {}, {}
    for r in rows:
        c = r["cofog99"]
        if len(c) == 6:
            cells[(c, r["na_item"])] = lei(r["value"])
        elif len(c) == 4 and r["na_item"] == "TE":
            totals[c] = lei(r["value"])
        elif c == "TOTAL":
            totals[(c, r["na_item"])] = lei(r["value"])
    return cells, totals, names


def published_totals(main: dict, gdp: dict) -> dict:
    vals = {r["na_item"]: r["value"] for r in to_rows(main)
            if r["time"] == YEAR and r["sector"] == "S13" and r["unit"] == "MIO_NAC"}
    out = {}
    for code, key in PUBLISHED_MAIN.items():
        if code not in vals:
            raise CensusError(f"main aggregates: {code} missing for {YEAR}")
        out[key] = lei(vals[code])
    rows = [r for r in to_rows(gdp) if r["time"] == YEAR and r["na_item"] == "B1GQ"]
    if len(rows) != 1:
        raise CensusError("GDP: expected one value")
    out["GDP"] = lei(rows[0]["value"])
    return out


def exact_items(cells: dict, subgroups: list[str], published: dict, rounding: list) -> dict:
    """Per subgroup, the cell of each ESA item and of D1, adjusted so each item adds up to its national total."""
    items = {code: published[code] for code in list(ESA.values()) + ["D1"]}
    out = {s: {} for s in subgroups}
    for code, want in items.items():
        for s in subgroups:
            out[s][code] = cells.get((s, code), 0)
        diff = want - sum(out[s][code] for s in subgroups)
        if diff:
            big = max(subgroups, key=lambda s: (abs(out[s][code]), s))
            out[big][code] += diff
            rounding.append({"item": code, "subgroup": big, "lei": diff})
    return out


def split_compensation(d1: dict, d11_total: int, d12_total: int) -> tuple[dict, dict]:
    """Wages (D11) and employers' contributions (D12) per subgroup from compensation (D1).

    D12 = CAM on D11 everywhere, plus imputed contributions allocated to divisions by a guessed share
    and within a division by D1. Then D11 = (D1 - imputed) / (1 + CAM), apportioned so that it adds up
    to the published D11 exactly; D12 is the rest of D1.
    """
    cam = GUESSES["cam_rate_ppm"]
    imputed_total = d12_total - d11_total * cam // UNIT
    if imputed_total < 0:
        raise CensusError("compensation: employers' contributions below the CAM")
    shares = GUESSES["imputed_contributions_by_division_ppm"]
    divs = sorted(shares)
    by_div = apportion(imputed_total, [shares[d] for d in divs])
    subgroups = sorted(d1)
    imputed = {s: 0 for s in subgroups}
    for div, amount in zip(divs, by_div):
        members = [s for s in subgroups if s.startswith(div)]
        for s, v in zip(members, apportion(amount, [d1[s] for s in members])):
            imputed[s] = v
    raw = [max(d1[s] - imputed[s], 0) for s in subgroups]
    wages = dict(zip(subgroups, apportion(d11_total, raw)))
    contributions = {s: d1[s] - wages[s] for s in subgroups}
    if any(v < 0 for v in contributions.values()):
        raise CensusError("compensation: a subgroup's wages exceed its compensation")
    return wages, contributions


def line_of(subgroup: str) -> str:
    for line_id, _, rule in LINES:
        if isinstance(rule, list) and subgroup in rule:
            return line_id
    for line_id, _, rule in LINES:
        if isinstance(rule, str) and subgroup.startswith(rule):
            return line_id
    raise CensusError(f"budget lines: subgroup {subgroup} has no line")


def unit_of(subgroup: str) -> str:
    return UNIT_OF_SUBGROUP.get(subgroup) or UNIT_OF_DIVISION.get(subgroup[:4]) or DEFAULT_UNIT


def employment() -> dict:
    """Posts by sector in December 2021: published by ministry, the rest guessed, the total exact."""
    p = MANUAL[0]["values"]
    g = GUESSES
    if p["central"] + p["local"] != p["total"]:
        raise CensusError("public posts: central and local do not add up")
    health = p["ministry_health"] + g["posts_health_central_outside_ministry"] + g["posts_health_local"]
    order = p["ministry_internal_affairs"] + p["ministry_justice"] + g["posts_order_justice_other"]
    central_other = (p["central"] - p["ministry_education"] - p["ministry_defence"] - p["ministry_health"]
                     - g["posts_health_central_outside_ministry"] - p["ministry_internal_affairs"]
                     - p["ministry_justice"] - g["posts_order_justice_other"])
    local_other = p["local"] - g["posts_health_local"] - g["posts_social_care_local"]
    if g["posts_health_local"] > p["local_own_revenue"] or central_other < 0 or local_other < 0:
        raise CensusError("public posts: the guessed sectors exceed the published totals")
    sectors = [
        {"id": "education", "posts": p["ministry_education"], "unit": "education_services.public",
         "level": "central", "confidence": "high"},
        {"id": "health", "posts": health, "unit": "health_services.public", "level": "central and local",
         "confidence": "low"},
        {"id": "defence", "posts": p["ministry_defence"], "unit": "defence_services.public", "level": "central",
         "confidence": "high"},
        {"id": "police_justice", "posts": order, "unit": "public_administration.public", "level": "central",
         "confidence": "medium"},
        {"id": "central_administration", "posts": central_other, "unit": "public_administration.public",
         "level": "central", "confidence": "medium"},
        {"id": "local_administration", "posts": local_other, "unit": "public_administration.public",
         "level": "local", "confidence": "medium"},
        {"id": "social_care", "posts": g["posts_social_care_local"], "unit": "social_care.public",
         "level": "local", "confidence": "low"},
    ]
    if sum(s["posts"] for s in sectors) != p["total"]:
        raise CensusError("public posts: sectors do not add up to the total")
    return {"month": "2021-12", "posts": p["total"], "central": p["central"], "local": p["local"],
            "sectors": sectors}


def build(exp: dict, main: dict, gdp: dict) -> tuple[dict, list]:
    cells, div_te, names = cofog_cells(exp)
    published = published_totals(main, gdp)
    subgroups = sorted({s for s, _ in cells})
    rounding: list = []
    items = exact_items(cells, subgroups, published, rounding)
    wages, contrib = split_compensation({s: items[s]["D1"] for s in subgroups}, published["D11"], published["D12"])

    def vector(s: str) -> list[int]:
        return [wages[s], contrib[s]] + [items[s][ESA[t]] for t in TYPES[2:]]

    vec = {s: vector(s) for s in subgroups}
    add = lambda vs: [sum(col) for col in zip(*vs)] if vs else [0] * len(TYPES)  # noqa: E731

    lines = []
    for line_id, name, _ in LINES:
        members = [s for s in subgroups if line_of(s) == line_id]
        spending = add([vec[s] for s in members])
        lines.append({"id": line_id, "name": name, "cofog": members, "spending": spending, "total": sum(spending)})
    divisions = sorted({s[:4] for s in subgroups})
    by_function = []
    for div in divisions:
        spending = add([vec[s] for s in subgroups if s.startswith(div)])
        by_function.append({"cofog": div, "name": names.get(div, div), "spending": spending, "total": sum(spending),
                            "published_total": div_te.get(div)})
    programmes = []
    for pid, rule in PROGRAMMES:
        if isinstance(rule, list):
            members = [s for s in rule if s in items]
        else:
            members = [s for s in subgroups if s.startswith(rule)]
        cash = sum(items[s]["D62"] for s in members)
        kind = sum(items[s]["D632"] for s in members)
        programmes.append({"id": pid, "cofog": members, "cash": cash, "in_kind": kind, "total": cash + kind})
    emp = employment()
    units = []
    for u in PUBLIC_UNITS:
        members = [s for s in subgroups if unit_of(s) == u]
        units.append({"unit": u, "posts": sum(x["posts"] for x in emp["sectors"] if x["unit"] == u),
                      "wages": sum(wages[s] for s in members),
                      "employer_contributions": sum(contrib[s] for s in members), "cofog": members})
    by_type = add([ln["spending"] for ln in lines])
    total = sum(by_type)
    cash = MANUAL[1]["values"]
    named = ("personnel", "goods_services", "social_assistance", "subsidies", "eu_funded_projects", "other")
    m = {
        "year": int(YEAR), "start_date": START_DATE, "unit": "lei a year",
        "gdp": published["GDP"],
        "types": TYPES, "esa": {t: ESA.get(t, "D11" if t == "wages" else "D12") for t in TYPES},
        "categories": CATEGORIES,
        "lines": lines, "by_function": by_function, "programmes": programmes,
        "employment": emp, "public_units": units,
        "published": {k: v for k, v in published.items() if k != "GDP"},
        "cash_execution": {**cash, "deficit": cash["revenue"] - cash["expenditure"],
                           "not_itemised": cash["expenditure"] - sum(cash[k] for k in named),
                           "note": "cash, consolidated; 'not_itemised' is interest, other transfers (with the "
                                   "contribution to the EU budget), capital spending and the rest, which the "
                                   "press reports did not give"},
        "totals": {
            "total": total,
            "by_type": dict(zip(TYPES, by_type)),
            "by_category": {c: sum(by_type[TYPES.index(t)] for t in ts) for c, ts in CATEGORIES.items()},
            "share_of_gdp_ppm": total * UNIT // published["GDP"],
            "posts": emp["posts"],
            "wages_per_post_a_month": round(published["D11"] / emp["posts"] / 12),
        },
    }
    if total != published["TE"]:
        raise CensusError(f"budget: lines add up to {total}, published total expenditure is {published['TE']}")
    return m, rounding


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "budget2021_ro.json")
    a = ap.parse_args()
    docs, prov = {}, []
    for ds in (EXP_ID, MAIN_ID, GDP_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs[ds] = json.loads(raw)
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    m, rounding = build(docs[EXP_ID], docs[MAIN_ID], docs[GDP_ID])
    m["provenance"] = {
        "sources": prov, "manual": MANUAL, "guesses": GUESSES, "rounding": rounding,
        "note": "Eurostat government finance statistics 2021 (ESA 2010, general government): spending by COFOG "
                "subgroup and transaction, national totals, GDP; public posts and the cash execution from the "
                "Ministry of Finance as reported in the press; guesses listed under `guesses`",
    }
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    t = m["totals"]
    print(f"OK - total {t['total']:,} lei ({t['share_of_gdp_ppm'] / 1e4:.2f}% of GDP), {t['posts']:,} posts -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
