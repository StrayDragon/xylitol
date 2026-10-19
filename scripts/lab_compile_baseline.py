#!/usr/bin/env python3
"""TEMPORARY W0.1 compile baseline: du + wall clock + cargo --timings.

Not in `just qa`. Re-run after crate-split batches to compare.

**Lifetime**: `_TODO.md` 脊柱上的 W* 全部收口后 MUST 删除本文件，或按
xylitol-dev cleanup-triage 把当次 summary 塞进对应 change `research/` 再 freeze。
禁止升格为 just recipe / qa 门。

  python3 scripts/lab_compile_baseline.py
  python3 scripts/lab_compile_baseline.py --cold   # clean workspace members, then build

Writes `.tmp/lab_compile_baseline/<utc>/` (gitignored): summary.json + cargo timing html.
Prints a short table to stdout.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_ROOT = ROOT / ".tmp" / "lab_compile_baseline"
MEMBERS = ("xylitol", "xylitol-tui", "xylitol-official-plugin-provider-openai")


def run(cmd: list[str], *, cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd, text=True, capture_output=True)


def du_bytes(path: Path) -> int:
    if not path.exists():
        return 0
    proc = run(["du", "-sb", str(path)], cwd=ROOT)
    if proc.returncode != 0:
        return 0
    return int(proc.stdout.split()[0])


def human(n: int) -> str:
    x = float(n)
    for unit in ("B", "K", "M", "G", "T"):
        if x < 1024 or unit == "T":
            return f"{x:.1f}{unit}" if unit != "B" else f"{int(x)}B"
        x /= 1024
    return f"{n}B"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--cold",
        action="store_true",
        help="cargo clean -p workspace members before build (not a full target wipe)",
    )
    ap.add_argument(
        "--bin",
        default="xylitol",
        help="binary name for size (default xylitol)",
    )
    args = ap.parse_args()

    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = OUT_ROOT / stamp
    out.mkdir(parents=True, exist_ok=True)

    target = Path(os.environ.get("CARGO_TARGET_DIR") or (ROOT / "target"))
    rustc = run(["rustc", "--version"], cwd=ROOT).stdout.strip()
    cargo_v = run(["cargo", "--version"], cwd=ROOT).stdout.strip()

    du_before = {
        "target": du_bytes(target),
        "debug": du_bytes(target / "debug"),
        "debug/build": du_bytes(target / "debug" / "build"),
        "debug/deps": du_bytes(target / "debug" / "deps"),
    }

    if args.cold:
        clean = ["cargo", "clean"]
        for p in MEMBERS:
            clean.extend(["-p", p])
        proc = run(clean, cwd=ROOT)
        (out / "cargo-clean.log").write_text(proc.stdout + proc.stderr, encoding="utf-8")
        if proc.returncode != 0:
            print(proc.stderr, file=sys.stderr)
            return proc.returncode

    cmd = [
        "cargo",
        "build",
        "--timings",
        "--bin",
        args.bin,
    ]
    t0 = time.perf_counter()
    proc = subprocess.run(cmd, cwd=ROOT)
    wall_s = time.perf_counter() - t0
    if proc.returncode != 0:
        return proc.returncode

    du_after = {
        "target": du_bytes(target),
        "debug": du_bytes(target / "debug"),
        "debug/build": du_bytes(target / "debug" / "build"),
        "debug/deps": du_bytes(target / "debug" / "deps"),
    }
    bin_path = target / "debug" / args.bin
    bin_size = bin_path.stat().st_size if bin_path.is_file() else None

    timing_src = target / "cargo-timings"
    timing_copied = None
    if timing_src.is_dir():
        dest = out / "cargo-timings"
        if dest.exists():
            shutil.rmtree(dest)
        shutil.copytree(timing_src, dest)
        html = sorted(dest.glob("cargo-timing-*.html"))
        timing_copied = str(html[-1].relative_to(ROOT)) if html else str(dest.relative_to(ROOT))

    summary = {
        "utc": stamp,
        "cold": bool(args.cold),
        "RUSTC_WRAPPER": os.environ.get("RUSTC_WRAPPER"),
        "rustc": rustc,
        "cargo": cargo_v,
        "cwd": str(ROOT),
        "CARGO_TARGET_DIR": str(target),
        "cmd": cmd,
        "wall_s": round(wall_s, 2),
        "du_before": du_before,
        "du_after": du_after,
        "bin": str(bin_path) if bin_size is not None else None,
        "bin_bytes": bin_size,
        "timings": timing_copied,
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")

    print(f"utc={stamp} cold={args.cold} wall={wall_s:.1f}s")
    print(f"rustc={rustc}")
    print("du_before  target={t}  debug={d}  build={b}  deps={e}".format(
        t=human(du_before["target"]),
        d=human(du_before["debug"]),
        b=human(du_before["debug/build"]),
        e=human(du_before["debug/deps"]),
    ))
    print("du_after   target={t}  debug={d}  build={b}  deps={e}".format(
        t=human(du_after["target"]),
        d=human(du_after["debug"]),
        b=human(du_after["debug/build"]),
        e=human(du_after["debug/deps"]),
    ))
    if bin_size is not None:
        print(f"bin={bin_path} size={human(bin_size)}")
    if timing_copied:
        print(f"timings={timing_copied}")
    print(f"summary={out / 'summary.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
