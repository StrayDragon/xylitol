#!/usr/bin/env python3
"""
Harness to programmatically clean up `#![allow(dead_code)]` crate-level attributes.

Strategy:
1. Find all Rust files with `#![allow(dead_code)]`.
2. For each file, remove the attribute and run `cargo check` under BOTH the
   lib+bins target set (matches `cargo clippy` as run by `just lint`) AND
   `--all-targets` (tests/examples); union the warnings. Checking both is
   required because a symbol referenced ONLY from tests is "alive" under
   `--all-targets` yet DEAD under the lib/bins view that CI's clippy
   enforces — checking only `--all-targets` yields false "safe to remove".
3. If either view introduces new dead_code/unused warnings originating from
   this file, restore the attribute.
4. Otherwise keep the removal and report it as cleaned.

Run from repo root:
    python3 scripts/cleanup_dead_code_allow.py
"""

import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
ALLOW_PATTERN = re.compile(r"^#!\[allow\(dead_code\)\]\s*\n", re.MULTILINE)


@dataclass
class Candidate:
    path: Path
    original: str


def find_candidates() -> list[Candidate]:
    candidates = []
    for rust_file in REPO_ROOT.rglob("*.rs"):
        if rust_file.is_relative_to(REPO_ROOT / "target"):
            continue
        text = rust_file.read_text()
        if ALLOW_PATTERN.search(text):
            candidates.append(Candidate(path=rust_file, original=text))
    return sorted(candidates, key=lambda c: str(c.path))


def run_cargo_check() -> tuple[int, str]:
    # Run BOTH the lib+bins target set (matches `cargo clippy` / `just lint`)
    # AND `--all-targets`, unioning the output. A symbol referenced only from
    # tests is "alive" under --all-targets but DEAD under the lib/bins view CI
    # enforces; checking only --all-targets masks such test-only dead code and
    # yields false "safe to remove" verdicts. Non-zero if either run fails.
    outputs: list[str] = []
    rc = 0
    for extra in (["--lib", "--bins"], ["--all-targets"]):
        proc = subprocess.run(
            ["cargo", "check", *extra, "--message-format=short"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        rc = rc or proc.returncode
        outputs.append(proc.stdout)
        outputs.append(proc.stderr)
    return rc, "\n".join(outputs)


def warnings_for_file(output: str, rel_path: str) -> list[str]:
    """Extract warning lines that mention the given file path."""
    matching = []
    for line in output.splitlines():
        lowered = line.lower()
        if (
            "warning" in lowered
            and rel_path in line
            and ("dead_code" in lowered or "unused" in lowered)
        ):
            matching.append(line)
    return matching


def main() -> int:
    candidates = find_candidates()
    if not candidates:
        print("No `#![allow(dead_code)]` attributes found.")
        return 0

    print(f"Found {len(candidates)} file(s) with `#![allow(dead_code)]`:\n")
    for c in candidates:
        print(f"  - {c.path.relative_to(REPO_ROOT)}")

    print("\nBaseline check...")
    base_code, base_output = run_cargo_check()
    if base_code != 0:
        print("ERROR: baseline `cargo check --lib` is failing; fix build first.")
        print(base_output[-2000:])
        return 1
    print("Baseline clean.\n")

    cleaned: list[Path] = []
    kept: list[tuple[Path, list[str]]] = []

    for candidate in candidates:
        rel = str(candidate.path.relative_to(REPO_ROOT))
        print(f"Trying {rel} ...", end=" ")

        stripped = ALLOW_PATTERN.sub("", candidate.original, count=1)
        candidate.path.write_text(stripped)

        code, output = run_cargo_check()
        file_warnings = warnings_for_file(output, rel)

        if code != 0 or file_warnings:
            candidate.path.write_text(candidate.original)
            print("KEEP attribute")
            if file_warnings:
                kept.append((candidate.path, file_warnings))
        else:
            print("REMOVE attribute")
            cleaned.append(candidate.path)

    print("\n" + "=" * 60)
    print(f"REMOVED (#![allow(dead_code)] no longer needed): {len(cleaned)}")
    for p in cleaned:
        print(f"  - {p.relative_to(REPO_ROOT)}")

    print(f"\nKEPT (attribute still required): {len(kept)}")
    for p, warnings in kept:
        print(f"  - {p.relative_to(REPO_ROOT)}")
        for w in warnings[:3]:
            print(f"      {w.strip()}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
