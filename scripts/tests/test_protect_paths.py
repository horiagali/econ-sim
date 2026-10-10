"""Unit tests for the protect-paths hook (ADR-0014 layer 1).

Run with `just hook-test` (part of `just check`).
"""
import json
import os
import subprocess
import sys
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
HOOK = os.path.join(ROOT, "scripts", "hooks", "protect_paths.py")
sys.path.insert(0, os.path.dirname(HOOK))

import protect_paths as pp  # noqa: E402

ACC = "crates/econ-mech-tax/tests/acceptance"

# Shell commands that write to a protected path: must be blocked.
BLOCKED = [
    # redirects
    "echo x > tests/golden/sim_200.hashes",
    "echo x >> tests/golden/sim_200.hashes",
    "cargo run -q -p econ-cli -- sim --ticks 200 --hashes >tests/golden/sim_200.hashes",
    "just golden-print > ./tests/golden/sim_200.hashes 2>&1",
    "just golden-print &> tests/golden/out.txt",
    "just golden-print >& tests/golden/out.txt",
    "just golden-print 2> tests/golden/err.txt",
    "just golden-print >| tests/golden/out.txt",
    "cat > schema/save.json <<EOF\n{}\nEOF",
    'echo x > "tests/golden/with space.txt"',
    "echo x > tests\\golden\\sim_200.hashes",
    "echo x > C:\\code\\elsewhere\\tests\\golden\\a.txt",
    "echo x > $CLAUDE_PROJECT_DIR/tests/golden/a.txt",
    "echo x > ${ROOT}/schema/a.json",
    # writing commands
    f"sed -i 's/a/b/' {ACC}/vat.rs",
    f"sed -i.bak -e 's/a/b/' {ACC}/vat.rs",
    f"sed -ni 's/a/b/p' {ACC}/vat.rs",
    f"sed --in-place 's/a/b/' {ACC}/vat.rs",
    f"perl -pi -e 's/a/b/' {ACC}/vat.rs",
    "rm tests/golden/sim_200.hashes",
    "rm -rf tests/golden",
    "rm -rf tests",
    "rm -rf crates/econ-mech-tax",
    "rm -rf ./*",
    "rm tests/gold*/sim_200.hashes",
    "mv tests/golden/sim_200.hashes /tmp/x",
    "mv /tmp/x tests/golden/sim_200.hashes",
    "cp /tmp/x tests/golden/sim_200.hashes",
    "cp -t schema a.json b.json",
    "cp a.json sche*",
    "touch schema/new.json",
    "mkdir -p schema/v2",
    "truncate -s 0 tests/golden/sim_200.hashes",
    "just golden-print | tee tests/golden/sim_200.hashes",
    "dd if=/dev/zero of=tests/golden/sim_200.hashes bs=1 count=1",
    "curl -s https://example.org/x -o schema/x.json",
    "curl --output=schema/x.json https://example.org/x",
    "find tests/golden -name '*.hashes' -delete",
    "find tests/golden -type f -exec rm {} \\;",
    "git checkout HEAD~1 -- tests/golden/sim_200.hashes",
    "git restore tests/golden",
    f"git rm {ACC}/vat.rs",
    "git mv schema/a.json schema/b.json",
    # compound commands and changed directory
    "cargo build && echo x > tests/golden/a.txt",
    "cargo build; rm tests/golden/a.txt",
    "cd tests/golden && rm sim_200.hashes",
    "cd tests && echo x > golden/a.txt",
    "pushd crates/econ-mech-tax; sed -i s/a/b/ tests/acceptance/vat.rs",
    "echo $(rm tests/golden/a.txt)",
    'echo "$(rm tests/golden/a.txt)"',
    "bash -c 'echo x > tests/golden/a.txt'",
    "FOO=1 sudo rm tests/golden/a.txt",
    "/usr/bin/rm tests/golden/a.txt",
    "python -c \"open('tests/golden/a.txt', 'w').write('x')\"",
    "python -c \"import pathlib; pathlib.Path('schema/a.json').write_text('x')\"",
    "node -e \"require('fs').writeFileSync('schema/a.json', '')\"",
    # PowerShell
    "Set-Content -Path tests\\golden\\sim_200.hashes -Value x",
    "set-content tests/golden/sim_200.hashes x",
    "Add-Content tests/golden/sim_200.hashes x",
    "cargo run -q -p econ-cli -- sim --hashes | Out-File -FilePath tests\\golden\\sim_200.hashes",
    "cargo run -q -p econ-cli -- sim --hashes | Out-File tests/golden/sim_200.hashes -Encoding ascii",
    "Remove-Item -Recurse -Force tests\\golden",
    "Remove-Item C:\\code\\elsewhere\\tests\\golden\\a.txt",
    "Move-Item tests\\golden\\a.txt b.txt",
    "Copy-Item a.txt -Destination schema\\a.json",
    "Copy-Item -Destination schema -Path a.json",
    "New-Item -ItemType File schema\\a.json",
    "Get-ChildItem tests\\golden | ForEach-Object { Remove-Item tests\\golden\\$_ }",
    "Set-Location tests\\golden; Remove-Item sim_200.hashes",
    "powershell -Command \"Set-Content tests/golden/a.txt x\"",
    "[IO.File]::WriteAllText('tests/golden/a.txt', 'x')",
    "Invoke-WebRequest https://example.org/x -OutFile schema\\x.json",
    "cargo run -q -p econ-cli -- sim --hashes *> tests\\golden\\a.txt",
]

