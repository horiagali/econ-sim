"""Moves between counties (spec economy/migration-remittances, step 6): the normaliser on small inputs
and the committed fixture."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))

import normalise_county_moves as nc  # noqa: E402

FIXTURES = HERE / "fixtures"
HEADER = "Medii de rezidenta, Macroregiuni  regiuni de dezvoltare si judete, Ani, UM: Numar persoane, Valoare"


def csv(values: dict, year: str = "Anul 2021") -> str:
    lines = [HEADER] + [f"Total, {k}, {year}, Numar persoane, {v}" for k, v in values.items()]
    return "\n".join(lines) + "\n"


class Normalise(unittest.TestCase):
    def test_small(self):
        bal = csv({"TOTAL": 0, "Regiunea NORD-VEST": 5, "Cluj": 7, "Bihor": -7})
        sett = csv({"TOTAL": 100, "Cluj": 60, "Bihor": 40})
        m = nc.build(bal, sett, ["RO113", "RO111"])
        self.assertEqual(m["net_2021"], [7, -7])
        self.assertEqual(m["settlements_2021"], 100)
        self.assertEqual(m["seats"], ["Cluj-Napoca", "Oradea"])
        self.assertEqual(m["seat_lat_milli"], [46770, 47070])

    def test_other_years_ignored(self):
        bal = csv({"Cluj": 7, "Bihor": -7}) + "Total, Cluj, Anul 2020, Numar persoane, 99\n"
        m = nc.build(bal, csv({"TOTAL": 1}), ["RO113", "RO111"])
        self.assertEqual(m["net_2021"], [7, -7])

    def test_unbalanced_rejected(self):
        with self.assertRaises(nc.CountyMovesError):
            nc.build(csv({"Cluj": 7, "Bihor": -6}), csv({"TOTAL": 1}), ["RO113", "RO111"])

    def test_missing_county_rejected(self):
        with self.assertRaises(nc.CountyMovesError):
            nc.build(csv({"Cluj": 0}), csv({"TOTAL": 1}), ["RO113", "RO111"])


class Fixture(unittest.TestCase):
    def setUp(self):
        self.m = json.loads((FIXTURES / "county_moves2021_ro.json").read_text(encoding="utf-8"))
        self.margins = json.loads((FIXTURES / "census2021_margins_ro.json").read_text(encoding="utf-8"))

    def test_counties_in_census_order(self):
        self.assertEqual(self.m["counties"], self.margins["counties"])
        for k in ("seats", "seat_lat_milli", "seat_lon_milli", "net_2021"):
            self.assertEqual(len(self.m[k]), 42, k)

    def test_balances(self):
        net = dict(zip(self.m["counties"], self.m["net_2021"]))
        self.assertEqual(sum(net.values()), 0)
        self.assertEqual(max(net, key=net.get), "RO322", "Ilfov gains most")
        for gains in ("RO113", "RO424"):
            self.assertGreater(net[gains], 0)
        for loses in ("RO317", "RO216", "RO211"):
            self.assertLess(net[loses], 0)

    def test_seats_in_romania(self):
        for lat, lon in zip(self.m["seat_lat_milli"], self.m["seat_lon_milli"]):
            self.assertTrue(43_600 <= lat <= 48_300 and 20_200 <= lon <= 29_800, (lat, lon))

    def test_totals_and_provenance(self):
        self.assertEqual(self.m["settlements_2021"], 374_526)
        self.assertEqual(self.m["between_county_share_ppm"], 400_000)
        self.assertEqual([s["licence"] for s in self.m["provenance"]["sources"]], ["ins", "ins"])


if __name__ == "__main__":
    unittest.main()
