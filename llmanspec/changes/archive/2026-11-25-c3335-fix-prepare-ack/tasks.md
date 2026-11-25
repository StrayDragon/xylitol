# Tasks: c3335-fix-prepare-ack

## 1. 准备确认只留探针

- [x] 1.1 按 `design.md` 删掉驱动上的 `prepare_acked`。`snapshot_from_view` 改为读探针。`fire_session_prepare`、`settle_tool_gate`、`ensure_tool_table_frozen`、`poll_prepare_progress`、`arm_tool_freeze_gate`，以及 actor 的 new、fork、resume，都走 `set_prepare_acked`。

## 2. 两条读取看到同一份 false

seam：`xylitol-runtime` 里覆盖 new、fork、resume 和 MCP 门禁的现有单测。不新增 BDD。

- [x] 2.1 单测断言这三条路径之后，`mcp_blocks_agent` 的读取和 `snapshot_from_view` 的读取都是 false。
- [x] 2.2 既有 MCP 门禁单测保持通过。

## 3. 临时评判（deslop-please）

不进 `just qa`。基线是 `.tmp/deslop-please.allow`。测试路径诊断见 `.tmp/deslop-please-deferred-tests.md`，本 change 不改。全仓基线只由不带 crate 参数的 `deslop-please --update-allow .tmp/deslop-please.allow` 重写。只检查一个 crate 时，其它 crate 的条目不在这次输出里，它们仍留在基线中。

- [x] 3.1 在本 change 分支上跑 `deslop-please xylitol-runtime`。`XyInProcessDriver::prepare_acked` 从报告里消失。
- [x] 3.2 `deslop-please xylitol-runtime --allow .tmp/deslop-please.allow` 没有基线以外的新诊断。
