"""Run and cache paths."""

from __future__ import annotations

import os
from pathlib import Path


def repo_root() -> Path:
    start = Path.cwd().resolve()
    for candidate in (start, *start.parents):
        if (candidate / "Cargo.toml").is_file():
            return candidate
    return start


def resolve_path(value: str) -> Path:
    path = Path(value)
    if path.is_absolute():
        return path
    return repo_root() / path


def default_run_root() -> Path:
    override = os.environ.get("XYLITOL_EVAL_RUN_ROOT")
    if override:
        return Path(override)
    return repo_root() / "target" / "eval-runs"


def default_cache_root() -> Path:
    override = os.environ.get("XYLITOL_EVAL_CACHE_ROOT")
    if override:
        return Path(override)
    return repo_root() / "target" / "eval-cache"


def run_dir(root: Path, run_id: str) -> Path:
    if not run_id:
        raise ValueError("run_id must not be empty")
    return root / run_id
