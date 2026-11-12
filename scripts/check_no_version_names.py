#!/usr/bin/env python3
"""QA check: no legacy wire literals and no version-named identifiers (read-only gate).

Rule A (literal, every scanned file): FORBIDDEN_LITERALS are matched on raw lines, so
comments and strings count too. Only `fory` is case-insensitive; the others are exact
(including the legacy envelope field name `rpcId`).

Rule B (identifier, .rs / .py only): identifier tokens outside string literals and
comments must not carry a version segment, i.e. a `_`-separated `v<digits>` part or a
CamelCase `V<digits>` word. Exemptions are external contracts, not wire naming:
  - ALLOWED_IDENTIFIERS: uuid crate API (new_v4, new_v7).
  - ALLOWED_SEGMENTS: session on-disk format versions v5 / v6 / v7 (URL paths like
    /v1 live in string literals, which rule B never inspects).
  - identifiers containing an EXTERNAL_NAME_MARKERS word (external model names).
Model names, URL paths and trace schema names live in string literals, which rule B
never inspects.

Rule C (spec version labels, llmanspec/specs/**/*.feature only): spec lines must not
carry bare version labels (`\\bv<digits>\\b`). Allowed: v5/v6/v7 (session on-disk
formats) and any token appearing as a URL path segment on the same line (line contains
`/<token>`, e.g. `/api/v1`, `/v1/responses`).

Scope: crates/ plugins/ src/ tests/ scripts/ and .agents/skills/ (.claude/skills is a
symlink to it), llmanspec/specs/ at any depth, every AGENTS.md, the root justfile and
every *.toml. Pruned: target/ vendor/ .git/ __pycache__/ node_modules/ .tools/ .venv/,
and llmanspec/changes/ + llmanspec/delayed-changes/ at any depth (archive included).
Cargo.lock is skipped; this file is self-excluded (FORBIDDEN_LITERALS is its only
definition site).

Usage:
  python3 scripts/check_no_version_names.py [--check] [--verbose]
Exit 0 when clean; exit 1 and `file:line: message` lines on stderr otherwise.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SELF_REL = Path(__file__).resolve().relative_to(REPO).as_posix()

SCAN_TOP_DIRS = frozenset({"crates", "plugins", "src", "tests", "scripts"})
SCAN_PREFIXES = (".agents/skills/",)
PRUNED_DIR_NAMES = frozenset(
    {"target", "vendor", ".git", "__pycache__", "node_modules", ".tools", ".venv"}
)
PRUNED_LLMANSPEC_DIRS = frozenset({"changes", "delayed-changes"})
SKIPPED_FILE_NAMES = frozenset({"Cargo.lock"})
IDENTIFIER_SUFFIXES = frozenset({".rs", ".py"})

FORBIDDEN_LITERALS: tuple[str, ...] = (
    "fory",
    "FORY",
    "WIRE_FORMAT_FORY_V3",
    "XYLITOL_ENVELOPE_V3",
    "XYLITOL_WIRE_V3",
    "application/x-fory-v3",
    "xy.wire.v3",
    "envelope.fbs",
    "codegen-envelope",
    "gen_envelope",
    "foryc",
    "rpcId",
)
CASE_INSENSITIVE_LITERALS = frozenset({"fory"})

ALLOWED_IDENTIFIERS = frozenset({"new_v4", "new_v7"})
ALLOWED_SEGMENTS = frozenset({"v1", "v5", "v6", "v7"})
EXTERNAL_NAME_MARKERS: tuple[str, ...] = ("deepseek",)

ALLOWED_SPEC_VERSION_TOKENS = frozenset({"v5", "v6", "v7"})
SPEC_VERSION_TOKEN_RE = re.compile(r"\bv\d+\b")

LITERAL_RE = re.compile(
    "|".join(
        f"(?i:{re.escape(lit)})" if lit in CASE_INSENSITIVE_LITERALS else re.escape(lit)
        for lit in sorted(FORBIDDEN_LITERALS, key=len, reverse=True)
    )
)
VERSION_HINT_RE = re.compile(r"[vV]\d")
IDENT_RE = re.compile(r"(?<![A-Za-z0-9_])[A-Za-z_][A-Za-z0-9_]*")
LOWER_SEGMENT_RE = re.compile(r"[vV]\d+")
CAMEL_SEGMENT_RE = re.compile(r"(?<![A-Z])V\d+")
CHAR_LITERAL_RE = re.compile(r"'(?:\\u\{[0-9a-fA-F_]+\}|\\.|[^'\\\n])'")
TRIPLE_QUOTES = ('"""', "'''")


def _keep_dir(parent_parts: tuple[str, ...], name: str) -> bool:
    if name in PRUNED_DIR_NAMES:
        return False
    return not (parent_parts[-1:] == ("llmanspec",) and name in PRUNED_LLMANSPEC_DIRS)


def _in_scope(rel: str) -> bool:
    parts = rel.split("/")
    name = parts[-1]
    if name in SKIPPED_FILE_NAMES:
        return False
    if name == "AGENTS.md" or name.endswith(".toml") or rel == "justfile":
        return True
    if parts[0] in SCAN_TOP_DIRS or rel.startswith(SCAN_PREFIXES):
        return True
    return any(
        parts[i] == "llmanspec" and parts[i + 1] == "specs" for i in range(len(parts) - 1)
    )


