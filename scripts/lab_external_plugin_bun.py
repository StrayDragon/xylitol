#!/usr/bin/env python3
"""Lab probe: verify the Bun demo external plugin over stdio JSON-RPC.

Usage:
    python3 scripts/lab_external_plugin_bun.py
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PLUGIN = REPO / "examples" / "plugins" / "bun-demo" / "plugin.ts"


def request(proc: subprocess.Popen[str], request_id: int, method: str, params: dict) -> dict:
    line = json.dumps(
        {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
    )
    assert proc.stdin is not None
    proc.stdin.write(line + "\n")
    proc.stdin.flush()
    assert proc.stdout is not None
    while True:
        raw = proc.stdout.readline()
        if not raw:
            raise RuntimeError("plugin exited before responding")
        response = json.loads(raw)
        if response.get("id") == request_id:
            return response


def main() -> int:
    bun = shutil.which("bun")
    if not bun:
        print("skip: bun is not on PATH", file=sys.stderr)
        return 2
    proc = subprocess.Popen(
        [bun, str(PLUGIN)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    try:
        init = request(
            proc,
            1,
            "initialize",
            {"protocol_version": 1, "plugin": "bun-demo"},
        )
        result = init["result"]
        assert result["protocol_version"] == 1, result
        assert result["name"] == "bun-demo", result
        assert "tools" in result["capabilities"], result
        names = {tool["name"] for tool in result["tools"]}
        assert names == {"echo", "add"}, names

        echo = request(
            proc,
            2,
            "tools/call",
            {"name": "echo", "arguments": {"text": "hello bun"}},
        )
        assert echo["result"]["content"] == "hello bun", echo

        add = request(
            proc,
            3,
            "tools/call",
            {"name": "add", "arguments": {"a": 2, "b": 40}},
        )
        assert add["result"]["content"] == "42", add
        print("ok: bun-demo initialize + echo + add")
        return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    sys.exit(main())
