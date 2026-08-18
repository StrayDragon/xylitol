#!/usr/bin/env python3
"""QA check: ath12 entry-module complexity via cccc-rs (c1840 / qg06).

Xylitol complexity model (see justfile `complexity` + this gate):

  Layer A — Clippy `-D warnings` (no cognitive_complexity; restriction/off).
  Layer B — (retired) file-LOC soft/hard caps: removed — line counts are NOT
            a quality signal (`src/AGENTS.md` 复杂度与体量); quality signals
            are function-level complexity metrics only.
  Layer C — THIS SCRIPT (HARD in `just qa`): Sonar cognitive + McCabe
            cyclomatic on ath12 *entry coordinators* only (ath12 MUST).
  Layer D — Soft radar (`--radar` / `just complexity`): whole production tree
            (tests/harness excluded) top-cognitive; smell signal, not a hard
            gate (slash/pending_ui already exceed entry thresholds).

Why cccc-rs (not Clippy cognitive / lizard alone): dual metrics, JSON-ready
`--max-*` exits, scores align with our trial baselines; Clippy scores diverge
and enabling it would fail ~10 existing functions today.

Usage:
  python3 scripts/check_complexity.py --check
  python3 scripts/check_complexity.py --check --verbose
  python3 scripts/check_complexity.py --radar
  python3 scripts/check_complexity.py --radar --verbose
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# Pin for reproducible installs into .tools/ (CLI only — not a Cargo dep).
CCCC_RS_CRATE = "cccc-rs-cli"
CCCC_RS_VERSION = "0.4.0"
CCCC_RS_BIN = "cccc-rs"

# HARD gate: ath12 entry coordinators (function complexity, not file LOC).
# Thresholds MUST match llmanspec ath12 / test-qa-gate qg06 (c1840).
ENTRY_PATHS = [
    REPO / "src/app/tui/host/mod.rs",
    REPO / "src/app/tui/layout/root/mod.rs",
    REPO / "src/app/tui/effects/mod.rs",
    REPO / "src/app/tui/bridge/mod.rs",
]

# Soft radar roots: whole production tree (tests/harness excluded via --exclude).
RADAR_PATHS = [REPO / "src"]
# Test-only slices that would dominate the top-cognitive ranking with noise.
RADAR_EXCLUDES = ["**/tests.rs", "**/tests/**", "**/harness.rs"]

# Ratchet: measured entry max is cog 30 / cyc 25 (`apply_tool_result_to_entries`).
# Keep 2 points of slack so a small arm can land; do not grow back toward 35/30.
# Keep in sync with `src/AGENTS.md` TUI 面复杂度闸 (复杂度与体量).
MAX_COGNITIVE = 32
MAX_CYCLOMATIC = 27

TOOLS_ROOT = REPO / ".tools"
TOOLS_BIN = TOOLS_ROOT / "bin" / CCCC_RS_BIN


def fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 1


def find_cccc_rs() -> Path | None:
    env = os.environ.get("XYLITOL_CCCC_RS")
    if env:
        p = Path(env)
        if p.is_file() and os.access(p, os.X_OK):
            return p
    which = shutil.which(CCCC_RS_BIN)
    if which:
        return Path(which)
    if TOOLS_BIN.is_file() and os.access(TOOLS_BIN, os.X_OK):
        return TOOLS_BIN
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    cargo_bin = cargo_home / "bin" / CCCC_RS_BIN
    if cargo_bin.is_file() and os.access(cargo_bin, os.X_OK):
        return cargo_bin
    return None


def ensure_cccc_rs(*, verbose: bool) -> Path | None:
    found = find_cccc_rs()
    if found is not None:
        return found
    if shutil.which("cargo") is None:
        fail(
            f"{CCCC_RS_BIN} not found and cargo unavailable; "
            f"install with: cargo install {CCCC_RS_CRATE} --version {CCCC_RS_VERSION} --locked"
        )
        return None
    if verbose:
        print(
            f"installing {CCCC_RS_CRATE}=={CCCC_RS_VERSION} into {TOOLS_ROOT} …",
            file=sys.stderr,
        )
    TOOLS_ROOT.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        [
            "cargo",
            "install",
            CCCC_RS_CRATE,
            "--version",
            CCCC_RS_VERSION,
            "--locked",
            "--root",
            str(TOOLS_ROOT),
        ],
        cwd=REPO,
        check=False,
    )
    if proc.returncode != 0 or not TOOLS_BIN.is_file():
        fail(
            f"failed to install {CCCC_RS_CRATE}; "
            f"try: cargo install {CCCC_RS_CRATE} --version {CCCC_RS_VERSION} --locked"
        )
        return None
    return TOOLS_BIN


def run_cccc(
    binary: Path,
    args: list[str],
    *,
    paths: list[Path],
) -> subprocess.CompletedProcess[str]:
    cmd = [str(binary), *args, *(str(p) for p in paths)]
    return subprocess.run(
        cmd,
        cwd=REPO,
        check=False,
        text=True,
        capture_output=True,
    )


def hard_gate(binary: Path, *, verbose: bool) -> int:
    missing = [p for p in ENTRY_PATHS if not p.is_file()]
    if missing:
        return fail("missing entry paths:\n  " + "\n  ".join(str(p) for p in missing))

    proc = run_cccc(
        binary,
        [
            "--max-cognitive",
            str(MAX_COGNITIVE),
            "--max-cyclomatic",
            str(MAX_CYCLOMATIC),
        ],
        paths=ENTRY_PATHS,
    )
    if proc.returncode != 0:
        # Re-run with table for humans when thresholds trip.
        table = run_cccc(
            binary,
            [
                "--table",
                "--min",
                "8",
                "--max-cognitive",
                str(MAX_COGNITIVE),
                "--max-cyclomatic",
                str(MAX_CYCLOMATIC),
            ],
            paths=ENTRY_PATHS,
        )
        out = (table.stdout or proc.stdout or "").strip()
        err = (table.stderr or proc.stderr or "").strip()
        print(
            "error: ath12 entry complexity gate failed "
            f"(max cognitive {MAX_COGNITIVE}, max cyclomatic {MAX_CYCLOMATIC})",
            file=sys.stderr,
        )
        if out:
            print(out, file=sys.stderr)
        if err:
            print(err, file=sys.stderr)
        print(
            "hint: split the hot function, or raise thresholds only via deliberate "
            "harness change (prefer refactor).",
            file=sys.stderr,
        )
        return 1

    if verbose:
        summary = run_cccc(binary, [], paths=ENTRY_PATHS)
        try:
            data = json.loads(summary.stdout or "{}")
            s = data.get("summary") or {}
            cog = s.get("cognitive") or {}
            cyc = s.get("cyclomatic") or {}
            print(
                "ok: entry complexity "
                f"files={s.get('file_count')} fns={s.get('function_count')} "
                f"cog_max={cog.get('max')} cyc_max={cyc.get('max')} "
                f"(limits {MAX_COGNITIVE}/{MAX_CYCLOMATIC})"
            )
        except json.JSONDecodeError:
            print(
                f"ok: entry complexity under cognitive≤{MAX_COGNITIVE} "
                f"cyclomatic≤{MAX_CYCLOMATIC}"
            )
    return 0


def soft_radar(binary: Path, *, verbose: bool) -> int:
    missing = [p for p in RADAR_PATHS if not p.exists()]
    if missing:
        return fail("missing radar paths:\n  " + "\n  ".join(str(p) for p in missing))

    proc = run_cccc(
        binary,
        [
            "--table",
            "--top-cognitive",
            "20",
            "--min",
            "10",
            *(f"--exclude={g}" for g in RADAR_EXCLUDES),
        ],
        paths=RADAR_PATHS,
    )
    # Radar never fails on threshold — only on tool/path errors.
    if proc.returncode not in (0, 1):
        # cccc may exit 1 only with --max-*; without max, expect 0.
        err = (proc.stderr or "").strip()
        out = (proc.stdout or "").strip()
        if not out and proc.returncode != 0:
            return fail(f"cccc-rs radar failed (exit {proc.returncode}): {err or out}")
    out = (proc.stdout or "").strip()
    if out:
        print(out, flush=True)
    if verbose or out:
        print(
            f"(radar soft — not a qa hard gate; entry limits "
            f"cognitive≤{MAX_COGNITIVE} cyclomatic≤{MAX_CYCLOMATIC})",
            flush=True,
        )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="hard-gate ath12 entry modules (default when neither --radar)",
    )
    parser.add_argument(
        "--radar",
        action="store_true",
        help="print soft top-cognitive ranking for the production tree (exit 0)",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="print ok summary / install progress (default: silent success)",
    )
    args = parser.parse_args()

    # `just check-scripts` always passes --check; allow --radar alone for humans.
    do_check = args.check or not args.radar
    do_radar = args.radar

    binary = ensure_cccc_rs(verbose=args.verbose)
    if binary is None:
        return 1

    rc = 0
    if do_check:
        rc = hard_gate(binary, verbose=args.verbose)
        if rc != 0:
            return rc
    if do_radar:
        rc = soft_radar(binary, verbose=args.verbose)
    return rc


if __name__ == "__main__":
    sys.exit(main())
