#!/usr/bin/env python3
"""Minimal NDJSON MCP stdio server for xylitol reload/provider-tools tests.

Stdlib only. Speaks newline-delimited JSON-RPC (MCP stdio transport).
Tools (env XYLITOL_MCP_FIXTURE_TOOLS, default `ping`):
  ping  — returns text "pong"
  echo  — returns text of args.message (default "")
"""

from __future__ import annotations

import json
import os
import sys


def _tools() -> list[str]:
    raw = os.environ.get("XYLITOL_MCP_FIXTURE_TOOLS", "ping")
    return [t.strip() for t in raw.split(",") if t.strip()]


TOOLS = _tools()


def _tool_defs() -> list[dict]:
    out = []
    for name in TOOLS:
        if name == "ping":
            out.append(
                {
                    "name": "ping",
                    "description": "fixture ping",
                    "inputSchema": {"type": "object", "properties": {}},
                }
            )
        elif name == "echo":
            out.append(
                {
                    "name": "echo",
                    "description": "fixture echo",
                    "inputSchema": {
                        "type": "object",
                        "properties": {"message": {"type": "string"}},
                    },
                }
            )
    return out


def _write(msg: dict) -> None:
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def _result(req_id, result: dict) -> None:
    _write({"jsonrpc": "2.0", "id": req_id, "result": result})


def _error(req_id, code: int, message: str) -> None:
    _write(
        {
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {"code": code, "message": message},
        }
    )


def _call_tool(name: str, arguments: dict | None) -> dict:
    args = arguments or {}
    if name == "ping":
        text = "pong"
    elif name == "echo":
        text = str(args.get("message", ""))
    else:
        raise KeyError(name)
    return {"content": [{"type": "text", "text": text}], "isError": False}


def main() -> None:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue

        method = msg.get("method")
        req_id = msg.get("id")

        # Notifications (no id): ignore after initialized.
        if req_id is None:
            continue

        if method == "initialize":
            params = msg.get("params") or {}
            version = params.get("protocolVersion") or "2025-03-26"
            _result(
                req_id,
                {
                    "protocolVersion": version,
                    "capabilities": {"tools": {"listChanged": False}},
                    "serverInfo": {"name": "xylitol-mcp-fixture", "version": "0.0.1"},
                },
            )
            continue

        if method == "tools/list":
            _result(req_id, {"tools": _tool_defs()})
            continue

        if method == "tools/call":
            params = msg.get("params") or {}
            name = params.get("name") or ""
            try:
                _result(req_id, _call_tool(name, params.get("arguments")))
            except KeyError:
                _error(req_id, -32602, f"unknown tool: {name}")
            continue

        if method == "ping":
            _result(req_id, {})
            continue

        _error(req_id, -32601, f"method not found: {method}")


if __name__ == "__main__":
    main()
