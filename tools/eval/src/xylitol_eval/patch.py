"""Patch collection and prediction serialization."""

from __future__ import annotations

import json
from pathlib import Path

from .errors import InfraError
from .sandbox import DockerSandbox


def collect_patch(sandbox: DockerSandbox, container: str, timeout: float = 120) -> str:
    sandbox.exec(
        container,
        ["git", "config", "--global", "--add", "safe.directory", "/testbed"],
        timeout=30,
    )
    add = sandbox.exec(container, ["git", "add", "-A"], timeout=timeout)
    if add.exit_code != 0:
        raise InfraError(f"git add failed: {add.stderr.strip()}")
    diff = sandbox.exec(container, ["git", "diff", "--cached"], timeout=timeout)
    if diff.exit_code != 0:
        raise InfraError(f"git diff failed: {diff.stderr.strip()}")
    return diff.stdout


def write_prediction(path: Path, instance_id: str, model_name: str, patch_text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    record = {
        "instance_id": instance_id,
        "model_name_or_path": model_name,
        "model_patch": patch_text,
    }
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, ensure_ascii=False) + "\n")


def read_predictions(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            rows.append(json.loads(line))
    return rows
