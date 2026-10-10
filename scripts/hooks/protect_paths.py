#!/usr/bin/env python3
"""Claude Code PreToolUse hook: block agent edits to protected test paths.

ADR-0014 layer 1. Golden runs, acceptance tests and schemas may only be
changed in a deliberate test-authoring session. To allow it for one session,
start Claude Code with the environment variable ECON_TEST_AUTHORING=1.

Edit/Write tools are checked by file path. Shell commands (Bash, PowerShell)
are blocked only when a *write* targets a protected path: a redirect into it,
or a writing command (`sed -i`, `mv`, `cp`, `rm`, `Set-Content`, `Out-File`,
…) with it as the target. Reads (`cat`, `--check-golden tests/golden/…`) and
`2>&1` pass. The shell check is a heuristic: variables, scripts and tools that
write on their own are not seen, so layer 2 (PR review) stays the real guard.

Exit code 2 blocks the tool call and shows the message to the agent.
"""
import fnmatch
import glob
import json
import os
import posixpath
import re
import sys

PROTECTED = [
    "tests/golden/*",
    "tests/golden/**",
    "crates/*/tests/acceptance/*",
    "crates/*/tests/acceptance/**",
    "schema/*",
    "schema/**",
]

PROTECTED_REL_RE = re.compile(r"^(tests/golden|crates/[^/]+/tests/acceptance|schema)(/|$)")
# Distinctive enough to match anywhere in a path we cannot place under the root.
PROTECTED_ANYWHERE_RE = re.compile(r"(^|/)(tests/golden|crates/[^/]+/tests/acceptance)(/|$)")
VARIABLE_PREFIX_RE = re.compile(r"^\$(\{[^}]*\}|[A-Za-z_][A-Za-z0-9_:]*)/?")
GLOB_CHARS_RE =re.compile(r"[*?\[]")

# Commands where every path argument is written, removed or moved.
WRITE_ANY = {
    "rm", "rmdir", "unlink", "shred", "truncate", "touch", "mkdir", "chmod", "chown", "tee",
    "mv", "move", "ren", "rename", "del", "erase", "rd",
    "remove-item", "ri", "move-item", "mi", "rename-item", "rni",
    "set-content", "sc", "add-content", "ac", "clear-content", "clc",
    "out-file", "new-item", "ni", "export-csv", "tee-object", "set-itemproperty",
}
# Commands that also destroy everything below a directory argument.
DESTRUCTIVE = {"rm", "rmdir", "mv", "move", "ren", "rename", "del", "erase", "rd",
               "remove-item", "ri", "move-item", "mi", "rename-item", "rni"}
# Commands that read their sources and write only the destination.
COPY_LIKE = {"cp", "copy", "copy-item", "cpi", "install", "ln", "rsync"}
DEST_FLAGS = {"-destination", "-t", "--target-directory"}
# `xcopy src dst /flags`, `robocopy src dst [files] /flags`: the destination is second.
COPY_SECOND = {"xcopy", "robocopy"}
# `curl -o file`, `sort -o file`, `Invoke-WebRequest -OutFile file`, …
OUTPUT_FLAGS = {"-o", "--output", "--out", "-outfile"}
CHDIR = {"cd", "chdir", "pushd", "set-location", "sl"}
SHELLS = {"bash", "sh", "zsh", "powershell", "pwsh", "cmd"}
SHELL_SCRIPT_FLAGS = {"-c", "-command", "/c", "-lc"}
INTERPRETERS = {"python", "python3", "py", "node", "perl", "ruby"}
GIT_WRITE = {"rm", "mv", "restore", "checkout", "clean", "apply", "stash"}
PREFIXES = {"sudo", "command", "exec", "time", "nohup", "env", "&", "."}
SCRIPT_WRITE_RE = re.compile(
    r"write_text|write_bytes|open\s*\([^)]*,\s*['\"][rbt]*[wax+]|writeFile|appendFile|unlink|rmtree|\bremove\b|\brename\b|"
    r"shutil|WriteAll|AppendAll|\bDelete\b|Set-Content|Out-File",
)
DOTNET_WRITE_RE = re.compile(r"WriteAll(Text|Lines|Bytes)|AppendAll(Text|Lines)|::Delete|::Move|::Copy", re.I)
ASSIGNMENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
HEREDOC_RE = re.compile(r"<<-?\s*(['\"]?)([A-Za-z_][A-Za-z0-9_]*)\1")

