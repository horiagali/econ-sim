"""Firm units of the starting economy (ADR-0016; spec economy/industries)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_firms as nf  # noqa: E402
import normalise_io as nio  # noqa: E402
from test_population import jsonstat  # noqa: E402

F = HERE / "fixtures"
UNIT = 1_000_000
SIZE_CODES = ["TOTAL", "0-9", "10-19", "20-49", "50-249", "GE250"]


def sizes_doc():
    """C29: 100 firms (60 micro, 30 small, 10 large); value added 10, 20 and 70; wages published as a total."""
    cells = {}
    for code, values in (("ENT_NR", [100, 60, 20, 10, 8, 2]), ("EMP_NR", [1000, 100, 100, 100, 200, 500]),
                         ("AV_MEUR", [100.0, 10.0, 5.0, 15.0, 20.0, 50.0])):
        for size, v in zip(SIZE_CODES, values):
            cells[("C29", code, size)] = v
    cells[("C29", "WAGE_MEUR", "TOTAL")] = 40.0
    return jsonstat({"nace_r2": ["C29"], "indic_sbs": ["ENT_NR", "EMP_NR", "AV_MEUR", "WAGE_MEUR"], "size_emp": SIZE_CODES}, cells)


def foreign_doc(home=40.0):
    return jsonstat({"nace_r2": ["C29_C30"], "indic_sbs": ["AV_MEUR"], "c_ctrl": ["WORLD", "DOM", "DE"]},
                    {("C29_C30", "AV_MEUR", "WORLD"): 100.0, ("C29_C30", "AV_MEUR", "DOM"): home,
                     ("C29_C30", "AV_MEUR", "DE"): 60.0})


IO = {"exchange_rate": 5.0, "industries": [{"code": "C29"}], "industry_group_of": [1], "output": [1_000_000_000],
      "compensation": [200_000_000], "operating_surplus": [90_000_000], "other_taxes_less_subsidies": [10_000_000],
      "catalogue": [{"industry": "C29", "parts": [{"id": "car_parts", "share": 600_000}, {"id": "cars", "share": 400_000}]}]}


class TestNormaliseFirms(unittest.TestCase):
    def test_divisions_of_a_table_industry(self):
        self.assertEqual(nf.divisions("C10-12"), ["C10", "C11", "C12"])
        self.assertEqual(nf.divisions("J59_60"), ["J59", "J60"])
        self.assertEqual(nf.divisions("L"), ["L68"])
        self.assertEqual(nf.divisions("C29"), ["C29"])
        self.assertEqual(nf.divisions("T"), ["T"])

    def test_size_classes_and_foreign_share_are_read(self):
        s = nf.size_structure(sizes_doc())["C29"]
        self.assertEqual(s["firms"], [60, 30, 10])
        self.assertEqual(s["value_added"], [10.0, 20.0, 70.0])
        self.assertIsNone(s["wages"])                                    # published as a total only
        self.assertAlmostEqual(nf.foreign_shares(foreign_doc())["C29_C30"], 0.6)

    def test_units_split_the_industry_by_size_and_control(self):
        m = nf.build(sizes_doc(), foreign_doc(), IO)
        units = {u["id"]: u for u in m["units"]}
        # 60% of value added is foreign-controlled: it is taken from the large class (70% of value added) first.
        self.assertIn("cars.large.foreign", units)
        self.assertIn("cars.large.domestic", units)
        self.assertNotIn("cars.small.foreign", units)
        cars = [u for u in m["units"] if u["industry"] == "cars"]
        self.assertAlmostEqual(sum(u["output"] for u in cars), 400_000_000, delta=5)
        self.assertAlmostEqual(units["cars.large.foreign"]["output"], 400_000_000 * 0.6, delta=5)
        self.assertEqual(units["cars.large.foreign"]["ownership"], {"state": 0, "domestic_private": 0, "foreign": UNIT})
        self.assertEqual(units["cars.micro.domestic"]["firm_count"], 24)   # 40% of the 60 micro firms
        self.assertAlmostEqual(m["totals"]["wages"], 200_000_000, delta=10)
        self.assertEqual(m["size_structure_guessed_for"], [])
        self.assertEqual(m["industries"], [{"id": "car_parts", "industry_group": 1}, {"id": "cars", "industry_group": 1}])

    def test_little_foreign_control_does_not_split_a_cohort(self):
        m = nf.build(sizes_doc(), foreign_doc(home=95.0), IO)              # 5% foreign: below the threshold
        self.assertTrue(all(u["controller"] == "domestic" for u in m["units"]))

    def test_an_industry_without_statistics_uses_a_guess_and_says_so(self):
        m = nf.build(jsonstat({"nace_r2": ["X"], "indic_sbs": ["ENT_NR"], "size_emp": ["TOTAL"]}, {}), foreign_doc(), IO)
        self.assertEqual(m["size_structure_guessed_for"], ["C29"])
        self.assertAlmostEqual(m["totals"]["output"], 1_000_000_000, delta=10)

    def test_committed_fixture_covers_the_catalogue_and_adds_up_to_the_input_output_table(self):
        m = json.loads((F / "firm_units2021_ro.json").read_text(encoding="utf-8"))
        io = json.loads((F / "io_table2021_ro.json").read_text(encoding="utf-8"))
        want = nio.catalogue_output(io)
        got: dict[str, int] = {}
        for u in m["units"]:
            got[u["industry"]] = got.get(u["industry"], 0) + u["output"]
            self.assertEqual(sum(u["ownership"].values()), UNIT, u["id"])
            self.assertGreaterEqual(u["firm_count"], 1, u["id"])
        self.assertEqual(set(got), set(want))
        for industry, output in want.items():
            self.assertAlmostEqual(got[industry] / output, 1.0, places=3, msg=industry)
        self.assertEqual(len({u["id"] for u in m["units"]}), len(m["units"]))
        self.assertAlmostEqual(m["totals"]["wages"] / io["totals"]["compensation"], 1.0, places=3)
        public = {u["industry"] for u in m["units"] if u["kind"] == "public"}
        self.assertEqual(public, {"public_administration", "defence_services", "education_services",
                                  "health_services", "social_care"})
        by = m["totals"]["value_added_by_controller"]
        self.assertTrue(0.2 < by["foreign"] / sum(by.values()) < 0.45)     # foreign-controlled firms: about a third
        self.assertTrue(500_000 < m["totals"]["firms"] < 1_500_000)


if __name__ == "__main__":
    unittest.main()
