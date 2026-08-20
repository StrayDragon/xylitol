#!/usr/bin/env python3
"""QA check: root `_TUI_MIGRATED_TODO.md` cannot vanish while attach P0/P1 is open.

While `AGENTS.md` contains `TUI_MIGRATED_TODO_REQUIRED`, the root checklist MUST exist.
While P0 or P1 still has `- [ ]` items, that marker MUST remain. P2 does not block.
The old delayed-changes copy MUST NOT exist (forgotten-in-a-subdir).

Usage:
  python3 scripts/check_tui_migrated_todo.py
  python3 scripts/check_tui_migrated_todo.py --check
  python3 scripts/check_tui_migrated_todo.py --check --verbose
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TODO = REPO / "_TUI_MIGRATED_TODO.md"
AGENTS = REPO / "AGENTS.md"
OLD = REPO / "llmanspec" / "delayed-changes" / "tui" / "attach-parity-checklist.md"
MARKER = "TUI_MIGRATED_TODO_REQUIRED"
HEADINGS = ("P0", "P1")


def section_unchecked(text: str, heading: str) -> list[str]:
    pat = re.compile(rf"^### {re.escape(heading)}\b.*$", re.MULTILINE)
    m = pat.search(text)
    if not m:
        return []
    rest = text[m.end() :]
    nxt = re.search(r"^### ", rest, re.MULTILINE)
    body = rest[: nxt.start()] if nxt else rest
    return [
        ln.rstrip()
        for ln in body.splitlines()
        if ln.lstrip().startswith("- [ ]")
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    errors: list[str] = []
    if not AGENTS.is_file():
        print("error: AGENTS.md missing", file=sys.stderr)
        return 1
    agents = AGENTS.read_text(encoding="utf-8")
    has_marker = MARKER in agents
    has_file = TODO.is_file()
    open_items: list[str] = []
    if has_file:
        body = TODO.read_text(encoding="utf-8")
        for h in HEADINGS:
            open_items.extend(section_unchecked(body, h))
        if "_TUI_MIGRATED_TODO.md" not in agents:
            errors.append("AGENTS.md MUST mention `_TUI_MIGRATED_TODO.md` while the file exists")

    if OLD.is_file():
        errors.append(
            f"move leftover to repo root: {OLD.relative_to(REPO)} "
            "(use `_TUI_MIGRATED_TODO.md`, do not hide under llmanspec/)"
        )

    if open_items:
        if not has_file:
            errors.append("P0/P1 still open but `_TUI_MIGRATED_TODO.md` is missing")
        if not has_marker:
            errors.append(
                "P0/P1 still open: AGENTS.md MUST keep "
                f"`{MARKER}` until those boxes are checked"
            )

    if has_marker and not has_file:
        errors.append(
            f"AGENTS.md has `{MARKER}` but `_TUI_MIGRATED_TODO.md` is missing; "
            "do not delete the checklist until P0+P1 are done, then drop marker+file+this script together"
        )

    if errors:
        for e in errors:
            print(f"error: {e}", file=sys.stderr)
        if open_items and args.verbose:
            print("open P0/P1 items:", file=sys.stderr)
            for ln in open_items:
                print(f"  {ln}", file=sys.stderr)
        return 1

    if args.verbose:
        if has_file:
            print(
                f"ok: `_TUI_MIGRATED_TODO.md` present; "
                f"P0/P1 open={len(open_items)}; marker={'yes' if has_marker else 'no'}"
            )
        else:
            print("ok: TUI migrated checklist retired")
    return 0


if __name__ == "__main__":
    sys.exit(main())
