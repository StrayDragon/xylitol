#!/usr/bin/env python3
"""Dev-environment self-check ("doctor") for machine-local setup state.

Purpose: the next person / next machine sees actionable WARNINGs right in
`just` (bare `just` runs brief mode) and can self-serve, instead of hitting
a confusing failure or reading docs/notifications.

NOT a `check_*` gate: this probes machine-local state (hooks, tools, env
vars, config files), never repo code — it MUST NOT run in `just qa`.

Usage:
  python3 scripts/doctor_dev_env.py            # brief: warnings only (silent when healthy)
  python3 scripts/doctor_dev_env.py --full     # + info items and the ok summary
  python3 scripts/doctor_dev_env.py --strict   # exit 1 when any warning fires (CI-ready)

Maintenance model: one entry per check in CHECKS below. A probe returns
(ok, detail); `detail` prints as the problem line when ok=False and as the
ok line in --full mode. Keep probes fast (<10ms) — bare `just` runs brief
mode on every invocation.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Callable

REPO = Path(__file__).resolve().parent.parent

Probe = Callable[[], "tuple[bool, str]"]


@dataclass(frozen=True)
class Check:
    id: str
    severity: str  # "warn" (needs fixing) | "info" (nice to have)
    fix: str  # copy-pasteable remedy, shown under every non-ok line
    probe: Probe


def _has(cmd: str) -> bool:
    return shutil.which(cmd) is not None


@lru_cache(maxsize=1)
def _hooks_dir() -> Path:
    """Hooks dir git itself would use (handles linked worktrees + core.hooksPath)."""
    out = subprocess.run(
        ["git", "rev-parse", "--git-path", "hooks"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    )
    return (REPO / out.stdout.strip()).resolve()


def _probe_prek() -> tuple[bool, str]:
    if not _has("prek"):
        return False, "prek not on PATH"
    missing = [h for h in ("pre-commit", "commit-msg") if not (_hooks_dir() / h).exists()]
    if missing:
        return False, f"missing prek shim(s): {', '.join(missing)}"
    return True, "prek shims installed (pre-commit, commit-msg)"


def _probe_git_lfs() -> tuple[bool, str]:
    if not _has("git-lfs"):
        return False, "git-lfs not on PATH"
    if not (_hooks_dir() / "pre-push").exists():
        return False, ".git/hooks/pre-push missing — LFS objects would not upload on push"
    return True, "git-lfs present, native pre-push hook installed"


def _probe_nextest() -> tuple[bool, str]:
    if _has("cargo-nextest"):
        return True, "cargo-nextest present"
    return False, "cargo-nextest not on PATH — `just test` falls back to the slower plain cargo-test path"


def _probe_cccc() -> tuple[bool, str]:
    if _has("cccc-rs") or (REPO / ".tools" / "bin" / "cccc-rs").exists():
        return True, "cccc-rs present (complexity gate ready)"
    return False, "cccc-rs missing — the complexity gate lazy-installs on first qa (slow first run)"


def _probe_worktree_target() -> tuple[bool, str]:
    if not (REPO / ".git").is_file():
        return True, "primary worktree — no CARGO_TARGET_DIR isolation needed"
    if os.environ.get("CARGO_TARGET_DIR"):
        return True, f"linked worktree with CARGO_TARGET_DIR={os.environ['CARGO_TARGET_DIR']}"
    return False, (
        "linked worktree without CARGO_TARGET_DIR — sharing a target dir across "
        "worktrees corrupts cargo fingerprints (repo hard rule)"
    )


def _probe_sccache() -> tuple[bool, str]:
    if _has("sccache") or os.environ.get("RUSTC_WRAPPER"):
        return True, "sccache / RUSTC_WRAPPER configured"
    return False, "sccache not configured — optional, but cold builds are much slower"


def _live_provider_path() -> Path:
    base = os.environ.get("XYLITOL_CONFIG_DIR")
    if not base:
        xdg = os.environ.get("XDG_CONFIG_HOME")
        base = f"{xdg}/xylitol" if xdg else "~/.config/xylitol"
    return Path(base).expanduser() / "dev" / "live-provider.yaml"


def _probe_live_provider() -> tuple[bool, str]:
    cfg = _live_provider_path()
    if not cfg.exists():
        return True, f"no live-provider config at {cfg} — `just test-live-provider` will skip (expected on CI)"
    if re.search(r"(?m)^\s*enabled:\s*true\s*(#.*)?$", cfg.read_text(encoding="utf-8")):
        return True, f"live-provider enabled at {cfg} — the live gate will hit the gateway"
    return True, f"live-provider present but disabled at {cfg} — the live gate will skip"


CHECKS: list[Check] = [
    Check("prek", "warn", "just setup", _probe_prek),
    Check("git-lfs", "warn", "git lfs install --local  (or just setup)", _probe_git_lfs),
    Check("nextest", "warn", "cargo install cargo-nextest --locked", _probe_nextest),
    Check("cccc-rs", "warn", "just setup", _probe_cccc),
    Check("worktree-target", "warn", 'eval "$(just cargo-wt-env)"', _probe_worktree_target),
    Check("sccache", "info", "see skill rust-build-tune (cargo install sccache + RUSTC_WRAPPER)", _probe_sccache),
    Check("live-provider", "info", "see `just gen-live-provider-example`", _probe_live_provider),
]


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("--full", action="store_true", help="also print info items and the ok summary")
    ap.add_argument("--strict", action="store_true", help="exit 1 when any warning fires")
    args = ap.parse_args()

    warnings = 0
    for check in CHECKS:
        ok, detail = check.probe()
        if ok:
            if args.full:
                print(f"ok   [{check.id}] {detail}")
            continue
        if check.severity == "warn":
            warnings += 1
            print(f"WARN [{check.id}] {detail}")
            print(f"     fix: {check.fix}")
        elif args.full:
            print(f"info [{check.id}] {detail}")
            print(f"     hint: {check.fix}")
    if args.full:
        print(f"doctor: {warnings} warning(s) across {len(CHECKS)} checks")
    elif warnings:
        print("doctor: run `just doctor --full` for the complete report")
    if args.strict and warnings:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
