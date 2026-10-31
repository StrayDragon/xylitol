"""Official SWE-bench grader adapter."""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .errors import InfraError


@dataclass
class GradeResult:
    report_path: Path
    report: dict[str, Any]
    exit_code: int
    stdout: str
    stderr: str


class SwebenchGrader:
    def grade(
        self,
        predictions_path: Path,
        run_id: str,
        instances_path: Path,
        model_name: str,
        report_dir: Path,
        max_workers: int = 2,
        timeout: float = 7200,
    ) -> GradeResult:
        report_dir.mkdir(parents=True, exist_ok=True)
        cmd = [
            sys.executable,
            "-m",
            "swebench.harness.run_evaluation",
            "--dataset_name",
            str(instances_path),
            "--predictions_path",
            str(predictions_path),
            "--run_id",
            run_id,
            "--max_workers",
            str(max_workers),
            "--report_dir",
            str(report_dir),
        ]
        try:
            completed = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                check=False,
                cwd=report_dir.parent,
            )
        except subprocess.TimeoutExpired as exc:
            raise InfraError(f"grader timed out after {timeout}s") from exc
        report_path = report_dir / f"{model_name.replace('/', '__')}.{run_id}.json"
        if not report_path.is_file():
            raise InfraError(
                "grader report not found; "
                f"exit={completed.returncode}; stderr={completed.stderr.strip()[:500]}"
            )
        report = json.loads(report_path.read_text(encoding="utf-8"))
        return GradeResult(
            report_path=report_path,
            report=report,
            exit_code=completed.returncode,
            stdout=completed.stdout,
            stderr=completed.stderr,
        )