# Reads and unrelated writes: must pass.
ALLOWED = [
    "cargo run -q -p econ-cli -- sim --ticks 200 --check-golden tests/golden/sim_200.hashes",
    "cargo run -q -p econ-cli -- sim --ticks 200 --check-golden tests/golden/sim_200.hashes 2>&1",
    "cargo run -q -p econ-cli -- sim --ticks 200 --check-golden tests/golden/sim_200.hashes 2>&1 | tail -5",
    "cargo run -q -p econ-cli -- sim --ticks 200 --check-golden tests/golden/sim_200.hashes > out.txt 2>&1",
    "cargo run -q -p econ-cli -- sim --ticks 200 --check-golden tests/golden/sim_200.hashes 2>/dev/null",
    "cargo run -q -p econ-cli -- sim --ticks 200 --check-golden tests\\golden\\sim_200.hashes 2>$null",
    "just golden-check 1>&2",
    "cargo run -q -p econ-cli -- sim --ticks 200 --hashes > hashes-windows.txt",
    "cat tests/golden/sim_200.hashes",
    "cat tests/golden/sim_200.hashes > /tmp/copy.txt",
    "head -3 tests/golden/sim_200.hashes | tee /tmp/head.txt",
    "wc -l < tests/golden/sim_200.hashes",
    "diff hashes-windows.txt tests/golden/sim_200.hashes",
    "tr -d '\\r' < tests/golden/sim_200.hashes > w.txt",
    f"grep -rn AC-VAT {ACC}",
    f"grep -o 'AC-VAT-0[0-9]' {ACC}/vat.rs",
    f"sed -n '1,20p' {ACC}/vat.rs",
    f"sed 's/a/b/' {ACC}/vat.rs > /tmp/vat.rs",
    "ls tests/golden schema",
    "cp tests/golden/sim_200.hashes /tmp/backup.hashes",
    "cp -r tests/golden /tmp/golden-backup",
    "find tests/golden -name '*.hashes'",
    "find . -name '*.rs' -exec grep -l acceptance {} \\;",
    "git diff main -- tests/golden",
    "git log --oneline -- tests/golden/sim_200.hashes",
    f"git add {ACC}/vat.rs",
    "git status",
    "git checkout windows-verification",
    'git commit -m "fix: hook no longer blocks `> tests/golden/x` in a message"',
    "git commit -m 'rm tests/golden is blocked; echo x > schema/a.json too'",
    "echo 'rm -rf tests/golden'",
    "git commit -F - <<'EOF'\nhook: block rm tests/golden/x\necho x > tests/golden/y\nEOF",
    "rm -rf target",
    "rm -rf crates/econ-mech-tax/target",
    "rm crates/econ-mech-tax/tests/acceptance_notes.txt",
    "rm *.txt",
    "mv a.txt b.txt",
    "sed -i 's/a/b/' crates/econ-mech-tax/src/lib.rs",
    "echo x > docs/schema/notes.md",
    "echo x > docs/tests/golden.md",
    "touch crates/econ-core/tests/golden_helpers.rs",
    "cd tests/golden && ls && cd ../.. && echo x > out.txt",
    "cargo test -p econ-mech-tax --test acceptance",
    "python scripts/traceability.py",
    "python -c \"print(open('tests/golden/sim_200.hashes').read()[:80])\"",
    "curl -s https://example.org/x -o /tmp/x.json",
    # PowerShell
    "Get-Content tests\\golden\\sim_200.hashes",
    "Get-Content tests\\golden\\sim_200.hashes | Set-Content out.txt",
    "Get-Content tests\\golden\\sim_200.hashes | Select-Object -First 3 | Out-File head.txt",
    "Copy-Item tests\\golden\\sim_200.hashes -Destination $env:TEMP\\backup.hashes",
    "Copy-Item tests\\golden\\sim_200.hashes backup.hashes",
    "Compare-Object (Get-Content a.txt) (Get-Content tests\\golden\\sim_200.hashes)",
    "Get-ChildItem tests\\golden | ForEach-Object { $_.Name }",
    "cargo run -q -p econ-cli -- sim --check-golden tests\\golden\\sim_200.hashes *>&1 | Out-String",
    "Remove-Item -Recurse -Force target",
]


