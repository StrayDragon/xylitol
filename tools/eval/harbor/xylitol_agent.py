"""Harbor custom agent for xylitol.

Run with:

    PYTHONPATH=tools/eval/harbor harbor run \
      -t hello-world/hello-world \
      -a xylitol_agent:XylitolAgent \
      -m openai/group/glm-5.3-flash

The adapter installs the Linux binary into the task container, writes a
temporary xylitol config, and runs `xylitol print` headlessly. Harbor's task
verifier grades the resulting working tree; no patch/preds collection is
needed.
"""

from __future__ import annotations

import os
import shlex
import tempfile
from pathlib import Path

import certifi

from harbor.agents.agent_auth_resolution import (
    ModelConnectionSpec,
    ProviderAccess,
)
from harbor.agents.capabilities import AgentCapabilities
from harbor.agents.installed.base import BaseInstalledAgent, with_prompt_template
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext
from harbor.models.model_api_format import ModelApiFormat


_REMOTE_DIR = "/tmp/xylitol-eval"
_CONFIG_DIR = f"{_REMOTE_DIR}/cfg"
_CONFIG_PATH = f"{_CONFIG_DIR}/config.yaml"
_SECRET_PATH = f"{_CONFIG_DIR}/secret.env"


class XylitolAgent(BaseInstalledAgent):
    capabilities = AgentCapabilities(base_url=True)
    MODEL_CONNECTION = ModelConnectionSpec(
        default_provider="openai",
        agent_provider_access=ProviderAccess(
            api_key_candidate_names=["MAGPIE_GATEWAY_KEY"],
            base_url_candidate_names=["MAGPIE_GATEWAY_URL"],
        ),
        supported_api_formats=[ModelApiFormat.OPENAI_RESPONSES],
        default_api_format_by_provider={"default": ModelApiFormat.OPENAI_RESPONSES},
        api_key_destination_names={"default": "MAGPIE_GATEWAY_KEY"},
        base_url_destination_names={"default": "MAGPIE_GATEWAY_URL"},
    )

    def __init__(
        self,
        *args,
        binary: str | None = None,
        max_steps_per_turn: int = 80,
        wall_seconds: int = 3600,
        **kwargs,
    ) -> None:
        super().__init__(*args, **kwargs)
        self.binary = binary or os.environ.get("XYLITOL_EVAL_BINARY", "")
        self.max_steps_per_turn = int(max_steps_per_turn)
        self.wall_seconds = int(wall_seconds)

    @staticmethod
    def name() -> str:
        return "xylitol"

    def version(self) -> str | None:
        return "0.0.0-pre"

    async def install(self, environment: BaseEnvironment) -> None:
        if not self.binary:
            raise ValueError("xylitol binary path is required (--ak binary=...)")
        binary = Path(self.binary)
        if not binary.is_file():
            raise ValueError(f"xylitol binary not found: {binary}")
        await self.exec_as_root(
            environment,
            command=f"mkdir -p {shlex.quote(_REMOTE_DIR)} /usr/local/bin",
        )
        await environment.upload_file(binary, "/usr/local/bin/xylitol")
        await self.exec_as_root(
            environment,
            command="chmod 755 /usr/local/bin/xylitol",
        )

    def _model_id(self) -> str:
        _, model_id = self._parse_model_name(self.model_name)
        return model_id

    def _render_config(self, base_url: str) -> str:
        model = self._model_id()
        return f"""models:
  default_model: "{model}"
  entries:
    "{model}":
      provider: openai
      model: "{model}"
      base_url: "{base_url}"
      api: openai-responses
      api_key: "{{{{ secret.MAGPIE_GATEWAY_KEY }}}}"
      thinking: true
      thinking_level_map:
        low: low
        high: high
      context_window: 1000000
      max_output_tokens: 128000
session:
  max_steps_per_turn: {self.max_steps_per_turn}
"""

    async def configure(
        self,
        environment: BaseEnvironment,
        base_url: str | None = None,
        api_key: str | None = None,
    ) -> None:
        if base_url is None:
            base_url = self.model_connection.env.get(
                "MAGPIE_GATEWAY_URL"
            ) or self._get_env("MAGPIE_GATEWAY_URL")
        if api_key is None:
            api_key = self.model_connection.env.get(
                "MAGPIE_GATEWAY_KEY"
            ) or self._get_env("MAGPIE_GATEWAY_KEY")
        if not base_url:
            raise ValueError("MAGPIE_GATEWAY_URL is required")
        if not api_key:
            raise ValueError("MAGPIE_GATEWAY_KEY is required")
        if "\n" in api_key or "\r" in api_key:
            raise ValueError("MAGPIE_GATEWAY_KEY must not contain newlines")
        base_url = base_url.rstrip("/")
        if not base_url.endswith("/v1"):
            base_url = f"{base_url}/v1"
        await self.exec_as_root(
            environment,
            command=(
                f"mkdir -p {shlex.quote(_REMOTE_DIR)}/cfg {shlex.quote(_REMOTE_DIR)}/home "
                f"&& chmod 777 {shlex.quote(_REMOTE_DIR)} "
                f"{shlex.quote(_REMOTE_DIR)}/cfg {shlex.quote(_REMOTE_DIR)}/home"
            ),
        )
        with tempfile.TemporaryDirectory(prefix="xylitol-harbor-") as tmp:
            config_path = Path(tmp) / "config.yaml"
            config_path.write_text(self._render_config(base_url), encoding="utf-8")
            secret_path = Path(tmp) / "secret.env"
            secret_path.write_text(
                f"MAGPIE_GATEWAY_KEY={api_key}\n", encoding="utf-8"
            )
            await environment.upload_file(config_path, _CONFIG_PATH)
            await environment.upload_file(secret_path, _SECRET_PATH)
            await environment.upload_file(certifi.where(), f"{_REMOTE_DIR}/ca.pem")
        await self.exec_as_root(
            environment,
            command=(
                f"chmod 600 {shlex.quote(_CONFIG_PATH)} "
                f"{shlex.quote(_SECRET_PATH)} "
                f"&& chmod 644 {shlex.quote(_REMOTE_DIR)}/ca.pem"
            ),
        )

    @with_prompt_template
    async def run(
        self,
        instruction: str,
        environment: BaseEnvironment,
        context: AgentContext,
    ) -> None:
        connection_env = self.model_connection.env
        url = connection_env.get("MAGPIE_GATEWAY_URL") or self._get_env(
            "MAGPIE_GATEWAY_URL"
        )
        key = connection_env.get("MAGPIE_GATEWAY_KEY") or self._get_env(
            "MAGPIE_GATEWAY_KEY"
        )
        if not url or not key:
            raise ValueError(
                "MAGPIE_GATEWAY_URL and MAGPIE_GATEWAY_KEY are required in agent env"
            )

        await self.configure(environment, base_url=url, api_key=key)

        logs_dir = self.environment_logs_dir
        output_path = logs_dir / "xylitol.txt"
        env = {
            "HOME": f"{_REMOTE_DIR}/home",
            "XYLITOL_CONFIG_DIR": f"{_REMOTE_DIR}/cfg",
            "XYLITOL_PROJECT_DIR": "/testbed",
            "SSL_CERT_FILE": f"{_REMOTE_DIR}/ca.pem",
            "GIT_AUTHOR_NAME": "eval",
            "GIT_AUTHOR_EMAIL": "eval@example.com",
            "GIT_COMMITTER_NAME": "eval",
            "GIT_COMMITTER_EMAIL": "eval@example.com",
        }
        command = (
            f"mkdir -p {shlex.quote(str(logs_dir))} && "
            f"timeout {self.wall_seconds} /usr/local/bin/xylitol print "
            "--no-trust "
            f"--config {shlex.quote(_CONFIG_PATH)} "
            f"--model {shlex.quote(self._model_id())} "
            f"{shlex.quote(instruction)} "
            f"2>&1 | tee {shlex.quote(str(output_path))} "
            "|| true"
        )
        await self.exec_as_agent(environment, command=command, env=env)
