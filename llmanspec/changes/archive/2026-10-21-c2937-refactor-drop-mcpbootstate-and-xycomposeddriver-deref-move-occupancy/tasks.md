# c2937 实施任务

Seam：既有 BDD（`infra-mcp` 首轮门 / 非阻塞启动 / loaded-resources；`agent-runtime` 工具表冻结；`agent-prompt` session_env 场景）+ kernel `contract.rs` 瀑布契约 + occupancy assembly fixture（迁 `tests/` 后仍跑）。不新增脱离 `.feature` 的 harness。前后对比在本 change 分支相对 merge-base 测量。

- [x] 1. plugin-sdk 增加 `session/prepare`（wait/timeout）。切会话 / resume / idle `/reload` emit 开火不 wait；generate 在拼出站前 wait。kernel 契约测：开火立即返回、wait 可超时放行。
- [x] 2. mcp 插件监听 `session/prepare`；删除 driver `McpBootState` 与 `begin/poll_mcp_bootstrap` 编排。loaded-resources / Assembling 仍由 Host 推信封。既有 MCP 门闸单测与 BDD 场景保持绿。
- [x] 3. 准备指纹有变时持久化 Env 行（对齐 `session_env`），插在下一 user 前；无变不追加；不改写既有 turn。单测覆盖有变/无变/resume。
- [x] 4. 拆掉 `XyComposedDriver`：进程内驱动即 `RuntimeDriver`；reload / todo 改挂载；embed 与 BDD 改调用点。
- [x] 5. occupancy 胶水下沉 `tests/`；CLI 直调 kernel occupancy；删除根包 `src/wiring`。crate 图门禁仍禁止 kernel → app-tui。
- [x] 6. `just qa`；触及 Assembling / resume 真终端路径时加 `just qa-e2e`。