class ShellCommands(unittest.TestCase):
    def writes(self, command, cwd=ROOT):
        return pp.command_writes(command, cwd, ROOT)

    def test_writes_to_protected_paths_are_blocked(self):
        for command in BLOCKED:
            with self.subTest(command=command):
                self.assertTrue(self.writes(command))

    def test_reads_and_other_writes_pass(self):
        for command in ALLOWED:
            with self.subTest(command=command):
                self.assertFalse(self.writes(command))

    def test_absolute_paths_under_the_root(self):
        win = ROOT.replace("/", "\\")
        posix = pp.norm_path(ROOT)
        self.assertTrue(self.writes(f"rm {posix}/tests/golden/sim_200.hashes"))
        self.assertTrue(self.writes(f'Remove-Item "{win}\\schema\\a.json"'))
        self.assertTrue(self.writes(f"rm -rf {posix}"))
        self.assertFalse(self.writes(f"rm -rf {posix}/target"))
        self.assertFalse(self.writes(f"cat {posix}/tests/golden/sim_200.hashes > {posix}/out.txt"))

    def test_git_bash_drive_paths(self):
        self.assertEqual(pp.norm_path("/c/code/econ-sim"), "c:/code/econ-sim")
        self.assertEqual(pp.norm_path("C:\\code\\econ-sim"), "c:/code/econ-sim")
        self.assertTrue(pp.command_writes("rm /c/code/econ-sim/schema/a.json", "c:\\code\\econ-sim", "C:\\code\\econ-sim"))

    def test_relative_paths_use_the_working_directory(self):
        self.assertTrue(self.writes("rm sim_200.hashes", cwd=os.path.join(ROOT, "tests", "golden")))
        self.assertTrue(self.writes("echo x > ../schema/a.json", cwd=os.path.join(ROOT, "crates")))
        self.assertFalse(self.writes("rm sim_200.hashes", cwd=os.path.join(ROOT, "docs")))
        self.assertFalse(self.writes("echo x > ../out/a.json", cwd=os.path.join(ROOT, "tests", "golden")))

    def test_parent_traversal_is_normalised(self):
        self.assertTrue(self.writes("rm crates/../tests/golden/sim_200.hashes"))
        self.assertFalse(self.writes("rm tests/golden/../../out.txt"))

    def test_tokenizer_keeps_quoted_operators_as_words(self):
        tokens = pp.tokenize("echo 'a > b' \"c | d\" 2>&1 > out.txt")
        self.assertEqual(
            tokens,
            [("word", "echo"), ("word", "a > b"), ("word", "c | d"), ("redir", ">&"), ("word", "1"),
             ("redir", ">"), ("word", "out.txt")],
        )


class HookProcess(unittest.TestCase):
    """Run the hook the way Claude Code does: JSON payload on stdin, exit code 2 blocks."""

    def run_hook(self, tool_input, authoring=False):
        env = dict(os.environ, CLAUDE_PROJECT_DIR=ROOT)
        env.pop("ECON_TEST_AUTHORING", None)
        if authoring:
            env["ECON_TEST_AUTHORING"] = "1"
        payload = json.dumps({"cwd": ROOT, "tool_input": tool_input})
        return subprocess.run([sys.executable, HOOK], input=payload, capture_output=True, text=True, env=env)

    def test_edit_of_a_protected_file_is_blocked(self):
        for rel in ("tests/golden/sim_200.hashes", f"{ACC}/vat.rs", "schema/save.json"):
            with self.subTest(path=rel):
                result = self.run_hook({"file_path": os.path.join(ROOT, *rel.split("/"))})
                self.assertEqual(result.returncode, 2)
                self.assertIn("BLOCKED", result.stderr)

    def test_edit_elsewhere_passes(self):
        for rel in ("crates/econ-mech-tax/src/lib.rs", "docs/roadmap.md", "scripts/hooks/protect_paths.py"):
            with self.subTest(path=rel):
                self.assertEqual(self.run_hook({"file_path": os.path.join(ROOT, *rel.split("/"))}).returncode, 0)

    def test_shell_write_is_blocked_and_read_passes(self):
        self.assertEqual(self.run_hook({"command": "echo x > tests/golden/sim_200.hashes"}).returncode, 2)
        self.assertEqual(
            self.run_hook({"command": "just golden-check 2>&1; cat tests/golden/sim_200.hashes"}).returncode, 0
        )

    def test_test_authoring_session_is_allowed(self):
        self.assertEqual(self.run_hook({"command": "rm tests/golden/sim_200.hashes"}, authoring=True).returncode, 0)

    def test_empty_or_invalid_payload_passes(self):
        result = subprocess.run([sys.executable, HOOK], input="", capture_output=True, text=True)
        self.assertEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
