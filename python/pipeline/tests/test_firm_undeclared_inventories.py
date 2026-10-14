"""Undeclared sales and opening inventories of the firm units (spec economy/firm-units-undeclared-inventories)."""
import copy
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import firm_reference as ref  # noqa: E402
import normalise_firm_undeclared_inventories as nfu  # noqa: E402
from test_population import jsonstat  # noqa: E402

F = HERE / "fixtures"
U = 1_000_000


def unit(uid, industry, size, controller, output, value_added, wages, kind="cohort"):
    return {"id": uid, "industry": industry, "kind": kind, "size": size, "controller": controller,
            "output": output, "value_added": value_added, "wages": wages}


# Two industry groups: farms (group 0) and shops (group 1); a public unit in group 1.
FIRMS = {
    "industries": [{"id": "farms", "industry_group": 0}, {"id": "shops", "industry_group": 1}],
    "units": [
        unit("farms.micro.domestic", "farms", "micro", "domestic", 2_000_000, 1_000_000, 200_000),
        unit("farms.large.domestic", "farms", "large", "domestic", 3_000_000, 1_000_000, 500_000),
        unit("shops.micro.domestic", "shops", "micro", "domestic", 1_000_000, 600_000, 300_000),
        unit("shops.large.foreign", "shops", "large", "foreign", 4_000_000, 2_000_000, 1_000_000),
        unit("schools.public", "shops", "public", "state", 1_000_000, 900_000, 900_000, kind="public"),
    ],
}
MARGINS = {
    "industries": [{"id": "farms", "inventory_days": 120, "finished_share": 750_000, "household_purchases": 1_000_000},
                   {"id": "shops", "inventory_days": 150, "finished_share": 50_000, "household_purchases": 3_000_000}],
    "undeclared_wage_share": [400_000, 100_000],
    "household_business_propensity": [300_000, 150_000],
    "surplus_from_sales": [U, U],
    "undeclared_farm_income": 100_000, "undeclared_self_employment_income": 50_000, "informal_income": 200_000,
    "undeclared_value_added_share": 271_000,
    "wage_tilt": [1_150_000, 950_000, 900_000, 800_000], "informal_tilt": [U, U, U, 0],
    "profit_tilt": [1_500_000, 1_500_000, 500_000, 150_000], "profit_group_floor": 20_000,
    "max_undeclared_share": 900_000,
    "inventories": {"corporations": 1_000_000, "household_business": 300_000},
    "anchors": {"undeclared_share_of_private_value_added": 271_000, "vat_gap": 367_000},
}


