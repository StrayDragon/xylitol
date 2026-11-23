#!/usr/bin/env python3
"""Check that the two XyDriver declarations agree on their shared methods.

The runtime port (crates/xylitol-runtime/src/driver/proto.rs) and the app-tui
client mirror (crates/xylitol-app-tui/src/driver/proto.rs) declare the same
trait name. app-tui may not depend on runtime, so the mirror cannot be
generated; the shared subset is kept in step by hand.

This gate compares every method name present on both sides and requires the
async-ness, receiver, parameter list and return type to match exactly.
Methods that exist on only one side are architectural differences and are
checked against an explicit allow-list.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

RUNTIME_PROTO = ROOT / "crates/xylitol-runtime/src/driver/proto.rs"
APP_TUI_PROTO = ROOT / "crates/xylitol-app-tui/src/driver/proto.rs"

RUNTIME_ONLY = {"run_turn", "take_follow_up"}
APP_TUI_ONLY = {
    "assembly_gate_open",
    "attach_observe",
    "attach_session",
    "bind_debug_scene",
    "copy_text_to_clipboard",
    "detach_session",
    "drain_idle_events",
    "exposes_session_store",
    "link_health",
    "read_clipboard_text",
    "refresh_surface_caches",
    "session_store",
    "stage_clipboard_image",
    "take_resync_rebuild",
}

SIGNATURE_RE = re.compile(
    r"^\s*(?P<async>async\s+)?fn\s+(?P<name>[a-z_0-9]+)"
    r"\s*(?:\((?P<params>.*?)\))?"
    r"\s*(?P<ret>->\s*[^{;]+)?"
    r"\s*(?:\{|;|$)"
)


def trait_body(path: Path, trait: str = "XyDriver") -> str:
    source = path.read_text(encoding="utf-8")
    start = source.find(f"pub trait {trait}")
    if start == -1:
        raise SystemExit(f"error: trait {trait} not found in {path}")
    brace = source.find("{", start)
    depth = 0
    for i in range(brace, len(source)):
        ch = source[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return source[brace + 1 : i]
    raise SystemExit(f"error: unbalanced braces for trait {trait} in {path}")


def normalize(text: str) -> str:
    """Strip module qualifiers so Value and serde_json::Value compare equal."""
    text = re.sub(r"\bserde_json::", "", text)
    text = re.sub(r"\bxylitol_plugin_sdk::", "", text)
    text = re.sub(r"\bcrate::", "", text)
    return " ".join(text.split())


def receiver(params: str) -> str:
    first = params.split(",")[0].strip() if params.strip() else ""
    if first.startswith("&mut self"):
        return "&mut self"
    if first.startswith("&self"):
        return "&self"
    if first.startswith("self"):
        return "self"
    return ""


def methods(path: Path) -> dict[str, tuple[bool, str, str, str]]:
    out: dict[str, tuple[bool, str, str, str]] = {}
    body = trait_body(path)
    joined = re.sub(r"\(\s*\n", "(", body)
    joined = re.sub(r",\s*\n\s*", ", ", joined)
    joined = re.sub(r"\n\s*\)", ")", joined)
    for line in joined.splitlines():
        match = SIGNATURE_RE.match(line)
        if not match:
            continue
        params = normalize(match.group("params") or "")
        ret = normalize(match.group("ret") or "")
        out[match.group("name")] = (
            bool(match.group("async")),
            receiver(params),
            params,
            ret,
        )
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero on any mismatch (default behaviour)",
    )
    parser.parse_args()

    runtime = methods(RUNTIME_PROTO)
    app_tui = methods(APP_TUI_PROTO)

    failures: list[str] = []

    for name in sorted(set(runtime) & set(app_tui)):
        if runtime[name] != app_tui[name]:
            failures.append(
                f"{name}: runtime={runtime[name]} app-tui={app_tui[name]}"
            )

    unexpected_runtime = sorted(set(runtime) - set(app_tui) - RUNTIME_ONLY)
    unexpected_app_tui = sorted(set(app_tui) - set(runtime) - APP_TUI_ONLY)
    for name in unexpected_runtime:
        failures.append(f"{name}: runtime-only method not in allow-list")
    for name in unexpected_app_tui:
        failures.append(f"{name}: app-tui-only method not in allow-list")

    stale_runtime = sorted(RUNTIME_ONLY - set(runtime))
    stale_app_tui = sorted(APP_TUI_ONLY - set(app_tui))
    for name in stale_runtime:
        failures.append(f"{name}: listed as runtime-only but absent")
    for name in stale_app_tui:
        failures.append(f"{name}: listed as app-tui-only but absent")

    shared = len(set(runtime) & set(app_tui))
    if failures:
        print(
            f"driver trait parity: {len(failures)} problem(s) "
            f"across {shared} shared method(s)"
        )
        for line in failures:
            print(f"  {line}")
        return 1

    print(
        f"driver trait parity ok: {shared} shared method(s), "
        f"{len(RUNTIME_ONLY)} runtime-only, {len(APP_TUI_ONLY)} app-tui-only"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
