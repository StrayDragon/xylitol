"""Docker sandbox adapter."""

from __future__ import annotations

import os
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .errors import InfraError


def _as_text(value: str | bytes | None) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value or ""


@dataclass
class ExecResult:
    argv: list[str]
    stdout: str
    stderr: str
    exit_code: int | None
    timed_out: bool = False


class DockerSandbox:
    def __init__(self, binary: str = "docker", platform: str = "linux/amd64") -> None:
        self.binary = binary
        self.platform = platform

    def _run(
        self,
        args: list[str],
        timeout: float | None = None,
        env: dict[str, str] | None = None,
    ) -> ExecResult:
        run_env = os.environ.copy()
        if env:
            run_env.update(env)
        try:
            completed = subprocess.run(
                [self.binary, *args],
                capture_output=True,
                text=True,
                timeout=timeout,
                check=False,
                env=run_env,
            )
        except FileNotFoundError as exc:
            raise InfraError(f"docker binary not found: {self.binary}") from exc
        except subprocess.TimeoutExpired as exc:
            return ExecResult(
                argv=args,
                stdout=_as_text(exc.stdout),
                stderr=_as_text(exc.stderr),
                exit_code=None,
                timed_out=True,
            )
        return ExecResult(
            argv=args,
            stdout=completed.stdout,
            stderr=completed.stderr,
            exit_code=completed.returncode,
        )

    def ensure_image(self, image: str) -> None:
        inspect = self._run(["image", "inspect", image], timeout=60)
        if inspect.exit_code == 0:
            return
        last = None
        for attempt in range(3):
            pull = self._run(["pull", "--platform", self.platform, image], timeout=1800)
            if pull.exit_code == 0:
                return
            last = pull
            time.sleep(2 * (attempt + 1))
        raise InfraError(f"failed to pull image {image}: {last.stderr.strip() if last else 'unknown'}")

    def start(self, image: str, name: str, limits: dict[str, Any]) -> str:
        cpus = limits.get("cpus", 2)
        memory = limits.get("memory", "4g")
        args = [
            "run",
            "-d",
            "--rm",
            "--name",
            name,
            "--platform",
            self.platform,
            "--cpus",
            str(cpus),
            "--memory",
            str(memory),
            "-w",
            "/testbed",
            image,
            "sleep",
            "infinity",
        ]
        result = self._run(args, timeout=120)
        if result.exit_code != 0:
            raise InfraError(f"failed to start container {name}: {result.stderr.strip()}")
        return result.stdout.strip()

    def exec(
        self,
        container: str,
        argv: list[str],
        timeout: float,
        env: dict[str, str] | None = None,
    ) -> ExecResult:
        args = ["exec"]
        for key in (env or {}):
            args.extend(["-e", key])
        args.append(container)
        args.extend(argv)
        return self._run(args, timeout=timeout, env=env)

    def copy_in(self, container: str, src: Path, dst: str) -> None:
        result = self._run(["cp", str(src), f"{container}:{dst}"], timeout=120)
        if result.exit_code != 0:
            raise InfraError(f"failed to copy {src} to {container}:{dst}: {result.stderr.strip()}")

    def remove(self, container: str) -> None:
        self._run(["rm", "-f", container], timeout=60)