class TestReference(unittest.TestCase):
    def test_apportion_with_caps_is_exact_and_respects_caps(self):
        self.assertEqual(ref.apportion_capped(10, [1, 1, 1], [100, 100, 100]), [4, 3, 3])
        self.assertEqual(ref.apportion_capped(10, [8, 1, 1], [2, 100, 100]), [2, 4, 4])
        with self.assertRaises(ref.FirmError):
            ref.apportion_capped(10, [1, 1], [2, 3])

    def test_totals_are_hit_and_bounds_hold(self):
        c = ref.undeclared_and_inventories(FIRMS, MARGINS)
        units = FIRMS["units"]
        private_va = sum(u["value_added"] for u in units if u["kind"] == "cohort")
        self.assertEqual(sum(c["undeclared_value_added"]), private_va * 271_000 // U)
        # Group undeclared wages: 40% of 700,000 and 10% of 1,300,000 (the public unit is left out).
        self.assertEqual(sum(c["undeclared_wages"][:2]), 280_000)
        self.assertEqual(sum(c["undeclared_wages"][2:]), 130_000)
        self.assertEqual(sum(c["inventory"]) + sum(c["inventory_materials"]), 1_300_000)
        for k, u in enumerate(units):
            self.assertLessEqual(c["undeclared_wages"][k], u["wages"])
            self.assertLessEqual(c["undeclared_sales"][k], u["output"])
        # The public unit has nothing; the foreign unit gets no household income, so the least.
        self.assertEqual([c[k][4] for k in ref.COLUMNS], [0] * 7)
        self.assertLess(c["undeclared_sales_share"][3], c["undeclared_sales_share"][2])

    def test_household_inventories_go_to_domestic_micro_units_only(self):
        m = copy.deepcopy(MARGINS)
        m["inventories"] = {"corporations": 0, "household_business": 300_000}
        c = ref.undeclared_and_inventories(FIRMS, m)
        held = [a + b for a, b in zip(c["inventory"], c["inventory_materials"])]
        self.assertEqual(held[1] + held[3] + held[4], 0)
        self.assertEqual(sum(held), 300_000)

    def test_errors_are_named(self):
        for change, name in (({"undeclared_value_added_share": 10_000}, "AnchorsInconsistent"),
                             ({"max_undeclared_share": 10_000}, "NoRoom"),
                             ({"wage_tilt": [U] * 3}, "ShapeMismatch"),
                             ({"undeclared_wage_share": [U + 1, 0]}, "ShareAboveOne")):
            m = copy.deepcopy(MARGINS)
            m.update(change)
            with self.assertRaises(ref.FirmError) as e:
                ref.undeclared_and_inventories(FIRMS, m)
            self.assertEqual(str(e.exception), name)

    def test_same_inputs_same_hash(self):
        a = ref.columns_hash(ref.undeclared_and_inventories(FIRMS, MARGINS))
        self.assertEqual(a, ref.columns_hash(ref.undeclared_and_inventories(FIRMS, copy.deepcopy(MARGINS))))


class TestNormaliser(unittest.TestCase):
    def test_margins_come_from_the_inputs(self):
        io = {"industry_groups": ["a", "b"], "industries": [{"code": "A01"}, {"code": "G47"}],
              "catalogue": [{"industry": "A01", "parts": [{"id": "farms", "share": U}]},
                            {"industry": "G47", "parts": [{"id": "shops", "share": U}]}],
              "final": {"households": {"domestic": [10, 30]}}}
        incomes = {"industry_groups": ["a", "b"], "farm_income": 1000, "farm_in_kind": 600, "farm_declared": 200_000,
                   "self_employment_income": 500, "self_employment_declared": 600_000, "informal_income": 70,
                   "undeclared_wage_share": [1, 2],
                   "informal_propensity": {"employee_by_industry_group": [3, 4]}}
        doc = jsonstat({"sector": ["S11", "S14_S15"], "asset10": ["N12N"]},
                       {("S11", "N12N"): 2.0, ("S14_S15", "N12N"): 0.5})
        g = nfu.GUESSES["surplus_from_sales"]
        saved = list(g)
        g[:] = [1.0, 1.0]
        try:
            m = nfu.build(FIRMS, io, incomes, doc)
        finally:
            g[:] = saved
        self.assertEqual(m["inventories"], {"corporations": 2_000_000, "household_business": 500_000})
        self.assertEqual(m["undeclared_farm_income"], 12 * 320)
        self.assertEqual(m["undeclared_self_employment_income"], 12 * 200)
        self.assertEqual(m["informal_income"], 12 * 70)
        self.assertEqual([i["inventory_days"] for i in m["industries"]], [120, 150])
        self.assertEqual([i["household_purchases"] for i in m["industries"]], [10, 30])

    def test_committed_fixture_is_the_reference_on_the_committed_inputs(self):
        firms = json.loads((F / "firm_units2021_ro.json").read_text(encoding="utf-8"))
        m = json.loads((F / "firm_undeclared_inventories2021_ro.json").read_text(encoding="utf-8"))
        c = ref.undeclared_and_inventories(firms, m)
        self.assertEqual([r["id"] for r in m["units"]], [u["id"] for u in firms["units"]])
        for f, row in enumerate(m["units"]):
            self.assertEqual({k: row[k] for k in ref.COLUMNS}, {k: c[k][f] for k in ref.COLUMNS}, row["id"])
        self.assertEqual(f"{ref.columns_hash(c):016x}", m["totals"]["hash"])
        fit = ref.fit(firms, m, c)
        self.assertEqual(fit["undeclared_value_added_total"], fit["undeclared_value_added_target"])
        self.assertEqual(fit["inventories_total"], fit["inventories_target"])
        self.assertTrue(0.15 <= fit["vat_gap_proxy"] <= 0.367)
        shares = [fit["by_class"][k]["undeclared_sales_share"] for k in ref.CLASSES]
        self.assertEqual(shares, sorted(shares, reverse=True))
        # The owner's decision of 2026-10-14: informal income lands on large domestic units too, so micro units
        # hide under half their sales, and large domestic units stay well under micro and small ones.
        self.assertEqual(m["informal_tilt"], [U, U, U, 0])
        by = {k: fit["by_class"][k]["undeclared_sales_share"] for k in ref.CLASSES}
        self.assertLess(by["micro"], 0.5)
        self.assertLess(by["large"] + 0.1, by["small"])
        self.assertLess(by["foreign"], 0.06)
        incomes = json.loads((F / "other_incomes2021_ro.json").read_text(encoding="utf-8"))
        self.assertEqual(m["undeclared_wage_share"], incomes["undeclared_wage_share"])
        self.assertTrue(all(abs(v) < 1e-5 for v in fit["undeclared_wage_share_gap_by_group"].values()))


if __name__ == "__main__":
    unittest.main()