def iter_scoped_files() -> list[str]:
    rels: set[str] = set()
    for dirpath, dirnames, filenames in os.walk(REPO):
        rel_dir = Path(dirpath).relative_to(REPO)
        dirnames[:] = sorted(d for d in dirnames if _keep_dir(rel_dir.parts, d))
        for name in filenames:
            rel = (rel_dir / name).as_posix()
            if rel != SELF_REL and _in_scope(rel) and (REPO / rel).is_file():
                rels.add(rel)
    return sorted(rels)


def _string_end(line: str, start: int, delim: str) -> int:
    j = start
    while j < len(line):
        if line[j] == "\\":
            j += 2
            continue
        if line.startswith(delim, j):
            return j + len(delim)
        j += 1
    return -1


def code_only(lines: list[str], suffix: str) -> list[str]:
    rust = suffix == ".rs"
    out: list[str] = []
    state: str | None = None
    for line in lines:
        code: list[str] = []
        i = 0
        while i < len(line):
            if state is not None:
                end = _string_end(line, i, state)
                if end < 0:
                    if not (rust or state in TRIPLE_QUOTES):
                        state = None
                    break
                i = end
                state = None
                continue
            ch = line[i]
            if rust:
                if line.startswith("//", i):
                    break
                if ch == '"':
                    state = '"'
                    i += 1
                    continue
                if ch == "'":
                    char_lit = CHAR_LITERAL_RE.match(line, i)
                    i = char_lit.end() if char_lit else i + 1
                    continue
            else:
                if ch == "#":
                    break
                if line.startswith(TRIPLE_QUOTES, i):
                    state = line[i : i + 3]
                    i += 3
                    continue
                if ch in "\"'":
                    state = ch
                    i += 1
                    continue
            code.append(ch)
            i += 1
        out.append("".join(code))
    return out


def literal_hits(line: str) -> list[str]:
    found: dict[str, str] = {}
    for match in LITERAL_RE.finditer(line):
        found.setdefault(match.group(0).lower(), match.group(0))
    return list(found.values())


def spec_version_label_hits(line: str) -> list[str]:
    return [
        token
        for token in SPEC_VERSION_TOKEN_RE.findall(line)
        if token not in ALLOWED_SPEC_VERSION_TOKENS and f"/{token}" not in line
    ]


def _version_segment(token: str) -> str | None:
    if token in ALLOWED_IDENTIFIERS:
        return None
    if any(marker in token.lower() for marker in EXTERNAL_NAME_MARKERS):
        return None
    for part in token.split("_"):
        if LOWER_SEGMENT_RE.fullmatch(part):
            candidates = [part]
        else:
            candidates = [m.group(0) for m in CAMEL_SEGMENT_RE.finditer(part)]
        for candidate in candidates:
            if candidate.lower() not in ALLOWED_SEGMENTS:
                return candidate
    return None


def version_identifiers(code: str, literals: list[str]) -> list[tuple[str, str]]:
    lowered_literals = [lit.lower() for lit in literals]
    found: dict[str, str] = {}
    for token in IDENT_RE.findall(code):
        if token in found or any(lit in token.lower() for lit in lowered_literals):
            continue
        segment = _version_segment(token)
        if segment is not None:
            found[token] = segment
    return list(found.items())


def scan_file(path: Path) -> list[tuple[int, str]]:
    raw = path.read_bytes()
    if b"\0" in raw[:8192]:
        return []
    text = raw.decode("utf-8", errors="replace")
    lines = text.split("\n")
    code = None
    if path.suffix in IDENTIFIER_SUFFIXES and VERSION_HINT_RE.search(text):
        code = code_only(lines, path.suffix)
    rel = path.relative_to(REPO)
    is_spec_feature = rel.parts[:2] == ("llmanspec", "specs") and path.suffix == ".feature"
    findings: list[tuple[int, str]] = []
    for lineno, line in enumerate(lines, 1):
        literals = literal_hits(line)
        for lit in literals:
            findings.append((lineno, f"forbidden literal {lit!r}"))
        if code is not None:
            for token, segment in version_identifiers(code[lineno - 1], literals):
                findings.append(
                    (lineno, f"version-named identifier {token!r} (segment {segment!r})")
                )
        if is_spec_feature:
            for token in spec_version_label_hits(line):
                findings.append((lineno, f"version label in spec {token!r}"))
    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="read-only gate (the only mode; accepted for qa-check-scripts)",
    )
    parser.add_argument(
        "--verbose", action="store_true", help="print an ok summary on success"
    )
    args = parser.parse_args()

    files = iter_scoped_files()
    violations: list[tuple[str, int, str]] = []
    for rel in files:
        for lineno, message in scan_file(REPO / rel):
            violations.append((rel, lineno, message))

    if violations:
        for rel, lineno, message in violations:
            print(f"{rel}:{lineno}: {message}", file=sys.stderr)
        file_count = len({rel for rel, _, _ in violations})
        print(
            f"error: version-name gate failed: {len(violations)} violation(s) "
            f"in {file_count} file(s)",
            file=sys.stderr,
        )
        return 1

    if args.verbose:
        print(f"ok: no version-named identifiers or legacy literals in {len(files)} file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