SEPARATORS = {";", "&&", "||", "|", "&", "\n", "(", ")", "{", "}"}


def blocked(what: str) -> int:
    print(
        f"BLOCKED: {what} touches a protected test/golden/schema path (ADR-0014). "
        "Implementation sessions may not change tests. If this change is "
        "intended, stop and ask the owner to run a test-authoring session "
        "(ECON_TEST_AUTHORING=1).",
        file=sys.stderr,
    )
    return 2


def norm_path(path: str) -> str:
    """Forward slashes, `c:/…` for drive paths (also from Git Bash `/c/…`), lower-case drive."""
    p = path.replace("\\", "/")
    m = re.match(r"^/([A-Za-z])(/|$)", p)
    if m:
        p = f"{m.group(1)}:/{p[3:]}"
    if re.match(r"^[A-Za-z]:", p):
        p = p[0].lower() + p[1:]
    return p


def is_abs(p: str) -> bool:
    return p.startswith("/") or bool(re.match(r"^[a-z]:/", p))


def rel_to_root(token: str, cwd: str, root: str):
    """Path of `token` relative to the project root, or None if outside it."""
    p = norm_path(token)
    if not is_abs(p):
        p = posixpath.join(norm_path(cwd), p)
    p = posixpath.normpath(p)
    r = posixpath.normpath(norm_path(root))
    if p.lower() == r.lower():
        return "."
    if p.lower().startswith(r.lower().rstrip("/") + "/"):
        return p[len(r.rstrip("/")) + 1:]
    return None


def protected_dirs(root: str):
    dirs = ["tests/golden", "schema"]
    pattern = os.path.join(root, "crates", "*", "tests", "acceptance")
    for d in sorted(glob.glob(pattern)):
        dirs.append(os.path.relpath(d, root).replace("\\", "/"))
    return dirs


def is_protected(token: str, cwd: str, root: str, destructive: bool = False) -> bool:
    """Does writing to `token` change a protected path?

    With `destructive`, a directory that contains a protected path counts too.
    """
    token = token.strip()
    if not token:
        return False
    if token.startswith("$"):
        # `$VAR/tests/golden/x`: the variable is unknown, the rest is telling
        rest = VARIABLE_PREFIX_RE.sub("", norm_path(token))
        return bool(PROTECTED_ANYWHERE_RE.search(rest) or PROTECTED_REL_RE.match(rest))
    rel = rel_to_root(token, cwd, root)
    if rel is None:
        return bool(PROTECTED_ANYWHERE_RE.search(posixpath.normpath(norm_path(token))))
    if PROTECTED_REL_RE.match(rel):
        return True
    dirs = protected_dirs(root)
    if GLOB_CHARS_RE.search(rel):
        globs = rel.split("/")
        for d in dirs:
            parts = d.split("/")
            if all(fnmatch.fnmatch(part, g) for part, g in zip(parts, globs)):
                # the glob names the protected directory, something in it, or (shorter) a parent
                if destructive or len(globs) >= len(parts):
                    return True
        return False
    if destructive:
        return rel == "." or any(d.startswith(rel + "/") for d in dirs)
    return False


def strip_heredocs(command: str) -> str:
    """Drop here-document bodies: they are data, not commands."""
    lines = command.split("\n")
    out, end = [], None
    for line in lines:
        if end is not None:
            if line.strip() == end:
                end = None
            continue
        out.append(line)
        m = HEREDOC_RE.search(line)
        if m:
            end = m.group(2)
    return "\n".join(out)


