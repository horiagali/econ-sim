"""The lock rule of scripts/traceability.py (ADR-0014, Amendment 1)."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from traceability import check, owed_ids  # noqa: E402


def spec(status: str, owed: str = "") -> str:
    return (
        f"---\nstatus: {status}\n---\n## Acceptance tests\n"
        "- [ ] **AC-X-01** `[unit]` one\n- [ ] **AC-X-02** `[sim]` two\n"
        + owed
        + "\n## Open questions\n- AC-X-01: not an owed line, it is under another heading\n"
    )


OWED_02 = "\n### Tests owed\nWaiting for the simulation.\n- AC-X-02: the full tick\n"


class LockRule(unittest.TestCase):
    def test_owed_lines_are_read_under_their_heading_only(self):
        self.assertEqual(owed_ids(spec("locked", OWED_02)), {"AC-X-02"})
        self.assertEqual(owed_ids(spec("locked")), set())
        # A line without a reason does not count.
        self.assertEqual(owed_ids("## Tests owed\n- AC-X-02:\n- AC-X-03\n"), set())

    def test_draft_spec_without_tests_only_notes(self):
        errors, notes, covered, total = check({"a.md": spec("draft")}, set(), set())
        self.assertEqual((errors, len(notes), covered, total), ([], 2, 0, 2))

    def test_locked_spec_needs_a_test_or_an_owed_line(self):
        errors, _, _, _ = check({"a.md": spec("locked")}, {"AC-X-01"}, set())
        self.assertEqual(len(errors), 1)
        self.assertIn("AC-X-02", errors[0])
        errors, notes, covered, _ = check({"a.md": spec("locked", OWED_02)}, {"AC-X-01"}, set())
        self.assertEqual((errors, covered), ([], 1))
        self.assertTrue(notes[0].endswith("(owed)"))

    def test_an_ignored_stub_is_not_a_live_test(self):
        errors, _, _, _ = check({"a.md": spec("locked")}, {"AC-X-01"}, {"AC-X-02"})
        self.assertIn("stub only", errors[0])

    def test_owed_list_must_stay_true(self):
        # Listed as owed, but the test now exists.
        errors, _, _, _ = check({"a.md": spec("locked", OWED_02)}, {"AC-X-01", "AC-X-02"}, set())
        self.assertEqual(len(errors), 1)
        self.assertIn("remove it", errors[0])
        # Listed as owed in a spec that does not define it.
        other = "---\nstatus: locked\n---\n## Tests owed\n- AC-X-02: elsewhere\n"
        errors, _, _, _ = check({"a.md": spec("draft"), "b.md": other}, {"AC-X-01"}, set())
        self.assertTrue(any("not a criterion of that spec" in e for e in errors))

    def test_stale_test_ids_fail(self):
        errors, _, _, _ = check({"a.md": spec("draft")}, {"AC-GONE-01"}, set())
        self.assertIn("AC-GONE-01", errors[0])


if __name__ == "__main__":
    unittest.main()
