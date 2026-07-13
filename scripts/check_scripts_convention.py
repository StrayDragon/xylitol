#!/usr/bin/env python3
"""QA check: scripts/ naming convention for gates vs maintenance tools.

Convention:
  - scripts/check_*.py / scripts/check-*.py — non-mutating; MUST run via `just qa`
  - other scripts/* (e.g. cleanup_*) — maintenance; MUST NOT be required by qa

Usage:
  python3 scripts/check_scripts_convention.py
  python3 scripts/check_scripts_convention.py --check
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCRIPTS = REPO / "scripts"
JUSTFILE = REPO / "justfile"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="same as default (non-mutating gate)",
    )
    parser.parse_args()

    if not SCRIPTS.is_dir():
        print("error: scripts/ directory missing", file=sys.stderr)
        return 1
    if not JUSTFILE.is_file():
        print("error: justfile missing", file=sys.stderr)
        return 1

    just = JUSTFILE.read_text(encoding="utf-8")
    qa_line = next((ln for ln in just.splitlines() if ln.startswith("qa:")), "")
    if "check-scripts-wired" not in qa_line or "check-scripts" not in qa_line:
        print(
            "error: just qa must depend on check-scripts-wired and check-scripts",
            file=sys.stderr,
        )
        print(f"  got: {qa_line!r}", file=sys.stderr)
        return 1

    checks = sorted(
        [
            *SCRIPTS.glob("check_*.py"),
            *SCRIPTS.glob("check-*.py"),
        ]
    )
    # This file must be among them when present.
    me = Path(__file__).resolve()
    if me not in {p.resolve() for p in checks}:
        print(f"error: self not matched as check_*.py: {me}", file=sys.stderr)
        return 1

    print(f"ok: {len(checks)} scripts/check_* gate(s); qa wires check-scripts*")
    return 0


if __name__ == "__main__":
    sys.exit(main())
