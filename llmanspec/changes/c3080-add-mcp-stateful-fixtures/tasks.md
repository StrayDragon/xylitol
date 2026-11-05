# Tasks — Stateful MCP fixture

- [x] 1. 新 fixture：`tests/support/mcp_stateful_fixture.py`（PEP 723、mcp==2.3.0、tools/prompts/resources/templates/stateful/elicitation、`--self-check`）；现有 fixture 不动。
- [x] 2. CI：安装 uv 与 Python 3.11，运行 fixture self-check。
- [x] 3. Specs 与验证：test-* 条款；`uv run` 实测。
- [x] 4. 门禁：`just fmt` / `just lint` / `llman-sdd validate c3080-add-mcp-stateful-fixtures --strict` / `just qa`。
