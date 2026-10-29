"""Unit tests for the test-count guard (ADR-0014 layer 3).

Run with `just hook-test` (part of `just check`).
"""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "scripts"))

import test_guard as tg  # noqa: E402

ACC = "crates/econ-x/tests/acceptance/a.rs"
SRC = "crates/econ-x/src/lib.rs"

BASE_ACC = """\
#[test]
fn ac_x_01() {
    assert_eq!(1, 1);
    assert!(true);
}

#[test]
fn ac_x_02() {
    assert_ne!(1, 2);
}
"""


def snap(files, golden=(), approvals=None):
    return tg.build_snapshot(dict(files), set(golden), approvals)


def statuses(src, path=ACC):
    return {t.name: t.status for t in tg.scan_rust(path, src).tests}


class ScanTests(unittest.TestCase):
    def test_counts_tests_and_assertions(self):
        s = tg.scan_rust(ACC, BASE_ACC)
        self.assertEqual([t.name for t in s.tests], ["ac_x_01", "ac_x_02"])
        self.assertEqual(s.assertions, 3)

    def test_comments_and_strings_are_not_code(self):
        src = """\
// #[ignore]
/* #[test] fn nope() { assert!(x) } */
/// assert_eq!(a, b) in a doc comment
#[test]
fn real() {
    let s = "#[ignore] assert!(x) if false";
    let r = r#"assert_eq!("a", "b")"#;
    let c = '"';
    assert!(s.len() > 0);
}
"""
        s = tg.scan_rust(ACC, src)
        self.assertEqual([(t.name, t.status) for t in s.tests], [("real", "live")])
        self.assertEqual(s.assertions, 1)
        self.assertEqual(s.skip_markers, 0)

    def test_ignore_forms(self):
        src = """\
#[test]
#[ignore]
fn plain() {}

#[test]
#[ignore = "waits for M7"]
fn with_reason() {}

#[ignore]
#[test]
fn ignore_first() {}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release builds only"
)]
fn release_only() {}

#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn conditional() {}

#[cfg(any())]
#[test]
fn disabled() { assert!(true); }

#[test]
fn early_return() {
    return;
    assert!(false);
}

#[test]
fn early_ok() -> Result<(), E> {
    return Ok(());
}

#[test]
pub(crate) fn live() {}
"""
        self.assertEqual(statuses(src), {
            "plain": "ignored", "with_reason": "ignored", "ignore_first": "ignored",
            "release_only": "release-only", "conditional": "ignored",
            "disabled": "disabled", "early_return": "skipped", "early_ok": "skipped",
            "live": "live",
        })

    def test_skip_markers(self):
        src = "#[cfg(not(all()))]\nmod off {}\n#[test]\nfn t() { if false { assert!(x); } }\n"
        self.assertEqual(tg.scan_rust(ACC, src).skip_markers, 2)

    def test_project_assert_helpers_count_but_not_their_definitions(self):
        src = "fn assert_pays(x: i64) { assert!(x > 0); }\n#[test]\nfn t() { assert_pays(1); assert_pays(2); prop_assert_eq!(1, 1); }\n"
        self.assertEqual(tg.scan_rust(ACC, src).assertions, 4)

    def test_src_files_count_only_cfg_test_modules(self):
        src = """\
pub fn f(x: i64) { assert!(x > 0); debug_assert!(x < 9); }

#[cfg(test)]
mod tests {
    #[test]
    fn t() { assert_eq!(f(1), ()); }
}
"""
        s = tg.scan_rust(SRC, src)
        self.assertEqual(len(s.tests), 1)
        self.assertEqual(s.assertions, 1)

    def test_same_name_in_two_modules(self):
        src = "mod a { #[test] fn t() {} }\nmod b { #[test] fn t() {} }\n"
        self.assertEqual([t.id for t in tg.scan_rust(ACC, src).tests], [f"{ACC}::t", f"{ACC}::t#2"])


