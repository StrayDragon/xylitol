#!/usr/bin/env python3
"""QA check: specta TypeScript bindings match the checked-in copy.

Regenerates packages/xylitol-client-typescript-sdk/bindings.ts via the
gen_protocol_ts example and diffs it against the copy in git. Drift MUST
fail. OpenAPI is not this gate.

Usage:
  python3 scripts/check_protocol_ts_bindings.py
  python3 scripts/check_protocol_ts_bindings.py --check
  python3 scripts/check_protocol_ts_bindings.py --check --verbose
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CHECKED_IN = REPO / "packages" / "xylitol-client-typescript-sdk" / "bindings.ts"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="same as default (non-mutating gate)",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="print ok summary on success",
    )
    args = parser.parse_args()

    if not CHECKED_IN.is_file():
        print(
            f"error: missing {CHECKED_IN.relative_to(REPO)}; run `just gen-sdk`",
            file=sys.stderr,
        )
        return 1

    proc = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "--example",
            "gen_protocol_ts",
            "--",
            "--stdout",
        ],
        cwd=REPO,
        check=False,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        print("error: specta export failed (cargo run --example gen_protocol_ts)", file=sys.stderr)
        return 1

    generated = proc.stdout
    checked = CHECKED_IN.read_text(encoding="utf-8")
    if generated != checked:
        print(
            "error: packages/xylitol-client-typescript-sdk/bindings.ts is stale; run `just gen-sdk`",
            file=sys.stderr,
        )
        # Short context: first mismatch line.
        gen_lines = generated.splitlines()
        chk_lines = checked.splitlines()
        for i, (a, b) in enumerate(zip(gen_lines, chk_lines), start=1):
            if a != b:
                print(f"  first diff at line {i}", file=sys.stderr)
                print(f"    generated: {a[:200]}", file=sys.stderr)
                print(f"    checked:   {b[:200]}", file=sys.stderr)
                break
        else:
            print(
                f"  length mismatch generated={len(gen_lines)} checked={len(chk_lines)}",
                file=sys.stderr,
            )
        return 1

    if args.verbose:
        print(f"ok: {CHECKED_IN.relative_to(REPO)} matches specta export")
    return 0


if __name__ == "__main__":
    sys.exit(main())
