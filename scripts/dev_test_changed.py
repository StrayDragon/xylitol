#!/usr/bin/env python3
"""Run the workspace tests for packages touched by the current diff.

Inner-loop helper, not a gate. `just qa` remains the authoritative matrix.
Root-level/workspace/spec/test changes fall back to the full `just test`.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

FULL_FALLBACK_PREFIXES = (
    "tests/",
    "llmanspec/",
    "scripts/",
    ".github/",
    ".cargo/",
    "docs/",
    "configs/",
)
FULL_FALLBACK_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "justfile",
    ".cargo-crap.toml",
    ".uncomment.toml",
}
ROOT_PACKAGE = "xylitol"
MAX_PACKAGES = 8


def run(args: list[str], *, capture: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=REPO,
        check=True,
        text=True,
        capture_output=capture,
    )


def git_output(args: list[str]) -> str:
    return run(["git", *args], capture=True).stdout.strip()


def merge_base() -> str | None:
    for candidate in ("origin/HEAD", "origin/main", "HEAD~1"):
        try:
            return git_output(["merge-base", "HEAD", candidate])
        except subprocess.CalledProcessError:
            continue
    return None


def changed_files(base: str | None) -> list[str]:
    files: set[str] = set()
    if base:
        out = git_output(["diff", "--name-only", base])
        files.update(line for line in out.splitlines() if line)
    untracked = git_output(["ls-files", "--others", "--exclude-standard"])
    files.update(line for line in untracked.splitlines() if line)
    return sorted(files)


def cargo_metadata() -> dict:
    out = run(["cargo", "metadata", "--format-version", "1"], capture=True).stdout
    return json.loads(out)


def package_for_file(path: str, package_dirs: list[tuple[Path, str]]) -> str | None:
    file = (REPO / path).resolve()
    best: tuple[int, str] | None = None
    for directory, name in package_dirs:
        try:
            file.relative_to(directory)
        except ValueError:
            continue
        depth = len(directory.parts)
        if best is None or depth > best[0]:
            best = (depth, name)
    return best[1] if best else None


def select_packages(metadata: dict, files: list[str]) -> tuple[list[str], bool]:
    if any(path in FULL_FALLBACK_FILES for path in files):
        return [], True
    if any(path.startswith(FULL_FALLBACK_PREFIXES) for path in files):
        return [], True

    rust_files = [path for path in files if path.endswith(".rs")]
    if not rust_files:
        return [], False

    packages = metadata["packages"]
    by_id = {pkg["id"]: pkg["name"] for pkg in packages}
    package_dirs = [
        (Path(pkg["manifest_path"]).parent.resolve(), pkg["name"]) for pkg in packages
    ]

    changed: set[str] = set()
    for path in rust_files:
        name = package_for_file(path, package_dirs)
        if name is None:
            return [], True
        changed.add(name)

    reverse: dict[str, set[str]] = {}
    for node in metadata["resolve"]["nodes"]:
        name = by_id.get(node["id"])
        if name is None:
            continue
        for dep_id in node["dependencies"]:
            dep_name = by_id.get(dep_id)
            if dep_name:
                reverse.setdefault(dep_name, set()).add(name)

    selected = set(changed)
    for name in changed:
        for dependent in reverse.get(name, set()):
            if dependent != ROOT_PACKAGE:
                selected.add(dependent)
    if len(selected) > MAX_PACKAGES:
        return [], True
    return sorted(selected), False


def run_tests(packages: list[str]) -> None:
    if not packages:
        print("no Rust package changes; run `just test` for the full matrix", file=sys.stderr)
        return
    if shutil.which("cargo-nextest"):
        args = [
            "cargo",
            "nextest",
            "run",
            "--all-features",
            "--profile",
            "agent",
            "--show-progress",
            "none",
            "--cargo-quiet",
            "--success-output",
            "never",
        ]
        for package in packages:
            args.extend(["-p", package])
        run(args)
    else:
        args = ["cargo", "test", "-q", "--all-features"]
        for package in packages:
            args.extend(["-p", package])
        run(args)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--print",
        action="store_true",
        help="print the selected packages without running tests",
    )
    args = parser.parse_args()

    base = merge_base()
    files = changed_files(base)
    metadata = cargo_metadata()
    packages, full = select_packages(metadata, files)

    if full:
        if args.print:
            print("full")
            return 0
        print("workspace/root-level change detected; running full `just test`", file=sys.stderr)
        run(["just", "test"])
        return 0

    if args.print:
        print(" ".join(packages) if packages else "none")
        return 0

    print(f"test-changed: {', '.join(packages) if packages else 'no Rust changes'}", file=sys.stderr)
    run_tests(packages)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except subprocess.CalledProcessError as error:
        sys.exit(error.returncode)
