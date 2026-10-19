#!/usr/bin/env python3
"""One-shot W2.4 copy: src/agent + trust + utils → xylitol-runtime. Not a QA gate."""

from __future__ import annotations

import shutil
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "src"
DST = REPO / "crates" / "xylitol-runtime" / "src"

SKIP_NAMES = {
    "tests.rs",
}


def copytree(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(
        src,
        dst,
        ignore=shutil.ignore_patterns("tests.rs", "tests"),
    )


def rewrite_text(text: str) -> str:
    replacements = [
        ("crate::protocol::", "xylitol_plugin_sdk::"),
        (
            "xylitol_official_plugin_provider_openai::ObsSessionContext",
            "xylitol_plugin_sdk::ObsSessionContext",
        ),
        (
            "xylitol_official_plugin_provider_openai::provider::langfuse_observation_properties_from",
            "xylitol_plugin_sdk::langfuse_observation_properties_from",
        ),
        (
            "xylitol_official_plugin_provider_openai::provider::trace::",
            "xylitol_plugin_sdk::",
        ),
        (
            "xylitol_official_plugin_provider_openai::provider::obs_session_context",
            "xylitol_plugin_sdk::obs_session_context",
        ),
        (
            "xylitol_official_plugin_provider_openai::provider::tool_wire::to_wire_tool_name",
            "xylitol_plugin_sdk::tool_wire::to_wire_tool_name",
        ),
        (
            "xylitol_official_plugin_compaction::",
            "xylitol_plugin_sdk::",
        ),
        ("crate::agent::compaction::orchestrator::CompactionOrchestrator", "XyCompaction"),
        ("crate::agent::compaction::CompactionOrchestrator", "XyCompaction"),
        (
            "xylitol_official_plugin_provider_openai::provider::{ObsSessionScope, set_obs_session}",
            "xylitol_plugin_sdk::{ObsSessionScope, set_obs_session}",
        ),
    ]
    for old, new in replacements:
        text = text.replace(old, new)
    return text


def rewrite_tree(root: Path) -> None:
    for path in root.rglob("*.rs"):
        text = path.read_text()
        new = rewrite_text(text)
        if new != text:
            path.write_text(new)


def main() -> None:
    DST.mkdir(parents=True, exist_ok=True)

    copytree(SRC / "agent", DST / "agent")
    compaction = DST / "agent" / "compaction"
    if compaction.exists():
        shutil.rmtree(compaction)

    copytree(SRC / "infra" / "trust", DST / "trust")
    (DST / "utils").mkdir(exist_ok=True)
    shutil.copy(SRC / "utils" / "sync.rs", DST / "utils" / "sync.rs")
    shutil.copy(SRC / "utils" / "text.rs", DST / "utils" / "text.rs")
    shutil.copy(SRC / "utils" / "thought_clock.rs", DST / "utils" / "thought_clock.rs")

    rewrite_tree(DST)
    print("copied + rewrote", DST)


if __name__ == "__main__":
    main()
