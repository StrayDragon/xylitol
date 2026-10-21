#!/usr/bin/env python3
"""TEMPORARY (xylitol-dev skill): cccc-rs vs cargo-crap overlap / gate scope.

Not in repo scripts/ or justfile. See references/complexity-cccc-crap.md.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

TUI_ENTRY_FILES = (
    "crates/xylitol-app-tui/src/host/mod.rs",
    "crates/xylitol-app-tui/src/layout/root/mod.rs",
    "crates/xylitol-app-tui/src/effects/mod.rs",
    "crates/xylitol-app-tui/src/bridge/mod.rs",
)

CCCC_EXCLUDES = (
    "**/tests.rs",
    "**/tests/**",
    "**/harness.rs",
    "**/examples/**",
)

SCAN_ROOTS = ("src", "crates", "plugins")

MAX_COGNITIVE = 32
MAX_CYCLOMATIC = 27


def repo_root() -> Path:
    proc = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        text=True,
        capture_output=True,
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr or "run from inside the xylitol git work tree\n")
        raise SystemExit(proc.returncode)
    return Path(proc.stdout.strip())


@dataclass(frozen=True)
class FnRow:
    path: str
    name: str
    line: int
    cognitive: int
    cyclomatic: int

    @property
    def key(self) -> tuple[str, int, str]:
        return (self.path, self.line, self.name)


def run(cmd: list[str], *, cwd: Path) -> str:
    proc = subprocess.run(cmd, cwd=cwd, text=True, capture_output=True)
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr or proc.stdout or f"failed: {cmd}\n")
        raise SystemExit(proc.returncode)
    return proc.stdout


def cccc_bin(root: Path) -> str:
    return run(["just", "_cccc-bin"], cwd=root).strip()


def cccc_top(root: Path, top: int, min_score: int) -> list[FnRow]:
    bin_path = cccc_bin(root)
    args = [bin_path, "--top-cognitive", str(top), "--min", str(min_score)]
    for ex in CCCC_EXCLUDES:
        args.extend(["--exclude", ex])
    args.extend(SCAN_ROOTS)
    data = json.loads(run(args, cwd=root))
    rows: list[FnRow] = []
    for item in data.get("top", []):
        rows.append(
            FnRow(
                path=item["path"],
                name=item["name"],
                line=int(item["line"]),
                cognitive=int(item["cognitive"]),
                cyclomatic=int(item["cyclomatic"]),
            )
        )
    return rows


def cccc_tui_gate(root: Path) -> tuple[list[FnRow], list[FnRow]]:
    bin_path = cccc_bin(root)
    data = json.loads(run([bin_path, "--min", "1", *TUI_ENTRY_FILES], cwd=root))
    all_fns: list[FnRow] = []
    for f in data.get("files", []):
        for fn in f.get("functions", []):
            all_fns.append(
                FnRow(
                    path=f["path"],
                    name=fn["name"],
                    line=int(fn["line"]),
                    cognitive=int(fn["cognitive"]),
                    cyclomatic=int(fn["cyclomatic"]),
                )
            )
    violations = [
        r
        for r in all_fns
        if r.cognitive > MAX_COGNITIVE or r.cyclomatic > MAX_CYCLOMATIC
    ]
    return all_fns, violations


def crap_top(root: Path, top: int, min_cc: float) -> list[dict]:
    proc = subprocess.run(
        [
            "cargo",
            "crap",
            "--workspace",
            "--format",
            "json",
            "--top",
            str(top),
            "--min",
            str(min_cc),
        ],
        cwd=root,
        text=True,
        capture_output=True,
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr or proc.stdout or "cargo crap failed\n")
        raise SystemExit(proc.returncode)
    data = json.loads(proc.stdout)
    return list(data.get("entries", []))


def crap_index(entries: list[dict]) -> dict[tuple[str, int, str], dict]:
    out: dict[tuple[str, int, str], dict] = {}
    for e in entries:
        key = (e["file"], int(e["line"]), e["function"])
        out[key] = e
    return out


def main() -> None:
    root = repo_root()
    out_root = root / ".tmp" / "lab_complexity_compare"

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--top", type=int, default=25, help="Top-N per tool")
    parser.add_argument("--min", type=int, default=10, help="cccc --min / crap floor")
    parser.add_argument(
        "--write",
        action="store_true",
        help=f"Write JSON snapshot under {out_root}/",
    )
    args = parser.parse_args()

    ccc_top = cccc_top(root, args.top, args.min)
    crap_entries = crap_top(root, args.top, args.min)
    crap_by_key = crap_index(crap_entries)

    ccc_keys = {r.key for r in ccc_top}
    crap_keys = {(e["file"], int(e["line"]), e["function"]) for e in crap_entries}
    overlap = ccc_keys & crap_keys

    tui_all, tui_viol = cccc_tui_gate(root)
    tui_crap = crap_index(
        json.loads(
            run(["cargo", "crap", "-p", "xylitol-app-tui", "--format", "json"], cwd=root)
        ).get("entries", [])
    )
    tui_cc_viol = 0
    for row in tui_all:
        ce = tui_crap.get(row.key)
        if ce and float(ce.get("cyclomatic", 0)) > MAX_CYCLOMATIC:
            tui_cc_viol += 1

    print("## cccc vs cargo-crap (lab_complexity_compare)\n")
    print(f"- Top-{args.top} overlap (same file+line+name): **{len(overlap)}** / {args.top}")
    print(
        f"- TUI ath12 entry scope: {len(tui_all)} functions, "
        f"cccc gate violations (cog>{MAX_COGNITIVE} or cyc>{MAX_CYCLOMATIC}): **{len(tui_viol)}**, "
        f"crap CC>{MAX_CYCLOMATIC} on matched: **{tui_cc_viol}**"
    )
    print(
        "\n**Verdict:** keep cccc HARD gate (cognitive); "
        "use CRAP+duplicates for review/CI; see complexity-cccc-crap.md."
    )

    print("\n### cccc top cognitive\n")
    print("| cog | cyc | location | function |")
    print("| ---: | ---: | --- | --- |")
    for r in ccc_top[: min(15, len(ccc_top))]:
        print(f"| {r.cognitive} | {r.cyclomatic} | `{r.path}:{r.line}` | {r.name} |")

    print("\n### cargo-crap top cyclomatic (no LCOV → pessimistic CRAP)\n")
    print("| CC | CRAP | location | function |")
    print("| ---: | ---: | --- | --- |")
    for e in crap_entries[: min(15, len(crap_entries))]:
        print(
            f"| {e['cyclomatic']:.0f} | {e['crap']:.0f} | "
            f"`{e['file']}:{e['line']}` | {e['function']} |"
        )

    if overlap:
        print("\n### In both top lists\n")
        for r in ccc_top:
            if r.key not in overlap:
                continue
            ce = crap_by_key[r.key]
            print(
                f"- `{r.path}:{r.line}` **{r.name}** — "
                f"cog={r.cognitive} cccc_cyc={r.cyclomatic} crap_cc={ce['cyclomatic']:.0f}"
            )

    if args.write:
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        out_dir = out_root / stamp
        out_dir.mkdir(parents=True, exist_ok=True)
        payload = {
            "top": args.top,
            "min": args.min,
            "overlap_count": len(overlap),
            "cccc_top": [r.__dict__ for r in ccc_top],
            "crap_top": crap_entries,
            "tui_entry_violations_cccc": [r.__dict__ for r in tui_viol],
        }
        (out_dir / "summary.json").write_text(
            json.dumps(payload, indent=2) + "\n", encoding="utf-8"
        )
        print(f"\n(wrote {out_dir / 'summary.json'})")


if __name__ == "__main__":
    main()
