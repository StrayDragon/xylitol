#!/usr/bin/env python3
"""QA check: designing modules + generated AGENT-INDEX freshness.

Usage:
  python3 scripts/check_tui_designing.py --check
  python3 scripts/check_tui_designing.py --check --verbose
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DESIGNING = REPO / "src" / "app" / "tui" / "designing"
MODULES = DESIGNING / "modules"
INDEX = DESIGNING / "generated" / "AGENT-INDEX.md"
GEN = REPO / "scripts" / "gen_designing_index.py"
AGENTS = DESIGNING / "AGENTS.md"
HEX_RE = re.compile(r"#[0-9a-fA-F]{6}")


def fail(msg: str, errors: list[str]) -> None:
    errors.append(msg)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="non-mutating gate")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    errors: list[str] = []
    if not AGENTS.is_file():
        fail(f"missing {AGENTS.relative_to(REPO)}", errors)
    if not MODULES.is_dir():
        fail(f"missing {MODULES.relative_to(REPO)}", errors)

    mods = sorted(p for p in MODULES.iterdir() if p.is_dir()) if MODULES.is_dir() else []
    for mod in mods:
        if not (mod / "intent.md").is_file():
            fail(f"designing: {mod.name} missing intent.md", errors)
        else:
            n = len((mod / "intent.md").read_text(encoding="utf-8").splitlines())
            if n > 80:
                fail(f"designing: {mod.name}/intent.md is {n} lines (soft top 80)", errors)
        if not (mod / "preview.ts").is_file():
            fail(f"designing: {mod.name} missing preview.ts", errors)
        states = list((mod / "states").glob("*.yaml")) if (mod / "states").is_dir() else []
        if not states:
            fail(f"designing: {mod.name} has no states/*.yaml", errors)
        for path in [mod / "intent.md", *states]:
            if not path.is_file():
                continue
            text = path.read_text(encoding="utf-8")
            if HEX_RE.search(text):
                fail(f"no-raw-hex: {path.relative_to(REPO)}", errors)

    want = subprocess.check_output(
        [sys.executable, str(GEN), "--stdout"],
        cwd=REPO,
        text=True,
    )
    if not INDEX.is_file():
        fail("missing generated/AGENT-INDEX.md — run: just gen-designing-index", errors)
    elif INDEX.read_text(encoding="utf-8") != want:
        fail("stale generated/AGENT-INDEX.md — run: just gen-designing-index", errors)

    app = DESIGNING / "app"
    for name in ("package.json", "index.html", "src/main.ts"):
        if not (app / name).is_file():
            fail(f"designing app missing {name}", errors)

    if errors:
        print("check_tui_designing: FAIL", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1
    if args.verbose:
        print(f"ok: designing ({len(mods)} modules, index fresh)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
