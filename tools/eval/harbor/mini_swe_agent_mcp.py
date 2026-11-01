"""Mini-SWE Harbor agent with the missing `mcp` dependency.

Harbor's mini-swe-agent install pulls a recent LiteLLM that imports
`litellm.proxy._experimental.mcp_server`, which requires the `mcp` package.
Harbor's installer does not add it. This subclass installs the normal tool,
then adds `mcp` into the tool venv.
"""

from __future__ import annotations

from harbor.agents.installed.mini_swe_agent import MiniSweAgent
from harbor.environments.base import BaseEnvironment


class MiniSweAgentWithMcp(MiniSweAgent):
    @staticmethod
    def name() -> str:
        return "mini-swe-agent-mcp"

    async def install(self, environment: BaseEnvironment) -> None:
        await super().install(environment)
        await self.exec_as_agent(
            environment,
            command=(
                "set -euo pipefail; "
                "if [ -f \"$HOME/.local/bin/env\" ]; then . \"$HOME/.local/bin/env\"; fi; "
                "uv pip install --python \"$(uv tool dir)/mini-swe-agent/bin/python\" mcp"
            ),
        )