def tokenize(command: str):
    """Split a shell command into ("word" | "op" | "redir", text) tokens.

    Understands single and double quotes (Bash and PowerShell), so operators
    inside a quoted string (a commit message, say) are not operators.
    """
    tokens, buf, has_word = [], [], False
    i, n = 0, len(command)

    def flush():
        nonlocal has_word
        if has_word:
            tokens.append(("word", "".join(buf)))
        buf.clear()
        has_word = False

    while i < n:
        c = command[i]
        nxt = command[i + 1] if i + 1 < n else ""
        if c in "\\`" and nxt == "\n":  # line continuation
            i += 2
        elif c == "'":
            j = command.find("'", i + 1)
            j = n if j == -1 else j
            buf.append(command[i + 1:j])
            has_word = True
            i = j + 1
        elif c == '"':
            j = i + 1
            while j < n and command[j] != '"':
                if command[j] in "\\`" and j + 1 < n and command[j + 1] == '"':
                    j += 1
                buf.append(command[j])
                j += 1
            has_word = True
            i = j + 1
        elif c == ">":
            text = "".join(buf)
            if text.isdigit() or text == "*":  # fd prefix: 2>, *>
                buf.clear()
                has_word = False
            flush()
            op = ">"
            i += 1
            while i < n and command[i] in ">|":
                op += command[i]
                i += 1
            if i < n and command[i] == "&":
                op += "&"
                i += 1
            tokens.append(("redir", op))
        elif c == "<":
            flush()
            i += 1
        elif c in "{}" and has_word:  # `${VAR}/x`, `x}`: part of the word
            buf.append(c)
            i += 1
        elif c == "\n" or c in ";(){}":
            flush()
            tokens.append(("op", c))
            i += 1
        elif c in "|&":
            flush()
            if nxt == c:
                tokens.append(("op", c + c))
                i += 2
            else:
                tokens.append(("op", c))
                i += 1
        elif c.isspace():
            flush()
            i += 1
        else:
            buf.append(c)
            has_word = True
            i += 1
    flush()
    return tokens


def command_name(word: str) -> str:
    name = norm_path(word).rsplit("/", 1)[-1].lower()
    return name[:-4] if name.endswith(".exe") else name


def split_flags(args):
    """(positional args, lower-cased flags). `--flag=value` keeps the value as positional."""
    pos, flags = [], []
    for a in args:
        if a.startswith("-") and len(a) > 1:
            if "=" in a:
                flags.append(a.split("=", 1)[0].lower())
                pos.append(a.split("=", 1)[1])
            else:
                flags.append(a.lower())
        else:
            pos.append(a)
    return pos, flags


def simple_command_writes(words, cwd: str, root: str) -> bool:
    """Does one simple command (no pipes or separators) write to a protected path?"""
    while words and (ASSIGNMENT_RE.match(words[0]) or command_name(words[0]) in PREFIXES):
        words = words[1:]
    if not words:
        return False
    name, args = command_name(words[0]), words[1:]
    pos, flags = split_flags(args)

    def prot(tokens, destructive=False):
        return any(is_protected(t, cwd, root, destructive) for t in tokens)

    for i, a in enumerate(args):
        flag, _, value = a.partition("=")
        if flag.lower() in OUTPUT_FLAGS and prot([value] if value else args[i + 1:i + 2]):
            return True
    if name in WRITE_ANY:
        return prot(pos, name in DESTRUCTIVE)
    if name in COPY_SECOND:
        return prot(pos[1:2])
    if name in COPY_LIKE:
        for i, a in enumerate(args[:-1]):
            if a.lower() in DEST_FLAGS:
                return prot([args[i + 1]])
        return prot(pos[-1:])
    if name == "sed":
        in_place = any(f.startswith("--in-place") or re.match(r"^-[a-z]*i", f) for f in flags)
        return in_place and prot(pos)
    if name == "perl":
        in_place = any(re.match(r"^-[a-z0]*i", f) for f in flags)
        if in_place and prot(pos):
            return True
    if name == "dd":
        return prot([a[3:] for a in args if a.startswith("of=")])
    if name == "find":
        roots = []
        for a in args:
            if a.startswith("-"):
                break
            roots.append(a)
        roots = roots or ["."]
        for i, a in enumerate(args):
            if a == "-delete" and prot(roots, True):
                return True
            if a in ("-exec", "-execdir", "-ok") and i + 1 < len(args):
                inner = command_name(args[i + 1])
                rest = [x.lower() for x in args[i + 2:]]
                writes = inner in WRITE_ANY or (inner == "sed" and any(re.match(r"^-[a-z]*i", x) for x in rest))
                if writes and prot(roots, True):
                    return True
        return False
    if name == "git":
        return any(p in GIT_WRITE for p in pos[:1]) and prot(pos[1:], True)
    if name in SHELLS:
        for i, a in enumerate(args):
            if a.lower() in SHELL_SCRIPT_FLAGS:
                return command_writes(" ".join(args[i + 1:]), cwd, root)
        return False
    if name in INTERPRETERS:
        for i, a in enumerate(args[:-1]):
            if a in ("-c", "-e"):
                script = args[i + 1]
                return bool(SCRIPT_WRITE_RE.search(script)) and script_names_protected(script, cwd, root)
        return False
    return False


