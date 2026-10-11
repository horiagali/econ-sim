#!/usr/bin/env python3
"""Normalise Eurostat's inter-country input-output table (FIGARO) into the
input-output table of Romania for 2021 (spec `economy/industries`).

Writes `data/derived/io_table2021_ro.json`, in lei, for the 64 industries of
the table (NACE A64):

    domestic[i][j], imports_eu[i][j], imports_non_eu[i][j]   what industry j buys from industry i, by origin
    final[use].domestic / imports_eu / imports_non_eu [i]    households, government, investment, inventories
    exports_eu[i], exports_non_eu[i]                         output of industry i used abroad
    compensation, other_taxes_less_subsidies, operating_surplus, product_taxes [j]
    output[j]                                                column total = row total
    industry_group_of[j]                                     the ten industry groups of the population stages
    catalogue                                                the 90 industries of the game inside the 64 (guessed shares)

The table is measured. The split of a table industry into the finer
industries of the game's catalogue is an educated guess (`CATALOGUE`), to be
replaced by structural business statistics and energy statistics when the firm
table is built.
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
from normalise_census_jobs import GROUP_SECTIONS, INDUSTRY_GROUPS  # noqa: E402

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
USED_ID = "figaro_2021_used_in_ro"
MADE_ID = "figaro_2021_made_in_ro"
RATE_ID = "ro_exchange_rate_2021"
HOME = "RO"
EU = {"BE", "BG", "CZ", "DK", "DE", "EE", "IE", "EL", "ES", "FR", "HR", "IT", "CY", "LV", "LT", "LU", "HU", "MT",
      "NL", "AT", "PL", "PT", "RO", "SI", "SK", "FI", "SE"}
FINAL_USES = {"households": ["P3_S14", "P3_S15"], "government": ["P3_S13"], "investment": ["P51G"],
              "inventories": ["P5M"]}
FINAL_CODES = {code for codes in FINAL_USES.values() for code in codes}
VALUE_ADDED = {"compensation": "D1", "other_taxes_less_subsidies": "D29X39", "operating_surplus": "B2A3G"}
PRODUCT_TAXES = "D21X31"
ADJUSTMENTS = {"OP_RES", "OP_NRES"}       # purchases abroad by residents and at home by non-residents
MAX_IMBALANCE = 0.001                     # row total against column total, relative

# The game's 90 industries inside the table's 64: (table industry, [(catalogue id, share of its output)]).
# Shares are educated guesses where a table industry is split.
CATALOGUE = [
    ("A01", [("agri_cereals", 0.30), ("agri_oilseeds", 0.12), ("agri_fruit_veg", 0.22), ("agri_vineyards", 0.04),
             ("livestock_meat", 0.20), ("livestock_dairy", 0.12)]),
    ("A02", [("forestry", 1.0)]),
    ("A03", [("fishing", 1.0)]),
    ("B", [("coal_mining", 0.12), ("crude_oil", 0.30), ("natural_gas", 0.40), ("metal_ores", 0.05),
           ("uranium_fuel", 0.01), ("quarrying", 0.12)]),
    ("C10-12", [("food_meat_dairy", 0.30), ("food_bakery_milling", 0.20), ("food_other", 0.22), ("beverages", 0.20),
                ("tobacco", 0.08)]),
    ("C13-15", [("textiles_apparel", 0.75), ("leather_footwear", 0.25)]),
    ("C16", [("wood_products", 1.0)]),
    ("C17", [("paper_printing", 1.0)]),
    ("C18", [("paper_printing", 1.0)]),
    ("C19", [("refined_petroleum", 1.0)]),
    ("C20", [("fertilisers", 0.20), ("chemicals", 0.80)]),
    ("C21", [("pharmaceuticals", 1.0)]),
    ("C22", [("plastics_rubber", 1.0)]),
    ("C23", [("building_materials", 1.0)]),
    ("C24", [("steel", 0.65), ("nonferrous_metals", 0.35)]),
    ("C25", [("metal_products", 0.90), ("defence_equipment", 0.10)]),
    ("C26", [("electronic_components", 0.45), ("computers_comms", 0.40), ("consumer_electronics", 0.15)]),
    ("C27", [("electrical_equipment", 0.80), ("consumer_electronics", 0.20)]),
    ("C28", [("industrial_machinery", 0.75), ("agri_construction_machinery", 0.25)]),
    ("C29", [("car_parts", 0.60), ("cars", 0.33), ("trucks_buses", 0.07)]),
    ("C30", [("rail_equipment", 0.30), ("aircraft", 0.30), ("shipbuilding", 0.40)]),
    ("C31_32", [("furniture", 0.65), ("medical_devices", 0.15), ("other_manufacturing", 0.20)]),
    ("C33", [("industrial_machinery", 1.0)]),
    ("D35", [("power_nuclear", 0.08), ("power_hydro", 0.12), ("power_coal", 0.08), ("power_gas", 0.07),
             ("power_wind", 0.05), ("power_solar", 0.01), ("power_biomass", 0.01), ("power_storage", 0.005),
             ("power_grid", 0.17), ("power_supply", 0.255), ("gas_heat_supply", 0.15)]),
    ("E36", [("water_waste", 1.0)]),
    ("E37-39", [("water_waste", 1.0)]),
    ("F", [("construction_buildings", 0.60), ("construction_civil", 0.40)]),
    ("G45", [("vehicle_trade_repair", 1.0)]),
    ("G46", [("wholesale_trade", 1.0)]),
    ("G47", [("retail_trade", 1.0)]),
    ("H49", [("road_freight", 0.60), ("rail_transport", 0.12), ("passenger_transport", 0.28)]),
    ("H50", [("water_transport", 1.0)]),
    ("H51", [("air_transport", 1.0)]),
    ("H52", [("post_logistics", 1.0)]),
    ("H53", [("post_logistics", 1.0)]),
    ("I", [("hotels_tourism", 0.30), ("restaurants", 0.70)]),
    ("J58", [("media_publishing", 1.0)]),
    ("J59_60", [("media_publishing", 1.0)]),
    ("J61", [("telecom_services", 1.0)]),
    ("J62_63", [("it_software", 0.90), ("data_centres", 0.10)]),
    ("K64", [("banking_services", 1.0)]),
    ("K65", [("insurance_pensions", 1.0)]),
    ("K66", [("banking_services", 0.50), ("insurance_pensions", 0.50)]),
    ("L", [("real_estate", 1.0)]),
    ("M69_70", [("professional_services", 1.0)]),
    ("M71", [("engineering_rnd", 1.0)]),
    ("M72", [("engineering_rnd", 1.0)]),
    ("M73", [("professional_services", 1.0)]),
    ("M74_75", [("professional_services", 1.0)]),
    ("N77", [("admin_support", 1.0)]),
    ("N78", [("admin_support", 1.0)]),
    ("N79", [("admin_support", 1.0)]),
    ("N80-82", [("admin_support", 1.0)]),
    ("O84", [("public_administration", 0.85), ("defence_services", 0.15)]),
    ("P85", [("education_services", 1.0)]),
    ("Q86", [("health_services", 1.0)]),
    ("Q87_88", [("social_care", 1.0)]),
    ("R90-92", [("recreation_culture", 1.0)]),
    ("R93", [("recreation_culture", 1.0)]),
    ("S94", [("personal_services", 1.0)]),
    ("S95", [("personal_services", 1.0)]),
    ("S96", [("personal_services", 1.0)]),
    ("T", [("personal_services", 1.0)]),
    ("U", [("personal_services", 1.0)]),
]


def group_of(code: str) -> int:
    """The industry group (of the population stages) of a table industry, by its NACE section letter."""
    for g, sections in enumerate(GROUP_SECTIONS):
        if code[0] in sections:
            return g
    raise CensusError(f"industry {code} is in no industry group")


def bloc(country: str) -> str:
    return "domestic" if country == HOME else "imports_eu" if country in EU else "imports_non_eu"


def build(used_doc: dict, made_doc: dict, rate: float) -> dict:
    """`used_doc`: everything used in Romania by origin; `made_doc`: Romania's output by destination;
    `rate`: lei per euro. Amounts become whole lei."""
    index = used_doc["dimension"]["ind_ava"]["category"]["index"]
    labels = used_doc["dimension"]["ind_ava"]["category"].get("label", {})
    special = set(VALUE_ADDED.values()) | {PRODUCT_TAXES} | ADJUSTMENTS
    codes = [c for c in sorted(index, key=index.get) if c not in special]
    pos = {c: k for k, c in enumerate(codes)}
    n = len(codes)
    if n == 0:
        raise CensusError("input-output table: no industries")
    lei = lambda mio_eur: round(float(mio_eur) * rate * 1_000_000)  # noqa: E731
    zeros = lambda: [0] * n  # noqa: E731
    flows = {b: [zeros() for _ in range(n)] for b in ("domestic", "imports_eu", "imports_non_eu")}
    final = {use: {b: zeros() for b in flows} for use in FINAL_USES}
    use_of = {code: use for use, cs in FINAL_USES.items() for code in cs}
    rows = {name: zeros() for name in VALUE_ADDED}
    product_taxes = zeros()
    product_taxes_final = {use: 0 for use in FINAL_USES}
    for r in to_rows(used_doc):
        i, u, v = r["ind_ava"], r["ind_use"], lei(r["value"])
        if i in pos and u in pos:
            flows[bloc(r["c_orig"])][pos[i]][pos[u]] += v
        elif i in pos and u in use_of:
            final[use_of[u]][bloc(r["c_orig"])][pos[i]] += v
        elif i == PRODUCT_TAXES and u in pos:
            product_taxes[pos[u]] += v
        elif i == PRODUCT_TAXES and u in use_of:
            product_taxes_final[use_of[u]] += v
        else:
            for name, code in VALUE_ADDED.items():
                if i == code and u in pos:
                    rows[name][pos[u]] += v
    exports = {"exports_eu": zeros(), "exports_non_eu": zeros()}
    for r in to_rows(made_doc):
        if r["c_dest"] != HOME and r["ind_ava"] in pos:
            key = "exports_eu" if r["c_dest"] in EU else "exports_non_eu"
            exports[key][pos[r["ind_ava"]]] += lei(r["value"])
    output = [sum(flows[b][i][j] for b in flows for i in range(n)) + product_taxes[j]
              + sum(rows[name][j] for name in rows) for j in range(n)]
    used = [sum(flows["domestic"][i]) + sum(final[use]["domestic"][i] for use in final)
            + exports["exports_eu"][i] + exports["exports_non_eu"][i] for i in range(n)]
    worst = max((abs(a - b) / max(a, 1) for a, b in zip(output, used)), default=0.0)
    if worst > MAX_IMBALANCE:
        raise CensusError(f"input-output table: an industry's uses differ from its output by {worst:.2%}")
    parents = [code for code, _ in CATALOGUE]
    if sorted(parents) != sorted(codes):
        raise CensusError("the catalogue's table industries are not those of the input-output table")
    for code, parts in CATALOGUE:
        if abs(sum(share for _, share in parts) - 1.0) > 1e-9:
            raise CensusError(f"catalogue shares of {code} do not sum to 1")
    value_added = sum(sum(rows[name]) for name in rows)
    return {
        "year": 2021, "unit": "lei", "exchange_rate": rate,
        "industries": [{"code": c, "label": labels.get(c, c)} for c in codes],
        "industry_groups": INDUSTRY_GROUPS,
        "industry_group_of": [group_of(c) for c in codes],
        **flows,
        "final": final,
        **exports,
        **rows,
        "product_taxes": product_taxes,
        "product_taxes_final": product_taxes_final,
        "output": output,
        "catalogue": [{"industry": code, "parts": [{"id": cid, "share": round(share * 1_000_000)} for cid, share in parts]}
                      for code, parts in CATALOGUE],
        "totals": {
            "output": sum(output), "value_added": value_added, "compensation": sum(rows["compensation"]),
            "household_consumption": sum(sum(final["households"][b]) for b in flows) + product_taxes_final["households"],
            "exports": sum(exports["exports_eu"]) + sum(exports["exports_non_eu"]),
            "imports": sum(sum(row) for b in ("imports_eu", "imports_non_eu") for row in flows[b])
            + sum(sum(final[use][b]) for use in final for b in ("imports_eu", "imports_non_eu")),
            "largest_imbalance": round(worst, 8),
        },
    }


def catalogue_output(table: dict) -> dict:
    """Output of each catalogue industry, in lei: its shares of the table industries it is part of."""
    pos = {ind["code"]: k for k, ind in enumerate(table["industries"])}
    out: dict[str, int] = {}
    for entry in table["catalogue"]:
        for part in entry["parts"]:
            out[part["id"]] = out.get(part["id"], 0) + table["output"][pos[entry["industry"]]] * part["share"] // 1_000_000
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=RAW)
    ap.add_argument("--out", type=Path, default=HERE / "data" / "derived" / "io_table2021_ro.json")
    a = ap.parse_args()
    docs, prov = {}, []
    for ds in (USED_ID, MADE_ID, RATE_ID):
        raw = (a.raw / f"{ds}.json").read_bytes()
        docs[ds] = json.loads(raw)
        side = a.raw / f"{ds}.provenance.json"
        p = json.loads(side.read_text(encoding="utf-8")) if side.exists() else {"id": ds}
        p["sha256"] = hashlib.sha256(raw).hexdigest()
        prov.append(p)
    rates = [float(r["value"]) for r in to_rows(docs[RATE_ID])]
    if len(rates) != 1:
        raise CensusError("exchange rate: expected one value")
    m = build(docs[USED_ID], docs[MADE_ID], rates[0])
    m["provenance"] = {"sources": prov,
                       "note": "Eurostat FIGARO inter-country input-output table 2021, industry by industry, "
                               "converted to lei at the year's average rate; the catalogue shares are educated guesses"}
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(json.dumps(m, separators=(",", ":")) + "\n", encoding="utf-8")
    print(f"OK - {len(m['industries'])} industries, {m['totals']} -> {a.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
