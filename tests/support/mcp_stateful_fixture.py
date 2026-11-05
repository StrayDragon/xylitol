# /// script
# requires-python = ">=3.11"
# dependencies = ["mcp==2.3.0"]
# ///
"""Stateful MCP fixture used by xylitol integration and BDD tests.

Run directly with: uv run tests/support/mcp_stateful_fixture.py
(PEP 723 inline metadata; no virtualenv setup is required.)
"""

from __future__ import annotations

import itertools
import sys

from mcp import types
from mcp.server import MCPServer

mcp = MCPServer("stateful-fixture")

_state = {"counter": 0}
_elicit_calls = itertools.count()


@mcp.tool()
def ping() -> str:
    """Return a fixed pong."""
    return "pong"


@mcp.tool()
def echo(text: str) -> str:
    """Echo the provided text."""
    return text


@mcp.tool()
def bump() -> int:
    """Increment the fixture counter and return the new value."""
    _state["counter"] += 1
    return _state["counter"]


@mcp.tool()
def current() -> int:
    """Return the current counter value without changing it."""
    return _state["counter"]


@mcp.tool()
def reset() -> int:
    """Reset the counter to zero and return it."""
    _state["counter"] = 0
    return _state["counter"]


@mcp.tool()
def elicit_name() -> str | types.InputRequiredResult:
    """Exercise the MRTR elicitation flow: ask for a name, then return it."""
    if next(_elicit_calls) == 0:
        return types.InputRequiredResult(
            inputRequests={
                "name": types.ElicitRequest(
                    params=types.ElicitRequestFormParams(
                        message="What is your name?",
                        requestedSchema={
                            "type": "object",
                            "properties": {"name": {"type": "string"}},
                            "required": ["name"],
                        },
                    )
                )
            },
            requestState="awaiting-name",
        )
    return "Ada"


@mcp.prompt()
def summarize(topic: str) -> str:
    """Prompt fixture: summarize a topic."""
    return f"Summarize the following topic: {topic}"


@mcp.resource("fixture://state")
def read_state() -> str:
    """Resource fixture: expose the counter state as JSON text."""
    return '{"counter": ' + str(_state["counter"]) + "}"


@mcp.resource("fixture://greeting/{name}")
def read_greeting(name: str) -> str:
    """Resource template fixture."""
    return f"Hello, {name}!"


if __name__ == "__main__":
    if "--self-check" in sys.argv:
        print("stateful MCP fixture ready")
    else:
        mcp.run()
