"""Dataset loading and selection."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

from .config import EvalConfig
from .errors import InfraError
from .paths import resolve_path


def load_dataset(config: EvalConfig, cache_root: Path | None = None) -> list[dict[str, Any]]:
    name = config.dataset.name
    suffix = Path(name).suffix.lower()
    if suffix in {".jsonl", ".json", ".parquet"}:
        rows = _load_local(resolve_path(name))
    elif cache_root is not None:
        cache_path = _cache_path(cache_root, name, config.dataset.revision)
        if cache_path.is_file():
            rows = _load_local(cache_path)
        else:
            rows = _load_hf(name, config.dataset.split, config.dataset.revision)
            _write_cache(cache_path, rows)
    else:
        rows = _load_hf(name, config.dataset.split, config.dataset.revision)

    normalized = [_normalize(row) for row in rows]
    by_id = {row["instance_id"]: row for row in normalized}

    if config.dataset.instance_ids:
        missing = [iid for iid in config.dataset.instance_ids if iid not in by_id]
        if missing:
            raise InfraError(f"instance ids not found in dataset: {', '.join(missing)}")
        selected = [by_id[iid] for iid in config.dataset.instance_ids]
    else:
        selected = normalized[: config.dataset.limit or len(normalized)]

    if config.dataset.limit:
        selected = selected[: config.dataset.limit]
    return selected


def _cache_path(cache_root: Path, name: str, revision: str) -> Path:
    digest = hashlib.sha256(f"{name}@{revision}".encode("utf-8")).hexdigest()[:16]
    return cache_root / f"{digest}.jsonl"


def _write_cache(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in rows),
        encoding="utf-8",
    )


def _load_local(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        raise InfraError(f"dataset file not found: {path}")
    if path.suffix.lower() == ".jsonl":
        rows = []
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                rows.append(json.loads(line))
        return rows
    if path.suffix.lower() == ".json":
        data = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(data, list):
            return data
        if isinstance(data, dict):
            for key in ("instances", "rows", "data"):
                value = data.get(key)
                if isinstance(value, list):
                    return value
        raise InfraError(f"unsupported JSON dataset shape: {path}")
    try:
        from datasets import load_dataset as hf_load_dataset
    except ImportError as exc:
        raise InfraError("datasets is required to read parquet; install the harness extra") from exc
    ds = hf_load_dataset("parquet", data_files=str(path), split="train")
    return [dict(row) for row in ds]


def _load_hf(name: str, split: str, revision: str) -> list[dict[str, Any]]:
    try:
        from datasets import load_dataset as hf_load_dataset
    except ImportError as exc:
        raise InfraError("datasets is required to load HF datasets; install the harness extra") from exc
    try:
        ds = hf_load_dataset(name, split=split, revision=revision)
    except Exception as exc:  # datasets raises many exception types
        raise InfraError(f"failed to load dataset {name}@{revision}: {exc}") from exc
    return [dict(row) for row in ds]


def _normalize(row: dict[str, Any]) -> dict[str, Any]:
    instance_id = row.get("instance_id")
    image = row.get("image")
    if not instance_id:
        raise InfraError("dataset row missing instance_id")
    if not image:
        raise InfraError(f"dataset row {instance_id} missing image")
    out = dict(row)
    out["instance_id"] = str(instance_id)
    out["image"] = str(image)
    out["problem_statement"] = str(row.get("problem_statement", ""))
    return out
