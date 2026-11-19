'''Preinstalled Harbor agent variant of Pi used by eval environments.'''
from __future__ import annotations

from harbor.agents.installed.pi import Pi
from harbor.environments.base import BaseEnvironment


class PiPreinstalled(Pi):
    async def install(self, environment: BaseEnvironment) -> None:
        await self.exec_as_agent(environment, command="pi --version")
