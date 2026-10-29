#!/usr/bin/env python3
"""QA check: target crate graph membership and forbidden Cargo edges.

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

REQUIRED_MEMBERS: dict[str, str] = {
    "crates/xylitol-plugin-sdk": "xylitol-plugin-sdk",
    "crates/xylitol-protocol": "xylitol-protocol",
    "crates/xylitol-wire": "xylitol-wire",
    "crates/xylitol-runtime": "xylitol-runtime",
    "crates/xylitol-serve": "xylitol-serve",
    "crates/xylitol-app-tui": "xylitol-app-tui",
    "crates/xylitol-app-print": "xylitol-app-print",
    "crates/xylitol-tui": "xylitol-tui",
    "crates/xylitol-kernel": "xylitol-kernel",
    "plugins/xylitol-official-plugin-session": "xylitol-official-plugin-session",
    "plugins/xylitol-official-plugin-compaction": "xylitol-official-plugin-compaction",
    "plugins/xylitol-official-plugin-provider-openai": "xylitol-official-plugin-provider-openai",
    "plugins/xylitol-official-plugin-provider-anthropic": "xylitol-official-plugin-provider-anthropic",
    "plugins/xylitol-official-plugin-tools": "xylitol-official-plugin-tools",
    "plugins/xylitol-official-plugin-tools-fff": "xylitol-official-plugin-tools-fff",
    "plugins/xylitol-official-plugin-mcp": "xylitol-official-plugin-mcp",
    "plugins/xylitol-official-plugin-hooks": "xylitol-official-plugin-hooks",
    "plugins/xylitol-official-plugin-config": "xylitol-official-plugin-config",
    "plugins/xylitol-official-plugin-obs": "xylitol-official-plugin-obs",
    "plugins/xylitol-official-plugin-todo": "xylitol-official-plugin-todo",
    "plugins/xylitol-official-plugin-workflow": "xylitol-official-plugin-workflow",
    "plugins/xylitol-official-plugin-plan": "xylitol-official-plugin-plan",
}

ALLOWED_XYLITOL_DEPS: dict[str, frozenset[str]] = {
    "xylitol-plugin-sdk": frozenset(),
    "xylitol-protocol": frozenset(),
    "xylitol-wire": frozenset({"xylitol-protocol", "xylitol-plugin-sdk"}),
    "xylitol-tui": frozenset(),
    "xylitol-runtime": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-serve": frozenset(
        {"xylitol-runtime", "xylitol-protocol", "xylitol-wire", "xylitol-plugin-sdk"}
    ),
    "xylitol-app-print": frozenset({"xylitol-runtime", "xylitol-plugin-sdk"}),
    "xylitol-app-tui": frozenset(
        {"xylitol-protocol", "xylitol-wire", "xylitol-tui", "xylitol-plugin-sdk"}
    ),
    "xylitol-official-plugin-session": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-compaction": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-provider-openai": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-provider-anthropic": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-tools": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-tools-fff": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-mcp": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-hooks": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-loop-policies": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-todo": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-workflow": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-plan": frozenset({"xylitol-plugin-sdk"}),
    "xylitol-official-plugin-config": frozenset(
        {
            "xylitol-plugin-sdk",
            "xylitol-runtime",
            "xylitol-official-plugin-hooks",
            "xylitol-official-plugin-mcp",
            "xylitol-official-plugin-loop-policies",
            "xylitol-official-plugin-todo",
            "xylitol-official-plugin-workflow",
            "xylitol-official-plugin-plan",
        }
    ),
    "xylitol-official-plugin-obs": frozenset(
        {"xylitol-plugin-sdk", "xylitol-official-plugin-config"}
    ),
    "xylitol-kernel": frozenset(
        {
            "xylitol-plugin-sdk",
            "xylitol-runtime",
            "xylitol-protocol",
            "xylitol-serve",
            "xylitol-app-print",
            "xylitol-official-plugin-session",
            "xylitol-official-plugin-compaction",
            "xylitol-official-plugin-provider-openai",
            "xylitol-official-plugin-provider-anthropic",
            "xylitol-official-plugin-tools",
            "xylitol-official-plugin-tools-fff",
            "xylitol-official-plugin-mcp",
            "xylitol-official-plugin-hooks",
            "xylitol-official-plugin-loop-policies",
            "xylitol-official-plugin-config",
            "xylitol-official-plugin-obs",
            "xylitol-official-plugin-todo",
            "xylitol-official-plugin-workflow",
            "xylitol-official-plugin-plan",
        }
    ),
}

ALL_PACKAGES = frozenset(ALLOWED_XYLITOL_DEPS) | {"xylitol"}

FORBIDDEN_DEPS: dict[str, frozenset[str]] = {
    "async-openai": ALL_PACKAGES - {"xylitol-official-plugin-provider-openai"},
    "rmcp": ALL_PACKAGES - {"xylitol-official-plugin-mcp"},
    "salvo": ALL_PACKAGES - {"xylitol-serve"},
}

PLUGIN_PREFIX = "xylitol-official-plugin-"
PLUGIN_PEER_EXCEPTIONS: frozenset[tuple[str, str]] = frozenset(
    {
        ("xylitol-official-plugin-config", "xylitol-official-plugin-hooks"),
        ("xylitol-official-plugin-config", "xylitol-official-plugin-mcp"),
        ("xylitol-official-plugin-obs", "xylitol-official-plugin-config"),
        ("xylitol-official-plugin-config", "xylitol-official-plugin-loop-policies"),
        ("xylitol-official-plugin-config", "xylitol-official-plugin-todo"),
        ("xylitol-official-plugin-config", "xylitol-official-plugin-workflow"),
        ("xylitol-official-plugin-config", "xylitol-official-plugin-plan"),
    }
)

RUTIS_ALLOWED = frozenset({"xylitol-plugin-sdk", "xylitol-kernel"})
RUTIS_LOADER_ALLOWED = frozenset({"xylitol-kernel"})


def dep_names(section: dict[str, object] | None) -> set[str]:
    if not section:
        return set()
    return set(section.keys())


def load_toml(path: Path) -> dict[str, object]:
    with path.open("rb") as fh:
        return tomllib.load(fh)


def check_forbidden(pkg: str, deps: set[str], errors: list[str]) -> None:
    for crate, banned_in in FORBIDDEN_DEPS.items():
        if pkg in banned_in and crate in deps:
            errors.append(f"{pkg}: must not depend on {crate}")


def check_plugin_peers(pkg: str, xy_deps: set[str], errors: list[str]) -> None:
    if not pkg.startswith(PLUGIN_PREFIX):
        return
    for dep in sorted(xy_deps):
        if dep.startswith(PLUGIN_PREFIX) and (pkg, dep) not in PLUGIN_PEER_EXCEPTIONS:
            errors.append(
                f"{pkg}: must not use plugin {dep} (plugins talk via services / events)"
            )


def check_rutis(pkg: str, all_deps: set[str], errors: list[str]) -> None:
    if "rutis" in all_deps and pkg not in RUTIS_ALLOWED:
        errors.append(f"{pkg}: must not depend on rutis")
    if "rutis-loader" in all_deps and pkg not in RUTIS_LOADER_ALLOWED:
        errors.append(f"{pkg}: must not depend on rutis-loader")


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
        all_deps = (
            deps
            | dep_names(data.get("dev-dependencies"))  # type: ignore[arg-type]
            | dep_names(data.get("build-dependencies"))  # type: ignore[arg-type]
        )
        xy_deps = {d for d in deps if d.startswith("xylitol-")}
        allowed = ALLOWED_XYLITOL_DEPS[pkg]
        extra = xy_deps - allowed
        if extra:
            errors.append(
                f"{pkg}: xylitol-* deps {sorted(extra)} not allowed "
                f"(max {sorted(allowed)})"
            )
        check_forbidden(pkg, deps, errors)
        check_plugin_peers(pkg, xy_deps, errors)
        check_rutis(pkg, all_deps, errors)

    root_pkg = root.get("package")
    if not isinstance(root_pkg, dict) or root_pkg.get("name") != "xylitol":
        errors.append("root [package].name must be xylitol")
    root_deps = dep_names(root.get("dependencies"))  # type: ignore[arg-type]
    root_all_deps = (
        root_deps
        | dep_names(root.get("dev-dependencies"))  # type: ignore[arg-type]
        | dep_names(root.get("build-dependencies"))  # type: ignore[arg-type]
    )
    check_forbidden("xylitol", root_deps, errors)
    check_rutis("xylitol", root_all_deps, errors)

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