def script_names_protected(script: str, cwd: str, root: str) -> bool:
    candidates = re.findall(r"[A-Za-z0-9_.:/\\*-]+", script)
    return any(("/" in c or "\\" in c) and is_protected(c, cwd, root) for c in candidates)


def command_writes(command: str, cwd: str, root: str) -> bool:
    """Does this shell command line write to a protected path?"""
    command = strip_heredocs(command)
    # PowerShell calling .NET directly: [IO.File]::WriteAllText('tests/golden/…', …)
    if DOTNET_WRITE_RE.search(command) and script_names_protected(command, cwd, root):
        return True
    tokens = tokenize(command)
    words, i = [], 0

    def end_simple():
        nonlocal cwd
        if not words:
            return False
        name = command_name(words[0])
        if name in CHDIR:
            target = next((w for w in words[1:] if not w.startswith("-")), None)
            if target:
                moved = rel_to_root(target, cwd, root)
                cwd = posixpath.join(norm_path(root), moved) if moved is not None else norm_path(target)
            return False
        return simple_command_writes(words, cwd, root)

    while i < len(tokens):
        kind, text = tokens[i]
        if kind == "redir":
            target = tokens[i + 1][1] if i + 1 < len(tokens) and tokens[i + 1][0] == "word" else ""
            i += 2 if target else 1
            # `2>&1` duplicates a descriptor; `>&file` and `> file` write a file
            if target and not (text.endswith("&") and target.isdigit()) and is_protected(target, cwd, root):
                return True
            continue
        if kind == "op":
            if end_simple():
                return True
            words = []
        else:
            # command substitution inside a word: "$(rm tests/golden/x)"
            for inner in re.findall(r"\$\(([^()]*)\)", text):
                if command_writes(inner, cwd, root):
                    return True
            words.append(text)
        i += 1
    return end_simple()


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        return 0
    if os.environ.get("ECON_TEST_AUTHORING") == "1":
        return 0
    tool_input = payload.get("tool_input") or {}
    root = os.environ.get("CLAUDE_PROJECT_DIR") or payload.get("cwd") or os.getcwd()
    command = tool_input.get("command")
    if isinstance(command, str):
        cwd = payload.get("cwd") or root
        if command_writes(command, cwd, root):
            return blocked("this shell command")
        return 0
    path = tool_input.get("file_path") or tool_input.get("notebook_path") or ""
    if not path:
        return 0
    try:
        rel = os.path.relpath(os.path.abspath(path), os.path.abspath(root))
    except ValueError:  # different drive on Windows
        return 0
    rel = rel.replace("\\", "/")
    if not any(fnmatch.fnmatch(rel, pat) for pat in PROTECTED):
        return 0
    return blocked(rel)


if __name__ == "__main__":
    sys.exit(main())
