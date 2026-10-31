"""Eval run orchestration."""

from __future__ import annotations

import hashlib
import json
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

from .agent import XylitolPrintRunner
from .config import EvalConfig
from .dataset import load_dataset
from .paths import default_cache_root, resolve_path
from .errors import InfraError
from .grade import SwebenchGrader
from .patch import collect_patch, write_prediction
from .report import InstanceResult, RunReport, write_run_json, write_summary
from .sandbox import DockerSandbox


def _utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _git_commit() -> str:
    try:
        completed = subprocess.run(
            ["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=False
        )
    except FileNotFoundError:
        return ""
    return completed.stdout.strip() if completed.returncode == 0 else ""


def _safe_name(value: str) -> str:
    return "".join(ch if ch.isalnum() or ch in "-_." else "_" for ch in value)


def run_eval(config: EvalConfig) -> RunReport:
    binary = resolve_path(config.agent.binary)
    if not binary.is_file():
        raise InfraError(f"agent binary not found: {binary}")
    run_id = config.grader.run_id or time.strftime("run-%Y%m%d-%H%M%S")
    run_dir = resolve_path(config.report.run_dir) / run_id
    run_dir.mkdir(parents=True, exist_ok=True)

    instances = load_dataset(config, cache_root=default_cache_root())
    instances_path = run_dir / "instances.jsonl"
    instances_path.write_text(
        "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in instances),
        encoding="utf-8",
    )
    predictions_path = run_dir / "preds.jsonl"
    predictions_path.write_text("", encoding="utf-8")

    sandbox = DockerSandbox()
    agent = XylitolPrintRunner()
    results: list[InstanceResult] = []
    started_at = _utc_now()

    for instance in instances:
        instance_id = instance["instance_id"]
        image = instance["image"]
        container = f"xylitol-eval-{_safe_name(run_id)}-{_safe_name(instance_id)}"
        result = InstanceResult(instance_id=instance_id, image=image)
        started = time.monotonic()
        try:
            sandbox.ensure_image(image)
            sandbox.start(image, container, {"cpus": config.sandbox.cpus, "memory": config.sandbox.memory})
            run_result = agent.run(instance, sandbox, container, config)
            result.exit_code = run_result.exit_code
            result.timed_out = run_result.timed_out
            result.wall_seconds = round(time.monotonic() - started, 3)
            patch_text = collect_patch(sandbox, container)
            result.patch_bytes = len(patch_text.encode("utf-8"))
            result.empty_patch = not patch_text.strip()
            write_prediction(predictions_path, instance_id, config.model.alias, patch_text)
            logs_dir = run_dir / "logs"
            logs_dir.mkdir(parents=True, exist_ok=True)
            (logs_dir / f"{_safe_name(instance_id)}.stdout.txt").write_text(
                run_result.stdout, encoding="utf-8"
            )
            (logs_dir / f"{_safe_name(instance_id)}.stderr.txt").write_text(
                run_result.stderr, encoding="utf-8"
            )
            result.logs = {
                "stdout": str(logs_dir / f"{_safe_name(instance_id)}.stdout.txt"),
                "stderr": str(logs_dir / f"{_safe_name(instance_id)}.stderr.txt"),
            }
        except Exception as exc:
            result.failure_class = "infra_error"
            result.wall_seconds = round(time.monotonic() - started, 3)
            result.logs = {"error": str(exc)}
        finally:
            sandbox.remove(container)
        results.append(result)

    grade = SwebenchGrader().grade(
        predictions_path=predictions_path,
        run_id=run_id,
        instances_path=instances_path,
        model_name=config.model.alias,
        report_dir=run_dir / "reports",
        max_workers=config.grader.max_workers,
    )
    report = grade.report
    resolved_ids = set(report.get("resolved_ids", []))
    for item in results:
        if item.empty_patch:
            item.failure_class = item.failure_class or "empty_patch"
            continue
        item.graded = item.instance_id in set(report.get("completed_ids", []))
        item.resolved = item.instance_id in resolved_ids
        if item.graded and not item.resolved:
            item.failure_class = item.failure_class or "test_fail"

    summary = RunReport(
        run_id=run_id,
        phase=config.phase,
        started_at=started_at,
        finished_at=_utc_now(),
        xylitol={
            "git_commit": _git_commit(),
            "binary_sha256": _sha256(binary),
            "target": "x86_64-unknown-linux-gnu",
        },
        harness={"name": "swebench", "version": config.grader.version, "run_id": run_id},
        dataset={
            "name": config.dataset.name,
            "revision": config.dataset.revision,
            "split": config.dataset.split,
            "instance_ids": [row["instance_id"] for row in instances],
        },
        model={
            "alias": config.model.alias,
            "provider": config.model.provider,
            "thinking_level": config.model.thinking_level,
            "api": config.model.api,
            "context_window": config.model.context_window,
        },
        config={"path": str(config.path), "sha256": config.sha256},
        limits={
            "max_turns": config.agent.max_turns,
            "wall_seconds": config.agent.wall_seconds,
            "max_workers": config.grader.max_workers,
        },
        instances=results,
    )
    summary.summary = summary.compute_summary()
    write_run_json(run_dir, summary)
    write_summary(run_dir, summary)
    return summary
