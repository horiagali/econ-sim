"""Opening financial balance sheets of the sectors (spec economy/accounting)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import normalise_balance_sheets as nb  # noqa: E402
import normalise_census as nc  # noqa: E402
from test_population import jsonstat  # noqa: E402

F = HERE / "fixtures"
PUBLISHED = ["S11", "S12", "S121_S122_S123", "S121", "S129", "S13", "S14_S15", "S2"]


def doc(deposits_of_households=100.0):
    """Deposits: households hold 100 and firms 50; the banks owe 140 and the central bank 10."""
    cells = {
        ("S14_S15", "ASS", "F22_F29"): deposits_of_households, ("S11", "ASS", "F22_F29"): 50.0,
        ("S12", "LIAB", "F22_F29"): 150.0, ("S121_S122_S123", "LIAB", "F22_F29"): 150.0,
        ("S121", "LIAB", "F22_F29"): 10.0,
        # loans: all financial corporations lend 80, of which monetary institutions 60, none by the central bank
        ("S12", "ASS", "F4"): 80.0, ("S121_S122_S123", "ASS", "F4"): 60.0, ("S14_S15", "LIAB", "F4"): 80.0,
    }
    return jsonstat({"sector": PUBLISHED, "finpos": ["ASS", "LIAB"], "na_item": list(nb.INSTRUMENTS.values()),
                     "unit": ["MIO_NAC"]},
                    {(s, side, item, "MIO_NAC"): v for (s, side, item), v in cells.items()})


class TestNormaliseBalanceSheets(unittest.TestCase):
    def test_model_sectors_are_built_from_published_ones(self):
        m = nb.build(doc())
        self.assertEqual(m["assets"]["deposits"]["households"], 100_000_000)
        self.assertEqual(m["liabilities"]["deposits"]["banks"], 140_000_000)          # monetary institutions less the central bank
        self.assertEqual(m["liabilities"]["deposits"]["central_bank"], 10_000_000)
        self.assertEqual(m["assets"]["loans"]["banks"], 60_000_000)
        self.assertEqual(m["assets"]["loans"]["other_financial"], 20_000_000)         # the rest of the financial sector
        self.assertEqual(m["gap"]["deposits"], 0)
        self.assertEqual(m["financial_net_worth"]["households"], 20_000_000)
        self.assertEqual(sum(m["financial_net_worth"].values()), 0)

    def test_an_instrument_that_does_not_balance_is_rejected(self):
        with self.assertRaises(nc.CensusError):
            nb.build(doc(deposits_of_households=300.0))

    def test_committed_fixture_balances_and_agrees_with_the_wealth_stage(self):
        m = json.loads((F / "balance_sheets2021q4_ro.json").read_text(encoding="utf-8"))
        for name in m["instruments"]:
            size = sum(m["assets"][name].values())
            self.assertLessEqual(abs(m["gap"][name]), 0.001 * size, name)
        # Financial net worth of all sectors, the rest of the world included, is zero up to the gaps.
        self.assertLess(abs(sum(m["financial_net_worth"].values())), 1_000_000_000)
        wealth = json.loads((F / "household_wealth2021_ro.json").read_text(encoding="utf-8"))["totals"]
        self.assertEqual(m["assets"]["currency"]["households"], wealth["cash"])
        self.assertEqual(m["assets"]["deposits"]["households"], wealth["deposits"])
        self.assertEqual(m["liabilities"]["loans"]["households"], wealth["housing_loans"] + wealth["consumer_loans"])
        # Government debt securities: about 488 billion lei; Romania owes the rest of the world on balance.
        self.assertAlmostEqual(m["liabilities"]["debt_securities"]["government"] / 1e9, 488.4, delta=0.5)
        self.assertGreater(m["financial_net_worth"]["rest_of_world"], 0)


if __name__ == "__main__":
    unittest.main()