class CompareTests(unittest.TestCase):
    def check(self, head_src, approvals=None, base_approvals=None, golden_head=("tests/golden/a.hashes",)):
        base = snap({ACC: BASE_ACC}, ["tests/golden/a.hashes"], base_approvals)
        head = snap({ACC: head_src}, golden_head, approvals)
        return tg.compare(base, head)

    def test_no_change_passes(self):
        self.assertEqual(self.check(BASE_ACC), ([], []))

    def test_more_tests_and_assertions_pass(self):
        failures, _ = self.check(BASE_ACC + "#[test]\nfn ac_x_03() { assert!(true); }\n")
        self.assertEqual(failures, [])

    def test_removed_assertion_fails(self):
        failures, _ = self.check(BASE_ACC.replace("    assert!(true);\n", ""))
        self.assertEqual(len(failures), 1)
        self.assertIn("acceptance assertions fell from 3 to 2", failures[0])

    def test_removed_test_fails(self):
        head = BASE_ACC.split("#[test]\nfn ac_x_02")[0] + "#[test]\nfn ac_x_02() { assert_ne!(1, 2); }\n"
        self.assertEqual(self.check(head)[0], [])
        failures, notes = self.check(BASE_ACC.split("#[test]\nfn ac_x_02")[0])
        self.assertTrue(any("live acceptance tests fell from 2 to 1" in f for f in failures))
        self.assertTrue(any("removed or renamed" in n for n in notes))

    def test_new_ignore_fails(self):
        failures, _ = self.check(BASE_ACC.replace("fn ac_x_02", "#[ignore]\nfn ac_x_02"))
        self.assertTrue(any("ignored test (was live)" in f and "ac_x_02" in f for f in failures))

    def test_new_test_born_ignored_fails(self):
        failures, _ = self.check(BASE_ACC + '#[test]\n#[ignore = "later"]\nfn ac_x_03() { assert!(true); }\n')
        self.assertEqual(len(failures), 1)
        self.assertIn("ignored test (new)", failures[0])

    def test_early_return_fails(self):
        failures, _ = self.check(BASE_ACC.replace("fn ac_x_02() {", "fn ac_x_02() {\n    return;"))
        self.assertTrue(any("skipped test" in f for f in failures))

    def test_release_only_is_allowed_and_reported(self):
        head = BASE_ACC.replace("fn ac_x_02", '#[cfg_attr(debug_assertions, ignore = "release")]\nfn ac_x_02')
        failures, notes = self.check(head)
        self.assertEqual(failures, [])
        self.assertTrue(any("release-only now (was live;" in n for n in notes))

    def test_if_false_fails(self):
        failures, _ = self.check(BASE_ACC.replace("assert_ne!(1, 2);", "if false { assert_ne!(1, 2); }"))
        self.assertTrue(any("skip marker" in f for f in failures))

    def test_removed_golden_file_fails(self):
        failures, _ = self.check(BASE_ACC, golden_head=())
        self.assertTrue(any("golden file removed: tests/golden/a.hashes" in f for f in failures))

    def test_existing_ignore_is_not_flagged_again(self):
        ignored = BASE_ACC.replace("fn ac_x_02", "#[ignore]\nfn ac_x_02")
        base = snap({ACC: ignored}, [])
        head = snap({ACC: ignored, ACC.replace("a.rs", "b.rs"): ""}, [])
        self.assertEqual(tg.compare(base, head)[0], [])
        moved = snap({ACC.replace("a.rs", "b.rs"): ignored}, [])
        self.assertEqual(tg.compare(base, moved)[0], [])


APPROVAL = """\
[[approval]]
check = "assertions"
group = "acceptance"
decrease = 1
approved_by = "horia"
approved_on = 2026-10-26
reason = "a duplicate check"
"""


class ApprovalTests(unittest.TestCase):
    weaker = BASE_ACC.replace("    assert!(true);\n", "")

    def compare(self, head_src, approvals, base_approvals=None):
        return tg.compare(snap({ACC: BASE_ACC}, [], base_approvals), snap({ACC: head_src}, [], approvals))

    def test_new_approval_lets_the_decrease_through(self):
        failures, notes = self.compare(self.weaker, APPROVAL)
        self.assertEqual(failures, [])
        self.assertTrue(any(n.startswith("APPROVED (horia, 2026-10-26: a duplicate check)") for n in notes))

    def test_approval_too_small_fails(self):
        weakest = self.weaker.replace("assert_ne!(1, 2);", "")
        failures, _ = self.compare(weakest, APPROVAL)
        self.assertTrue(any("fell from 3 to 1" in f for f in failures))

    def test_old_approval_does_not_count_again(self):
        failures, _ = self.compare(self.weaker, APPROVAL, base_approvals=APPROVAL)
        self.assertEqual(len(failures), 1)

    def test_incomplete_approval_fails(self):
        failures, _ = self.compare(self.weaker, APPROVAL.replace('reason = "a duplicate check"\n', ""))
        self.assertTrue(any("reason" in f for f in failures))

    def test_ignored_test_approval_by_target(self):
        head = BASE_ACC.replace("fn ac_x_02", "#[ignore]\nfn ac_x_02")
        approval = (
            f'[[approval]]\ncheck = "ignored-test"\ntarget = "{ACC}::ac_x_02"\n'
            'approved_by = "horia"\napproved_on = "2026-10-26"\nreason = "waits for M7"\n'
        )
        self.assertEqual(self.compare(head, approval)[0], [])
        wrong = approval.replace("ac_x_02", "ac_x_01")
        failures, _ = self.compare(head, wrong)
        self.assertTrue(any("ignored test (was live)" in f for f in failures))
        self.assertTrue(any("live acceptance tests fell" in f for f in failures))


@unittest.skipUnless(shutil.which("git"), "git is needed")
class GitEndToEnd(unittest.TestCase):
    """The whole script against a throwaway repository."""

    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.old_root = tg.ROOT
        tg.ROOT = self.dir
        self.git("init", "-q")
        self.write(ACC, BASE_ACC)
        self.write("tests/golden/a.hashes", "1\n")
        self.git("add", ".")
        self.git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "base")

    def tearDown(self):
        tg.ROOT = self.old_root
        shutil.rmtree(self.dir, ignore_errors=True)

    def git(self, *args):
        subprocess.run(["git", *args], cwd=self.dir, check=True, capture_output=True)

    def write(self, rel, text):
        path = os.path.join(self.dir, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)

    def run_guard(self):
        with open(os.devnull, "w") as null:
            old = sys.stdout, sys.stderr
            sys.stdout = sys.stderr = null
            try:
                return tg.main(["--base", "HEAD"])
            finally:
                sys.stdout, sys.stderr = old

    def test_unchanged_tree_passes(self):
        self.assertEqual(self.run_guard(), 0)

    def test_deleted_assert_in_the_worktree_fails(self):
        self.write(ACC, BASE_ACC.replace("    assert!(true);\n", ""))
        self.assertEqual(self.run_guard(), 1)

    def test_approval_in_the_worktree_passes(self):
        self.write(ACC, BASE_ACC.replace("    assert!(true);\n", ""))
        self.write(tg.APPROVALS, APPROVAL)
        self.assertEqual(self.run_guard(), 0)


if __name__ == "__main__":
    unittest.main()
