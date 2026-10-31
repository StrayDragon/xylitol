"""Stable ports for eval adapters.

M0 implements DockerSandbox, XylitolPrintRunner, SwebenchGrader and
SwebenchDataset. Later phases implement Harbor, Langfuse regression and cloud
sandboxes against the same shapes.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any, Protocol


class DatasetSource(Protocol):
    def load(self, config: Any) -> list[dict[str, Any]]:
        """Return dataset instances with instance_id and image."""


class Sandbox(Protocol):
    def ensure_image(self, image: str) -> None: ...
    def start(self, image: str, limits: dict[str, Any]) -> Any: ...
    def exec(self, container: Any, argv: list[str], timeout: float) -> Any: ...
    def copy_in(self, container: Any, src: Path, dst: str) -> None: ...
    def remove(self, container: Any) -> None: ...


class AgentRunner(Protocol):
    def run(self, instance: dict[str, Any], sandbox: Sandbox, config: Any) -> Any: ...


class Grader(Protocol):
    def grade(self, predictions_path: Path, run_id: str, instances: list[dict[str, Any]]) -> Any: ...


class Reporter(Protocol):
    def write(self, run_dir: Path, report: Any) -> None: ...
