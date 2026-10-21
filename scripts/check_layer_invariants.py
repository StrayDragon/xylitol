#!/usr/bin/env python3
"""QA check: layer invariants formerly enforced by BDD source probes.

Architecture edges belong here (or in check_crate_graph.py), not in BDD
`la_load("src/…")` steps. Case: D5 retire steps_layer_probe.

Usage:
  python3 scripts/check_layer_invariants.py
  python3 scripts/check_layer_invariants.py --check
  python3 scripts/check_layer_invariants.py --check --verbose
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ROOTSRC = frozenset({"main.rs", "lib.rs", "cli.rs", "embed.rs"})
XY_EXPORTS = ("XyModel", "XyTool", "XySessionStore", "XyEvent", "XyChunk")
TRACING_CRATES = frozenset({"tracing", "tracing-subscriber", "fastrace-tracing"})
DEFAULT_FEATURES = ("cli", "tui", "otel", "server", "mcp")
OPTIONAL_FEATURES = ("cli", "tui", "otel", "server", "mcp")
BUILTIN_NO_FLAG = ("tools", "hooks", "security", "print-mode")
PI_REF_FILES = (
    "crates/xylitol-protocol/src/envelope.rs",
    "crates/xylitol-plugin-sdk/src/model/meta.rs",
    "crates/xylitol-runtime/src/agent/runtime/react/mod.rs",
)


def read(rel: str) -> str:
    return (REPO / rel).read_text(encoding="utf-8")


def must_exist(rel: str, errors: list[str]) -> Path | None:
    path = REPO / rel
    if not path.is_file() and not path.is_dir():
        errors.append(f"missing {rel}")
        return None
    return path


def must_not_exist(rel: str, errors: list[str]) -> None:
    if (REPO / rel).exists():
        errors.append(f"forbidden path present: {rel}")


def walk_rs(root: Path):
    if not root.exists():
        return
    for path in root.rglob("*.rs"):
        yield path


def check_rootsrc(errors: list[str]) -> None:
    files = {p.name for p in (REPO / "src").glob("*.rs")}
    extra = files - ROOTSRC
    missing = ROOTSRC - files
    if extra:
        errors.append(f"src/*.rs extra files {sorted(extra)} (ROOTSRC is {sorted(ROOTSRC)})")
    if missing:
        errors.append(f"src/*.rs missing {sorted(missing)}")


def check_features(errors: list[str]) -> None:
    data = tomllib.loads(read("Cargo.toml"))
    features = data.get("features")
    if not isinstance(features, dict):
        errors.append("root Cargo.toml missing [features]")
        return
    default = features.get("default")
    if not isinstance(default, list):
        errors.append("[features].default must be a list")
        return
    default_set = {str(x) for x in default}
    for item in DEFAULT_FEATURES:
        if item not in default_set:
            errors.append(f"default features missing {item}")
    for extra in ("postgres", "sqlite"):
        if extra in default_set:
            errors.append(f"default features must not include {extra}")
    for flag in OPTIONAL_FEATURES:
        if flag not in features:
            errors.append(f"optional feature {flag} missing")
    for builtin in BUILTIN_NO_FLAG:
        if builtin in features:
            errors.append(f"builtin {builtin} must not be a feature flag")


def check_tracing(errors: list[str]) -> None:
    root = tomllib.loads(read("Cargo.toml"))
    workspace = root.get("workspace")
    deps = {}
    if isinstance(workspace, dict):
        wd = workspace.get("dependencies")
        if isinstance(wd, dict):
            deps.update(wd)
    rd = root.get("dependencies")
    if isinstance(rd, dict):
        deps.update(rd)
    for crate in TRACING_CRATES:
        if crate in deps:
            errors.append(f"workspace/root must not depend on {crate}")


def check_runtime_serve_plugins(errors: list[str]) -> None:
    for path in walk_rs(REPO / "crates/xylitol-serve/src"):
        text = path.read_text(encoding="utf-8")
        if "xylitol_official_plugin" in text:
            errors.append(f"{path.relative_to(REPO)}: serve must not use official plugins")
    for path in walk_rs(REPO / "crates/xylitol-runtime/src"):
        if path.name == "tests.rs" or "tests" in path.parts:
            continue
        for line in path.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if stripped.startswith("//") or stripped.startswith("use ") and "test" in path.name:
                continue
            if stripped.startswith("use xylitol_protocol") or " xylitol_protocol::" in line:
                errors.append(f"{path.relative_to(REPO)}: runtime must not use xylitol_protocol")
                break


def check_moved_occupancy(errors: list[str]) -> None:
    must_exist("crates/xylitol-serve/src/occupancy/host.rs", errors)
    must_not_exist("src/wiring.rs", errors)
    must_not_exist("src/wiring", errors)
    must_exist("crates/xylitol-app-tui/src/driver/remote.rs", errors)
    must_exist("crates/xylitol-kernel/src/occupancy/factory.rs", errors)


def check_docs_and_exports(errors: list[str]) -> None:
    agents = read("src/AGENTS.md")
    if "AGENTS.md" not in agents:
        errors.append("src/AGENTS.md must name AGENTS.md as architecture SSOT")
    if "protocol" not in agents or "runtime" not in agents:
        errors.append("src/AGENTS.md must describe protocol/runtime layers")
    lib = read("src/lib.rs")
    for name in XY_EXPORTS:
        if name not in lib:
            errors.append(f"src/lib.rs must re-export {name}")
    hits = 0
    for rel in PI_REF_FILES:
        path = must_exist(rel, errors)
        if path is None:
            continue
        hits += sum(1 for line in path.read_text(encoding="utf-8").splitlines() if "pi coding agent" in line)
    if hits:
        errors.append(f"pi coding agent refs must be 0, got {hits}")


def check_just_and_tokens(errors: list[str]) -> None:
    just = read("justfile")
    for needle in ("qa-check-tui-tokens", "qa-check-complexity", "test-tui-e2e"):
        if needle not in just:
            errors.append(f"justfile missing {needle}")
    must_exist("scripts/check_tui_tokens.py", errors)
    must_exist("crates/xylitol-app-tui/src/DESIGN.md", errors)


def check_tests_tmp(errors: list[str]) -> None:
    hits: list[str] = []
    needle = '"/tmp'
    writers = ("fs::write", "fs::create_dir", "File::create", "fs::remove", "create_dir_all")
    for path in walk_rs(REPO / "tests"):
        text = path.read_text(encoding="utf-8")
        if needle not in text:
            continue
        for line in text.splitlines():
            trimmed = line.strip()
            if needle in trimmed and any(w in trimmed for w in writers):
                hits.append(f"{path.relative_to(REPO)}: {trimmed}")
    if hits:
        errors.append("tests must not write fixed /tmp paths:\n  " + "\n  ".join(hits))


def check_print_thinking_wrap(errors: list[str]) -> None:
    src = read("crates/xylitol-app-print/src/print.rs")
    if src.count('write!(io::stderr(), "<think>"') != 1:
        errors.append("print.rs must write <think> to stderr exactly once")
    if src.count('write!(io::stderr(), "</think>"') != 1:
        errors.append("print.rs must write </think> to stderr exactly once")
    if "thinking_has_tags" not in src:
        errors.append("print.rs must guard tagged thinking")


def check_contains(errors: list[str]) -> None:
    pairs = [
        ("crates/xylitol-runtime/src/agent/prompt/templates/default_system.j2", "Available tools:"),
        ("crates/xylitol-runtime/src/agent/prompt/session_env.rs", "session_env"),
        ("crates/xylitol-app-tui/src/driver/remote.rs", "Command::SessionTree"),
        ("crates/xylitol-protocol/src/registry.rs", '"list_sessions"'),
        ("crates/xylitol-protocol/src/registry.rs", '"session_tree"'),
        ("crates/xylitol-protocol/src/registry.rs", '"reload"'),
        ("crates/xylitol-serve/src/occupancy/host.rs", "SessionSlot"),
        ("crates/xylitol-serve/src/occupancy/host.rs", "materialize_writer"),
        ("crates/xylitol-kernel/src/driver/in_process/mod.rs", "SessionExporter"),
        ("tests/bdd.rs", "mod bdd"),
    ]
    for rel, needle in pairs:
        path = REPO / rel
        if not path.is_file():
            errors.append(f"missing {rel} (needed for {needle!r})")
            continue
        text = path.read_text(encoding="utf-8")
        if needle not in text:
            if rel == "tests/bdd.rs" and needle == "mod bdd":
                if "suite" not in text:
                    errors.append("tests/bdd.rs must be the isolated BDD target")
                continue
            errors.append(f"{rel} must contain {needle!r}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    errors: list[str] = []
    check_rootsrc(errors)
    check_features(errors)
    check_tracing(errors)
    check_runtime_serve_plugins(errors)
    check_moved_occupancy(errors)
    check_docs_and_exports(errors)
    check_just_and_tokens(errors)
    check_tests_tmp(errors)
    check_print_thinking_wrap(errors)
    check_contains(errors)
    if errors:
        print("error: layer invariants failed:", file=sys.stderr)
        for item in errors:
            print(f"  {item}", file=sys.stderr)
        return 1
    if args.verbose:
        print("ok: layer invariants")
    return 0


if __name__ == "__main__":
    sys.exit(main())
