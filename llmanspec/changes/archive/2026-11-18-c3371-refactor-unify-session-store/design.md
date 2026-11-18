# Design

## 决策 1：校验语义放 port 方法，不放 kernel 旁路

新增 `XySessionStore::load_validated`（provided，默认 = `load_entries`），`LogSessionStore` 覆写并执行 `assert_session_cwd_exists`。

- 否决 kernel 侧 downcast 到具体类型：违反「依赖用端口」纪律（src/AGENTS.md）。
- 内存 store / 测试 MemStore 走默认实现，行为零变化；磁盘 store 才有 cwd 语义，与 r1094「从磁盘加载会话时」措辞对齐。
- 加法演进：不改既有 24 方法签名，无迁移路径问题。

## 决策 2：两个接线点，共用同一实现

恢复会话有两条生产路径（实测）：

1. `switch_session`（runtime in_process/session.rs:36）：TUI /session-resume 产品命令与 RPC SwitchSession/绑定都经 `execute_session_command → exec_switch_session → switch_session`。接线点：`exists` 检查通过后、`build_session_context` 前调用 `store.load_validated(id, &self.agent.cwd())`（agent.cwd() 已存在，session.rs:237）。switch 低频，validated load 与 build_session_context 的双读可接受。
2. `restore_session`（kernel bootstrap.rs:429）：CLI/TUI `--session` 经 `BootstrappedAgent::into_driver → agent.bind_session`（bootstrap.rs:179），不经 switch_session。接线点：restore_session 内改用 `LogSessionStore`，对已存在会话调用 `load_validated`，失败转 `BootstrapError`（文案即校验错误原文，满足 r1094「面向用户的 CWD 错误信息 即该可操作校验错误文本」）。

不在 runtime `bind_session` 原语内校验：该原语同时服务新会话创建与 occupancy 槽绑定，校验语义过宽。

## 决策 3：测试边界（seam，全部复用既有 harness）

- store 级：r1094 既有 4 条 GWT 场景（BDD step 经 dyn port 调 `load_validated`，fixture 从 SessionManager 换 LogSessionStore）——场景文本不变。
- 组件级单测（新增，用户明确要求）：`assert_session_cwd_exists` 3 条（可用直过 / fallback 救回 / 双缺失报错含双路径）+ runtime `switch_session` 2 条（lost cwd 拒绝且消息含双路径 / fallback 救回成功）。
- 分层不变量：BDD 步骤「恢复前完成同一 CWD 校验」走 `scripts/check_layer_invariants.py` 通用闸（已验证脚本 0 处引用 SessionManager/cwd/load_validated，不改脚本）；「infra SessionManager 实现协议端口」为孤儿步骤（无场景引用），仅改步骤文本防 stale 黑话。
- 回归：kernel bootstrap_tests、driver/in_process/tests、occupancy assembly fixture 既有测试群随迁移全绿。

## 删除验证（S2 结论复用）

删 `SessionManager`：16 个死方法与转发样板直接消失；cwd 校验在 1 处迁移点重出（迁入 LogSessionStore）；`exists_sync` 在 bootstrap 1 个调用点改用 LogSessionStore 同名方法（已存在，log/mod.rs:243，升 pub 即可）。

## 不做

- 不动 `build_session_context` / `migrate_legacy_before_switch` / profile 处理逻辑。
- 不为 `SessionManager` 留兼容别名或 re-export（本仓规则）。
- 不改 JSONL 格式与 SESSION_VERSION。
