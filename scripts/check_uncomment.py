#!/usr/bin/env python3
"""QA gate: uncomment removable-comment policy (read-only).

Runs only `uncomment --check` with `--changed-lines` — never strips comments,
never `lint --fix`, so the working tree stays untouched.

Legacy inline comments are not reported until a line is edited against `--base`.

Skip: XYLITOL_SKIP_UNCOMMENT=1
Base ref: UNCOMMENT_BASE, else origin/HEAD, else main.

Usage:
  python3 scripts/check_uncomment.py --check
  python3 scripts/check_uncomment.py --check --verbose
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

_SCAN_PATHS = (
    "src",
    "crates",
    "plugins",
    "scripts",
    "tests",
    "llmanspec",
    "configs",
    ".github",
    "Cargo.toml",
    "justfile",
    "AGENTS.md",
    "README.md",
)

def resolve_base() -> str:
    if env := os.environ.get("UNCOMMENT_BASE"):
        return env
    for candidate in ("origin/HEAD", "origin/main", "main"):
        r = subprocess.run(
            ["git", "rev-parse", "--verify", candidate],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        if r.returncode == 0:
            return candidate
    return "HEAD~1"


def run_uncomment_check(base: str) -> int:
    cmd = [
        "uncomment",
        "--check",
        "--changed-lines",
        "--changed-only",
        "--base",
        base,
        *_SCAN_PATHS,
    ]
    return subprocess.run(cmd, cwd=REPO).returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="non-mutating gate (default)",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="print ok summary on success",
    )
    args = parser.parse_args()

    if os.environ.get("XYLITOL_SKIP_UNCOMMENT") == "1":
        print("check_uncomment: skipped (XYLITOL_SKIP_UNCOMMENT=1)")
        return 0

    if shutil.which("uncomment") is None:
        print(
            "error: uncomment not on PATH (install: cargo install uncomment | pipx install uncomment)",
            file=sys.stderr,
        )
        return 1

    base = resolve_base()
    rc = run_uncomment_check(base)
    if rc != 0:
        return rc

    if args.verbose:
        print(f"ok: uncomment --check (changed-lines only, base={base})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
