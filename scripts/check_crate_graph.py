#!/usr/bin/env python3
"""QA check: target crate graph membership and forbidden Cargo edges.

Case: W2 rewrite (tag v0.0.0-pre.old). Empty shells must exist; xylitol-*
dependencies may only follow src/AGENTS.md. Root package `xylitol` and
`xylitol-ai-bridge` stay exempt until their cutover batches.

Usage:
  python3 scripts/check_crate_graph.py
  python3 scripts/check_crate_graph.py --check
  python3 scripts/check_crate_graph.py --check --verbose
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ROOT_MANIFEST = REPO / "Cargo.toml"

# path relative to repo → package name
REQUIRED_MEMBERS: dict[str, str] = {
    "crates/xylitol-plugin-sdk": "xylitol-plugin-sdk",
    "crates/xylitol-protocol": "xylitol-protocol",
    "crates/xylitol-runtime": "xylitol-runtime",
    "crates/xylitol-serve": "xylitol-serve",
    "crates/xylitol-app-tui": "xylitol-app-tui",
    "crates/xylitol-app-print": "xylitol-app-print",
    "crates/xylitol-tui": "xylitol-tui",
    "plugins/xylitol-official-plugin-session": "xylitol-official-plugin-session",
    "plugins/xylitol-official-plugin-compaction": "xylitol-official-plugin-compaction",
    "plugins/xylitol-official-plugin-provider-openai": "xylitol-official-plugin-provider-openai",
    "plugins/xylitol-official-plugin-provider-anthropic": "xylitol-official-plugin-provider-anthropic",
    "plugins/xylitol-official-plugin-tools": "xylitol-official-plugin-tools",
    "plugins/xylitol-official-plugin-mcp": "xylitol-official-plugin-mcp",
    "plugins/xylitol-official-plugin-hooks": "xylitol-official-plugin-hooks",
}

# Maximum allowed xylitol-* Cargo deps (subset is OK for empty shells).
ALLOWED_XYLITOL_DEPS: dict[str, frozenset[str]] = {
    "xylitol-plugin-sdk": frozenset(),
    "xylitol-protocol": frozenset(),
    "xylitol-tui": frozenset(),
    "xylitol-runtime": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-serve": frozenset({"xylitol-runtime", "xylitol-protocol"}),
    "xylitol-app-print": frozenset({"xylitol-runtime"}),
    "xylitol-app-tui": frozenset({"xylitol-protocol", "xylitol-tui"}),
    "xylitol-official-plugin-session": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-compaction": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-provider-openai": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-provider-anthropic": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-tools": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-mcp": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-hooks": frozenset({"xylitol-plugin-sdk"}),
}

# Third-party crates that must not appear in these packages' [dependencies].
FORBIDDEN_DEPS: dict[str, frozenset[str]] = {
    "fory": frozenset(ALLOWED_XYLITOL_DEPS) - {"xylitol-protocol"},
    "async-openai": frozenset(ALLOWED_XYLITOL_DEPS)
    - {"xylitol-official-plugin-provider-openai"},
    "rmcp": frozenset(ALLOWED_XYLITOL_DEPS) - {"xylitol-official-plugin-mcp"},
    "salvo": frozenset(ALLOWED_XYLITOL_DEPS) - {"xylitol-serve"},
}

TRANSITION_EXEMPT = frozenset({"xylitol", "xylitol-ai-bridge"})


def dep_names(section: dict[str, object] | None) -> set[str]:
    if not section:
        return set()
    return set(section.keys())


def load_toml(path: Path) -> dict[str, object]:
    with path.open("rb") as fh:
        return tomllib.load(fh)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="same as default")
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="print ok summary on success",
    )
    args = parser.parse_args()
    errors: list[str] = []

    if not ROOT_MANIFEST.is_file():
        print("error: root Cargo.toml missing", file=sys.stderr)
        return 1

    root = load_toml(ROOT_MANIFEST)
    workspace = root.get("workspace")
    if not isinstance(workspace, dict):
        print("error: root Cargo.toml has no [workspace]", file=sys.stderr)
        return 1
    members = workspace.get("members")
    if not isinstance(members, list):
        print("error: [workspace].members missing", file=sys.stderr)
        return 1
    member_set = {str(m) for m in members}

    for rel, pkg in REQUIRED_MEMBERS.items():
        if rel not in member_set:
            errors.append(f"workspace members missing {rel!r} ({pkg})")
            continue
        manifest = REPO / rel / "Cargo.toml"
        if not manifest.is_file():
            errors.append(f"missing {rel}/Cargo.toml")
            continue
        data = load_toml(manifest)
        package = data.get("package")
        if not isinstance(package, dict) or package.get("name") != pkg:
            errors.append(f"{rel}: [package].name must be {pkg!r}")
            continue
        deps = dep_names(data.get("dependencies"))  # type: ignore[arg-type]
        xy_deps = {d for d in deps if d.startswith("xylitol-")}
        allowed = ALLOWED_XYLITOL_DEPS[pkg]
        extra = xy_deps - allowed
        if extra:
            errors.append(
                f"{pkg}: xylitol-* deps {sorted(extra)} not allowed "
                f"(max {sorted(allowed)})"
            )
        for crate, banned_in in FORBIDDEN_DEPS.items():
            if pkg in banned_in and crate in deps:
                errors.append(f"{pkg}: must not depend on {crate}")

    # Root / ai-bridge stay fat during transition; do not apply FORBIDDEN_DEPS.
    root_pkg = root.get("package")
    if isinstance(root_pkg, dict) and root_pkg.get("name") not in TRANSITION_EXEMPT:
        errors.append("root [package].name must stay xylitol")

    if errors:
        print("error: crate graph gate failed:", file=sys.stderr)
        for item in errors:
            print(f"  {item}", file=sys.stderr)
        return 1

    if args.verbose:
        print(f"ok: {len(REQUIRED_MEMBERS)} target crates; forbidden edges clear")
    return 0


if __name__ == "__main__":
    sys.exit(main())
