#!/usr/bin/env python3
"""QA check: skill description trigger tail is in sync with references.

Each `.agents/skills/*/SKILL.md` that opts into the harness carries a generated
tail joining the per-reference `brief:` lines (see scripts/gen_skill_description.py).
This gate fails when that tail drifts, when a reference lacks a non-empty
`brief:`, or when a description exceeds the 1024-char size limit.

Usage:
  python3 scripts/check_skill_description.py --check
  python3 scripts/check_skill_description.py --check --verbose
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gen_skill_description import run  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="same as default (non-mutating gate)")
    parser.add_argument("--verbose", action="store_true", help="print ok summary on success")
    args = parser.parse_args()
    return run(check=True, verbose=args.verbose)


if __name__ == "__main__":
    sys.exit(main())
