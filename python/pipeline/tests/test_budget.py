"""The starting budget of 2021 (spec economy/budget-2021): the normaliser and the committed fixture."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import budget_reference as br  # noqa: E402
import normalise_budget as nb  # noqa: E402
from normalise_census import CensusError  # noqa: E402
from test_population import jsonstat  # noqa: E402

ITEMS = ["TE", "D1", "P2", "D62", "D632", "D3", "D4", "P5", "NP", "D7", "D9", "D29_D5_D8"]
CELLS = {  # million lei
    "GF0201": {"D1": 10.0, "P2": 2.0, "P5": 1.0},
    "GF0204": {"D1": 1.0},
    "GF1001": {"D1": 2.0, "D62": 5.0},
    "GF1002": {"D1": 0.5, "D62": 50.0, "D632": 1.0},
}


def exp_doc(p2_total=2.0):
    cofog = ["TOTAL", "GF02", "GF10", *CELLS]
    values = {}
    for s, row in CELLS.items():
        for item in ITEMS:
            values[("S13", "MIO_NAC", "2021", s, item)] = row.get(item, 0.0)
    values[("S13", "MIO_NAC", "2021", "GF02", "TE")] = 14.0
    values[("S13", "MIO_NAC", "2021", "GF10", "TE")] = 58.5
    values[("S13", "MIO_NAC", "2021", "TOTAL", "P2")] = p2_total
    return jsonstat({"sector": ["S13"], "unit": ["MIO_NAC"], "time": ["2021"], "cofog99": cofog, "na_item": ITEMS},
                    values)


def main_doc(p2_total=2.0, te=72.5):
    vals = {"TE": te, "D1PAY": 13.5, "D11PAY": 12.0, "D12PAY": 1.5, "P2": p2_total, "D62PAY": 55.0, "D632PAY": 1.0,
            "D3PAY": 0.0, "D4PAY": 0.0, "P5": 1.0, "NP": 0.0, "D7PAY": 0.0, "D76PAY": 0.0, "D9PAY": 0.0,
            "D29PAY": 0.0, "D622PAY": 0.0, "P3": 10.0, "TR": 60.0, "B9": -12.5}
    codes = list(vals)
    return jsonstat({"sector": ["S13"], "unit": ["MIO_NAC"], "time": ["2021"], "na_item": codes},
                    {("S13", "MIO_NAC", "2021", c): v for c, v in vals.items()})


def gdp_doc():
    return jsonstat({"time": ["2021"], "na_item": ["B1GQ"]}, {("2021", "B1GQ"): 1000.0})


class TestNormaliseBudget(unittest.TestCase):
    def test_apportion_is_exact(self):
        self.assertEqual(nb.apportion(10, [1, 1, 1]), [4, 3, 3])
        self.assertEqual(nb.apportion(0, [0, 0]), [0, 0])
        self.assertEqual(sum(nb.apportion(1_000_003, [5, 7, 11])), 1_000_003)

    def test_lines_add_up_to_the_published_totals(self):
        m, rounding = nb.build(exp_doc(), main_doc(), gdp_doc())
        self.assertEqual(rounding, [])
        self.assertEqual(m["totals"]["total"], 72_500_000)
        by_type = m["totals"]["by_type"]
        self.assertEqual(by_type["wages"], 12_000_000)
        self.assertEqual(by_type["employer_contributions"], 1_500_000)
        lines = {ln["id"]: ln for ln in m["lines"]}
        self.assertEqual(lines["research"]["cofog"], ["GF0204"])           # research of every function is one line
        self.assertEqual(lines["defence"]["cofog"], ["GF0201"])
        units = {u["unit"]: u for u in m["public_units"]}
        self.assertEqual(sum(u["wages"] for u in units.values()), 12_000_000)
        self.assertGreater(units["social_care.public"]["wages"], 0)       # GF1001 pays the social care unit
        progs = {p["id"]: p for p in m["programmes"]}
        self.assertEqual((progs["old_age"]["cash"], progs["old_age"]["in_kind"]), (50_000_000, 1_000_000))
        self.assertEqual(sum(s["posts"] for s in m["employment"]["sectors"]), m["employment"]["posts"])

    def test_rounding_goes_to_the_largest_cell_and_is_listed(self):
        m, rounding = nb.build(exp_doc(p2_total=2.1), main_doc(p2_total=2.1, te=72.6), gdp_doc())
        self.assertEqual(rounding, [{"item": "P2", "subgroup": "GF0201", "lei": 100_000}])
        self.assertEqual(m["totals"]["by_type"]["goods_services"], 2_100_000)

    def test_a_total_that_does_not_add_up_is_rejected(self):
        with self.assertRaises(CensusError):
            nb.build(exp_doc(), main_doc(te=80.0), gdp_doc())


class TestBudgetFixture(unittest.TestCase):
    def test_fixture_is_consistent_and_matches_the_reference(self):
        d = br.load()
        self.assertEqual(br.structure_errors(d["budget"]), [])
        out = br.summary(d)
        self.assertTrue(out["ok"], out["reconciliation"])

    def test_monthly_amounts_add_up_exactly(self):
        for v in (474_031_500_000, 7, -140_000_000, 0):
            parts = br.monthly(v)
            self.assertEqual(len(parts), 12)
            self.assertEqual(sum(parts), v * 100)
            self.assertLessEqual(max(parts) - min(parts), 1)


if __name__ == "__main__":
    unittest.main()
