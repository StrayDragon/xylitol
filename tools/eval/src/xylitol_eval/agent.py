"""Headless xylitol print runner."""

from __future__ import annotations

import os
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .config import EvalConfig
from .errors import InfraError
from .paths import resolve_path
from .sandbox import DockerSandbox, ExecResult


@dataclass
class AgentRunResult:
    exit_code: int | None
    timed_out: bool
    stdout: str
    stderr: str
    wall_seconds: float


def _thinking_levels(desired: str) -> list[str]:
    ordered = ["off", "low"]
    if desired in {"high", "max"}:
        ordered.append("high")
    if desired == "max":
        ordered.append("max")
    if desired not in ordered:
        ordered.append(desired)
    return ordered


def render_xylitol_config(config: EvalConfig) -> str:
    alias = config.model.alias
    if config.model.provider == "fake":
        return f"""models:
  default_model: "{alias}"
  entries:
    "{alias}":
      provider: fake
      model: "fake-1"
      thinking: false
session:
  max_turns: {config.agent.max_turns}
"""
    levels = (
        list(config.model.thinking_levels)
        if config.model.thinking_levels
        else _thinking_levels(config.model.thinking_level)
    )
    if config.model.thinking_level not in levels:
        raise InfraError(
            f"thinking level {config.model.thinking_level} not in {levels}"
        )
    level_lines = "\n".join(
        f"        {level}: {'null' if level == 'off' else level}" for level in levels
    )
    return f"""models:
  default_model: "{alias}"
  entries:
    "{alias}":
      provider: openai
      model: "{alias}"
      base_url: "{{{{ env.{config.model.base_url_env} }}}}/v1"
      api: {config.model.api}
      api_key: "{{{{ env.{config.model.api_key_env} }}}}"
      thinking: true
      thinking_level_map:
{level_lines}
      context_window: {config.model.context_window}
      max_output_tokens: {config.model.max_output_tokens}
session:
  max_turns: {config.agent.max_turns}
"""


class XylitolPrintRunner:
    def run(
        self,
        instance: dict[str, Any],
        sandbox: DockerSandbox,
        container: str,
        config: EvalConfig,
    ) -> AgentRunResult:
        binary = resolve_path(config.agent.binary)
        if not binary.is_file():
            raise InfraError(f"agent binary not found: {binary}")
        if config.model.provider != "fake":
            for env_key in (config.model.base_url_env, config.model.api_key_env):
                if not os.environ.get(env_key):
                    raise InfraError(f"environment variable {env_key} is required")

        problem = instance.get("problem_statement", "")
        if not problem.strip():
            raise InfraError(f"instance {instance['instance_id']} has empty problem_statement")

        with tempfile.TemporaryDirectory(prefix="xylitol-eval-") as temp_dir:
            config_path = Path(temp_dir) / "config.yaml"
            config_path.write_text(render_xylitol_config(config), encoding="utf-8")
            sandbox.copy_in(container, binary, "/usr/local/bin/xylitol")
            sandbox.exec(container, ["chmod", "+x", "/usr/local/bin/xylitol"], timeout=30)
            sandbox.exec(container, ["mkdir", "-p", "/tmp/eval/home", "/tmp/eval/cfg"], timeout=30)
            sandbox.copy_in(container, config_path, "/tmp/eval/cfg/config.yaml")

            env = {
                "HOME": "/tmp/eval/home",
                "XYLITOL_CONFIG_DIR": "/tmp/eval/cfg",
                "XYLITOL_PROJECT_DIR": "/testbed",
            }
            if config.model.provider != "fake":
                env[config.model.base_url_env] = os.environ[config.model.base_url_env]
                env[config.model.api_key_env] = os.environ[config.model.api_key_env]
            argv = [
                "/usr/local/bin/xylitol",
                "print",
                "--no-trust",
                "--config",
                "/tmp/eval/cfg/config.yaml",
                "--model",
                config.model.alias,
                problem,
            ]
            result: ExecResult = sandbox.exec(
                container, argv, timeout=float(config.agent.wall_seconds), env=env
            )
            if result.timed_out:
                sandbox.exec(container, ["pkill", "-f", "/usr/local/bin/xylitol"], timeout=30)
            return AgentRunResult(
                exit_code=result.exit_code,
                timed_out=result.timed_out,
                stdout=result.stdout,
                stderr=result.stderr,
                wall_seconds=0.0,
            )
