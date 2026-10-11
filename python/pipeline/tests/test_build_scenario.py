"""The interim scenario builder and its consistency report (ADR-0018)."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "reference"))

import build_scenario as bs  # noqa: E402


class TestBuildScenario(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.docs, cls.input_hashes = bs.load(bs.FIXTURES)
        cls.stages = bs.run_stages(cls.docs, 1000, 42)
        cls.report = bs.report(cls.docs, cls.stages, 1000)

    def test_every_fixture_is_an_input_with_a_hash(self):
        self.assertEqual(set(self.input_hashes), set(bs.INPUTS.values()))
        self.assertTrue(all(len(h) == 64 for h in self.input_hashes.values()))

    def test_stage_hashes_are_those_of_the_specs(self):
        # The reference hashes at 1:1000 that each stage's spec lists for its golden test.
        self.assertEqual(bs.stage_hashes(self.stages), {
            "population": "247112c87ae60cf2", "attributes": "8a0509e8ebc90647", "jobs": "7e3499aa1c44e7f1",
            "housing": "a4b8ac6c2c2bb24e", "wages": "e5c524e4e785f24f", "pensions": "963e671ef7035b92",
            "incomes": "40c861af66b29b98", "wealth": "8b7d9a0ea0d3c373", "employers": "40dc87a87a0e31af",
        })

    def test_the_parts_agree(self):
        self.assertTrue(self.report["ok"], {k: v for k, v in self.report["checks"].items() if not v["ok"]})
        failed = {name for name, c in self.report["checks"].items() if not c["ok"]}
        self.assertLessEqual(failed, set(bs.KNOWN_GAPS))
        self.assertEqual(len(self.report["checks"]), 12)

    def test_a_known_gap_is_reported_with_its_reason(self):
        for name in bs.KNOWN_GAPS:
            self.assertIn("known_gap", self.report["checks"][name])

    def test_a_broken_part_is_caught(self):
        # Halve every wage: the wage bill, and with it the report, must fail.
        broken = dict(self.stages, wage=[v // 2 for v in self.stages["wage"]])
        report = bs.report(self.docs, broken, 1000)
        self.assertFalse(report["checks"]["wage_bill"]["ok"])
        self.assertFalse(report["ok"])


if __name__ == "__main__":
    unittest.main()
