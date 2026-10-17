#!/usr/bin/env python3
"""One-shot compile-speed probe. Not wired into `just qa`.

Default: split `target/` disk usage (cheap).
Optional: `cargo check --no-default-features`, or `cargo build -Z timings`.

Examples:
  python3 scripts/lab_build_timings.py
  python3 scripts/lab_build_timings.py --check-no-default
  python3 scripts/lab_build_timings.py --timings
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "target" / "lab_build"


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess[str]:
    print("+", " ".join(cmd), flush=True)
    return subprocess.run(cmd, text=True, **kw)


def du_line(path: Path) -> str:
    if not path.exists():
        return f"(missing) {path}"
    proc = subprocess.run(
        ["du", "-sh", str(path)],
        capture_output=True,
        text=True,
        check=False,
    )
    return (proc.stdout or proc.stderr or "").strip() or f"(empty) {path}"


def split_target() -> None:
    target = REPO / "target"
    print("== target split ==")
    print(du_line(target))
    debug = target / "debug"
    for name in ("build", "incremental", "deps", "."):
        print(du_line(debug / name))
    build = debug / "build"
    if build.is_dir():
        proc = subprocess.run(
            ["bash", "-lc", f"find {build} -mindepth 1 -maxdepth 1 -type d | wc -l"],
            capture_output=True,
            text=True,
            check=False,
        )
        print(f"debug/build top-level dirs: {(proc.stdout or '').strip()}")


def check_no_default() -> int:
    proc = run(
        ["cargo", "check", "-p", "xylitol", "--no-default-features", "--message-format=short"],
        cwd=REPO,
    )
    return proc.returncode


def timings() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    proc = run(
        [
            "cargo",
            "build",
            "-p",
            "xylitol",
            "--bin",
            "xylitol",
            "-Z",
            "timings",
        ],
        cwd=REPO,
    )
    html = list(REPO.glob("cargo-timing*.html"))
    if not html:
        html = list((REPO / "target").glob("**/cargo-timing*.html"))
    for src in html:
        dest = OUT / src.name
        shutil.copy2(src, dest)
        print(f"copied timings html -> {dest}")
    return proc.returncode


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check-no-default", action="store_true")
    ap.add_argument("--timings", action="store_true")
    args = ap.parse_args()
    split_target()
    rc = 0
    if args.check_no_default:
        rc = check_no_default() or rc
    if args.timings:
        rc = timings() or rc
    return rc


if __name__ == "__main__":
    sys.exit(main())
