# c2936 实施任务

Seam：`Kernel::boot`（embed / CLI Print / Host）；crate 图扫描（`check_crate_graph.py`）；BDD 既有 Print/Host/MCP/hooks 场景改绑 kernel 入口。不新增脱离 `.feature` 的 harness。

- [x] 1. M0 spike 结论写入 `research/m0-rutis-spike.md`（edition 2024、instance 键、watch 门、瀑布预算、shutdown 超时、Chain+admit）
- [x] 2. plugin-sdk：`rt` 再导出、服务键、瀑布事件、`Registry<T>`、`SDK_API_VERSION`；sdk 精确锁 `rutis =0.13.0`
- [x] 3. 改 `check_crate_graph.py`：仅 sdk/kernel 可依赖 rutis；仅 kernel 可依赖 rutis-loader；登记 kernel 与新官方 crate
- [x] 4. 核心基础：`xylitol-official-plugin-config`、`xylitol-official-plugin-obs`、`xylitol-kernel` 骨架（boot/profile/config→rows/admit）
- [x] 5. 官方库 `PluginFactory` 入口：session、compaction、provider-openai/anthropic、tools、mcp、hooks
- [x] 6. runtime 核心插件：trust、session-host、agent-react、tool-table、driver；删除 `McpBootState` 手写编排与 `XyComposedDriver` 对 freeze 的包装职责
- [x] 7. serve 与 app-print 作为 profile 插件；`Kernel::boot` 打通 Print/Host/embed；删除 `src/wiring/**`、`src/cli/logging/**`、`tui_inprocess.rs`
- [x] 8. BDD 改接 kernel 测试入口（不留 wiring shim）；同步 workspace-assembly / layer-architecture / cli-entry / infra-mcp / agent-hooks / server-core 与根/`src/AGENTS.md`
- [x] 9. 契约测试：卸载零残留、瀑布顺序、实例回收、在飞调用显式失败一次、启动自检
- [x] 10. `just qa`；触及真终端路径时加 `just qa-e2e`
