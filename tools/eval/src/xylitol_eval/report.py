"""Run report schema and writers."""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1


@dataclass
class InstanceResult:
    instance_id: str
    image: str | None = None
    exit_code: int | None = None
    timed_out: bool = False
    patch_bytes: int = 0
    empty_patch: bool = False
    graded: bool = False
    resolved: bool = False
    failure_class: str | None = None
    wall_seconds: float | None = None
    tokens: dict[str, Any] | None = None
    cost_usd: float | None = None
    logs: dict[str, str] | None = None


@dataclass
class RunReport:
    run_id: str
    phase: str
    started_at: str
    finished_at: str
    xylitol: dict[str, Any]
    harness: dict[str, Any]
    dataset: dict[str, Any]
    model: dict[str, Any]
    config: dict[str, Any]
    limits: dict[str, Any]
    instances: list[InstanceResult] = field(default_factory=list)
    summary: dict[str, Any] = field(default_factory=dict)
    schema_version: int = SCHEMA_VERSION

    def to_dict(self) -> dict[str, Any]:
        data = asdict(self)
        if not data["summary"]:
            data["summary"] = self.compute_summary()
        return data

    def compute_summary(self) -> dict[str, Any]:
        completed = sum(1 for item in self.instances if item.graded)
        empty = sum(1 for item in self.instances if item.empty_patch)
        resolved = sum(1 for item in self.instances if item.resolved)
        errors = sum(1 for item in self.instances if item.failure_class == "infra_error")
        return {
            "submitted": len(self.instances),
            "completed": completed,
            "empty_patches": empty,
            "resolved": resolved,
            "errors": errors,
        }


def write_run_json(run_dir: Path, report: RunReport) -> Path:
    run_dir.mkdir(parents=True, exist_ok=True)
    path = run_dir / "run.json"
    path.write_text(
        json.dumps(report.to_dict(), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return path


def write_summary(run_dir: Path, report: RunReport) -> Path:
    run_dir.mkdir(parents=True, exist_ok=True)
    path = run_dir / "summary.md"
    summary = report.compute_summary()
    lines = [
        f"# Eval run {report.run_id}",
        "",
        f"- phase: {report.phase}",
        f"- model: {report.model.get('alias', '')}",
        f"- dataset: {report.dataset.get('name', '')} @ {report.dataset.get('revision', '')}",
        f"- harness: {report.harness.get('name', '')} {report.harness.get('version', '')}",
        "",
        "| instance | exit | timeout | empty patch | graded | resolved | failure |",
        "|---|---|---|---|---|---|---|",
    ]
    for item in report.instances:
        lines.append(
            "| {iid} | {exit_code} | {timeout} | {empty} | {graded} | {resolved} | {failure} |".format(
                iid=item.instance_id,
                exit_code="" if item.exit_code is None else item.exit_code,
                timeout="yes" if item.timed_out else "no",
                empty="yes" if item.empty_patch else "no",
                graded="yes" if item.graded else "no",
                resolved="yes" if item.resolved else "no",
                failure=item.failure_class or "",
            )
        )
    lines.extend(
        [
            "",
            f"submitted: {summary['submitted']}",
            f"completed: {summary['completed']}",
            f"empty patches: {summary['empty_patches']}",
            f"resolved: {summary['resolved']}",
            f"errors: {summary['errors']}",
            "",
        ]
    )
    path.write_text("\n".join(lines), encoding="utf-8")
    return path
