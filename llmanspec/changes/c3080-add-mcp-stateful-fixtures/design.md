# Design — Stateful MCP fixture

## 1. 新 fixture

- 落点 `tests/support/mcp_stateful_fixture.py`，不改现有 `mcp_fixture_server.py`。
- Python 官方 SDK `mcp==2.3.0`，使用 2.x 的 `MCPServer`（FastMCP 的现行形态）。
- PEP 723 inline metadata；`uv run tests/support/mcp_stateful_fixture.py` 直接运行，不初始化 venv。
- 覆盖：tools（ping / echo / stateful counter / reset / MRTR elicitation）、prompts（summarize）、resources（`fixture://state`）、resource templates（`fixture://greeting/{name}`）。
- `--self-check` 只验证导入与注册，不进入 stdio 循环。

## 2. CI

- CI 安装 uv（`astral-sh/setup-uv`）与 Python 3.11。
- 在 `just qa` 前跑 fixture self-check；fixture 失败即 CI 红。
- 现有 minimal fixture 的测试不变。

## 3. 断言节奏

- fixture 先行；tools / prompts / resources / elicitation 的主线断言随 c3070 / c3090 落地。
- 本 change 只保证 fixture 可启动、工具 / prompt / resource 注册完整、CI 可验证。

## 4. 风险

- mcp 2.3.0 的 `MCPServer` API 与旧 FastMCP 不同；以 `uv run` 实测为准。
- CI 网络下载 uv / Python / wheel；缓存由 setup-uv 负责。
