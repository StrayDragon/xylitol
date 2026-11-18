---
depends_on: []
title: 会话存储单路径收敛并接线恢复前 CWD 校验
needs_specs_change: true
branch: sdd/c3371-refactor-unify-session-store
base_branch: main
base_sha: 086e044208ded652c9bacd3dd9b089535f029d7d
---

## Why

全仓存在两个 `XySessionStore` 实现：`SessionManager`（473 行）与 `LogSessionStore`。`SessionManager` 的 19 个 trait 方法全部 1:1 转发 `inner`，42 个固有方法中 16 个外部调用为 0，`flush_pending_to_disk` 为空桩；同一插件经 plugin.rs 直供 `LogSessionStore`、经 kernel 走 `SessionManager`，同一 port 两条构造路径。架构审查 S2（2026-11-17）判定其薄透传，删除验证通过。

同时 spec r1094 末句「CLI 与 RPC 模式 须在恢复会话前完成同一 CWD 校验」当前无生产实现：`load_validated` 仅被 3 处 BDD step 调用；CLI `--session` 经 `restore_session` 只查 `exists_sync`，TUI /session-resume 与 RPC 恢复经 `switch_session` 均不校验。本次收敛顺带补上生产接线。

## What Changes

- sdk `XySessionStore` port 新增 provided 方法 `load_validated(id, fallback_cwd)`，默认实现等于普通 `load_entries`（内存实现零感知）。
- session 插件：cwd 校验逻辑（`assert_session_cwd_exists`）迁入 `LogSessionStore` 并覆写 port 方法；`exists_sync` 升 pub；补 3 条目录校验单测。
- runtime `switch_session` 在恢复前调用 `load_validated`（fallback 取 `agent.cwd()`）——覆盖 TUI /session-resume 与 RPC SwitchSession。
- kernel `restore_session`（CLI/TUI `--session` 门）改用 `LogSessionStore` 并在恢复前校验——覆盖 into_driver 的 `bind_session` 路径。
- 全部 `SessionManager` 调用点（kernel 组合/occupancy/embed/约 40 处测试）改用 `LogSessionStore`；删除 `manager/` 模块与导出，不留别名。
- spec 措辞同步：`agent-session-store.feature` 8 处与 `agent-runtime.feature` 1 处主语「SessionManager」改为「会话存储」，语义不变；孤儿 BDD 步骤文本同步改名。

## Capabilities

- `specs/agent-session-store`：r1094 等规则主语措辞（行为语义不变）。
- `specs/agent-runtime`：分层规则中实现名措辞。

## Impact

- 公开面：`xylitol_official_plugin_session::SessionManager` 删除（embed 唯一使用点同步改 `LogSessionStore::in_memory`）；`LogSessionStore` 成为插件唯一公开 store 类型。
- 行为变化：恢复已存会话时，CLI 与 RPC/TUI 恢复路径新增 cwd 校验（spec r1094 既有要求落地）；错误文案即既有可操作校验错误文本。
- 不变：`XySessionStore` 既有 24 方法语义、JSONL 磁盘格式、in-memory 行为。
