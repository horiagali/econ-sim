"""Opening balance sheets of the firm units (ADR-0016)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_census as nc  # noqa: E402
import normalise_firm_balance_sheets as nfb  # noqa: E402
from test_population import jsonstat  # noqa: E402

F = HERE / "fixtures"


def unit(uid, size, controller, output, value_added, wages, kind="cohort", industry="mills"):
    return {"id": uid, "industry": industry, "kind": kind, "size": size, "controller": controller, "firm_count": 2,
            "output": output, "value_added": value_added, "wages": wages}


FIRMS = {"units": [
    unit("mills.micro.domestic", "micro", "domestic", 100, 60, 40),      # surplus 20
    unit("mills.large.domestic", "large", "domestic", 300, 160, 100),    # surplus 60
    unit("mills.large.foreign", "large", "foreign", 600, 220, 100),      # surplus 120
    unit("town_halls.public", "public", "state", 500, 400, 400, kind="public", industry="public_administration"),
    unit("banks.large.domestic", "large", "domestic", 900, 500, 100, industry="banking_services"),
]}
SHEETS = {"assets": {i: {"firms": 0} for i in ("currency", "deposits", "debt_securities", "loans", "equity",
                                                "insurance_and_pensions", "other_accounts")},
          "liabilities": {i: {"firms": 0} for i in ("debt_securities", "loans", "equity", "other_accounts")}}
SHEETS["assets"]["deposits"]["firms"] = 1000
SHEETS["liabilities"]["loans"]["firms"] = 2700
SHEETS["liabilities"]["debt_securities"]["firms"] = 90
CAPITAL = jsonstat({"sector": ["S11", "S14_S15"], "asset10": ["N11N", "N111N"]},
                   {("S11", "N11N"): 0.002, ("S14_S15", "N11N"): 0.0005, ("S14_S15", "N111N"): 0.0004})


class TestFirmBalanceSheets(unittest.TestCase):
    def test_apportion_is_exact(self):
        self.assertEqual(nfb.apportion(10, [1, 1, 1]), [4, 3, 3])
        self.assertEqual(sum(nfb.apportion(1_000_003, [5, 7, 11])), 1_000_003)
        with self.assertRaises(nc.CensusError):
            nfb.apportion(10, [0, 0])

    def test_totals_are_spread_by_the_keys_over_non_financial_cohorts_only(self):
        m = nfb.build(FIRMS, SHEETS, CAPITAL)
        by = {u["id"]: u for u in m["units"]}
        self.assertEqual(set(by), {"mills.micro.domestic", "mills.large.domestic", "mills.large.foreign"})
        # Deposits follow output: 100, 300, 600 of 1,000.
        self.assertEqual([by[k]["deposits"] for k in by], [100, 300, 600])
        # Loans follow surplus times the tilt: 20 x 0.5, 60 x 1.2, 120 x 1.5 = 10, 72, 180 of 262.
        self.assertEqual(sum(u["loans"] for u in m["units"]), 2700)
        self.assertGreater(by["mills.large.foreign"]["loans"], 2 * by["mills.large.domestic"]["loans"])
        # Listed debt: large units only.
        self.assertEqual(by["mills.micro.domestic"]["debt_securities_issued"], 0)
        self.assertEqual(sum(u["debt_securities_issued"] for u in m["units"]), 90)
        # Fixed assets: corporations' 2,000 by surplus (200, 600, 1,200), plus households' 100 to the domestic micro unit.
        self.assertEqual([by[k]["fixed_assets"] for k in by], [300, 600, 1200])
        self.assertEqual(m["totals"]["fixed_assets"], 2100)

    def test_committed_fixture_adds_up_to_the_sector_balance_sheets(self):
        m = json.loads((F / "firm_balance_sheets2021_ro.json").read_text(encoding="utf-8"))
        sheets = json.loads((F / "balance_sheets2021q4_ro.json").read_text(encoding="utf-8"))
        firms = json.loads((F / "firm_units2021_ro.json").read_text(encoding="utf-8"))
        for name, side, instrument in nfb.POSITIONS:
            self.assertEqual(sum(u[name] for u in m["units"]), sheets[side][instrument]["firms"], name)
        self.assertEqual(m["totals"]["fixed_assets"],
                         m["fixed_assets_of_corporations"] + m["business_fixed_assets_of_households"])
        ids = {u["id"] for u in firms["units"]}
        self.assertTrue(all(u["id"] in ids for u in m["units"]))
        self.assertFalse(any(".public" in u["id"] or u["id"].startswith(("banking_services", "insurance_pensions"))
                             for u in m["units"]))
        self.assertTrue(all(u["fixed_assets"] > 0 and u["deposits"] > 0 for u in m["units"]))


if __name__ == "__main__":
    unittest.main()
