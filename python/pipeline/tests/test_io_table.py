"""Input-output table: normalisation of the inter-country table (spec economy/industries)."""
import json
import re
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_census as nc  # noqa: E402
import normalise_io as nio  # noqa: E402
from test_population import jsonstat  # noqa: E402

FIXTURE = HERE / "fixtures" / "io_table2021_ro.json"
SPEC = HERE.parent.parent / "docs" / "02-design" / "economy" / "industries.md"
INDUSTRIES = ["A01", "C29"]


def docs(extra_use=0.0):
    """Two industries. A01 sells 10 to itself and 20 to C29 at home, imports 5 from Germany and 3 from
    China into C29; households buy 30 of A01 and 40 of C29; C29 exports 50 to Germany and 15 to China."""
    rows = [*INDUSTRIES, "D1", "D29X39", "B2A3G", "D21X31", "OP_RES", "OP_NRES"]
    cols = [*INDUSTRIES, "P3_S14", "P3_S15", "P3_S13", "P51G", "P5M"]
    used = {
        ("RO", "A01", "A01"): 10.0, ("RO", "A01", "C29"): 20.0 + extra_use,
        ("DE", "A01", "C29"): 5.0, ("CN", "A01", "C29"): 3.0,
        ("RO", "A01", "P3_S14"): 30.0, ("RO", "C29", "P3_S14"): 38.0, ("RO", "C29", "P3_S15"): 2.0,
        ("DE", "C29", "P51G"): 7.0,
        # value added and taxes, so that columns equal rows: A01 output 60, C29 output 105
        ("DOM", "D1", "A01"): 20.0, ("DOM", "B2A3G", "A01"): 28.0, ("DOM", "D29X39", "A01"): 1.0, ("DOM", "D21X31", "A01"): 1.0,
        ("DOM", "D1", "C29"): 40.0, ("DOM", "B2A3G", "C29"): 30.0, ("DOM", "D29X39", "C29"): 2.0, ("DOM", "D21X31", "C29"): 5.0,
        ("DOM", "D21X31", "P3_S14"): 9.0,
    }
    made = {("DE", "C29", "C29"): 30.0, ("DE", "C29", "P3_S14"): 20.0, ("CN", "C29", "P51G"): 15.0,
            ("RO", "C29", "P3_S14"): 38.0}
    used_doc = jsonstat({"c_orig": ["RO", "DE", "CN", "DOM"], "ind_ava": rows, "ind_use": cols}, used)
    made_doc = jsonstat({"c_dest": ["RO", "DE", "CN"], "ind_ava": rows, "ind_use": cols}, made)
    return used_doc, made_doc


class TestNormaliseIo(unittest.TestCase):
    def build(self, **kw):
        original = nio.CATALOGUE
        nio.CATALOGUE = [("A01", [("agri_cereals", 0.4), ("livestock_meat", 0.6)]), ("C29", [("cars", 1.0)])]
        try:
            return nio.build(*docs(**kw), 5.0)
        finally:
            nio.CATALOGUE = original

    def test_flows_are_sorted_by_origin_and_converted_to_lei(self):
        m = self.build()
        self.assertEqual([i["code"] for i in m["industries"]], INDUSTRIES)
        self.assertEqual(m["domestic"][0], [50_000_000, 100_000_000])        # 10 and 20 million euro at 5 lei
        self.assertEqual(m["imports_eu"][0][1], 25_000_000)
        self.assertEqual(m["imports_non_eu"][0][1], 15_000_000)
        self.assertEqual(m["final"]["households"]["domestic"], [150_000_000, 200_000_000])   # both household columns
        self.assertEqual(m["final"]["investment"]["imports_eu"][1], 35_000_000)
        self.assertEqual((m["exports_eu"][1], m["exports_non_eu"][1]), (250_000_000, 75_000_000))
        self.assertEqual(m["compensation"], [100_000_000, 200_000_000])
        self.assertEqual(m["product_taxes_final"]["households"], 45_000_000)

    def test_output_is_the_column_total_and_equals_the_uses(self):
        m = self.build()
        self.assertEqual(m["output"], [300_000_000, 525_000_000])
        self.assertEqual(m["totals"]["largest_imbalance"], 0.0)
        self.assertEqual(m["industry_group_of"], [0, 1])                     # agriculture, industry
        with self.assertRaises(nc.CensusError):
            self.build(extra_use=10.0)                                       # a row that no longer balances

    def test_catalogue_splits_the_output_of_a_table_industry(self):
        out = nio.catalogue_output(self.build())
        self.assertEqual(out, {"agri_cereals": 120_000_000, "livestock_meat": 180_000_000, "cars": 525_000_000})

    def test_a_catalogue_that_does_not_cover_the_table_is_rejected(self):
        with self.assertRaises(nc.CensusError):
            nio.build(*docs(), 5.0)                                          # the real catalogue has 64 industries

    def test_committed_fixture_is_balanced_and_matches_the_national_accounts(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(len(m["industries"]), 64)
        self.assertLess(m["totals"]["largest_imbalance"], 1e-6)
        # Compensation of employees, 2021: 449,335.8 million lei in the national accounts.
        self.assertAlmostEqual(m["totals"]["compensation"] / 1e9, 449.3, delta=0.5)
        by_group = [0] * 10
        for j, g in enumerate(m["industry_group_of"]):
            by_group[g] += m["compensation"][j]
        self.assertAlmostEqual(by_group[8] / 1e9, 125.3, delta=0.5)          # public administration, education, health
        self.assertAlmostEqual(by_group[0] / 1e9, 8.6, delta=0.2)            # agriculture
        self.assertGreater(m["totals"]["imports"], m["totals"]["exports"])   # Romania's trade deficit

    def test_the_catalogue_of_the_fixture_is_the_catalogue_of_the_spec(self):
        m = json.loads(FIXTURE.read_text(encoding="utf-8"))
        # The catalogue's tables come before the section on the input-output table, which has a table of its own.
        catalogue_text = SPEC.read_text(encoding="utf-8").split("## The input-output table")[0]
        in_spec = set(re.findall(r"^\| `([a-z_]+)` \|", catalogue_text, re.M))
        out = nio.catalogue_output(m)
        self.assertEqual(set(out), in_spec)
        self.assertEqual(len(out), 90)
        for entry in m["catalogue"]:
            self.assertEqual(sum(p["share"] for p in entry["parts"]), 1_000_000, entry["industry"])
        self.assertAlmostEqual(sum(out.values()) / m["totals"]["output"], 1.0, places=6)


if __name__ == "__main__":
    unittest.main()
