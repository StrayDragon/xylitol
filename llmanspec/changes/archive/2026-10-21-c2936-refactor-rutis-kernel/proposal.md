---
depends_on: []
title: 以 rutis 为进程内核，组合根下沉为两层插件
branch: sdd/c2936-refactor-rutis-kernel
base_branch: main
base_sha: dabfc7f64f0b48e4a7c419a44e07cab306638f7f
---

# 以 rutis 为进程内核，组合根下沉为两层插件

## Why

根包 `src/` 仍承担 CLI 与 Host 装配（`wiring/` 约 15k 行）。Print / Host / embed 三条入口各自构造 baseline，已出现行为漂移；MCP 首轮门、工具冻结、hooks 总线、`/reload` 都是手写生命周期。需要把控制反转交给 rutis 0.13.0：fiber 生命周期、服务注册表、事件瀑布、loader rows。官方能力全部是 Plugin；核心插件由内核挂载且不可配置。第三方插件是产品方向、本次不交付，但解析器链与挂载准入位必须留下。

## What Changes

- 引入 `xylitol-kernel`：`Kernel::boot(Profile)` 统一 Print / Host / embed 装配；常驻 `Ctx`；核心插件直接 `plugin()`，可替换插件走 `rutis-loader` rows。
- `xylitol-plugin-sdk::rt` 再导出 rutis 类型；官方插件只依赖 sdk。新增服务键、瀑布事件、`Registry<T>`、SDK API 版本常量。
- 官方 crate 增加 `PluginFactory<Config>` 入口（session-store、compaction、provider-*、tools、mcp、hooks、permission/skills 若抽出）。
- 核心插件：config、obs、trust、registries、session-host、agent-react、tool-table、driver。ReAct / 单写者 / trust 仍是不变量执行者，只是挂成不可卸核心插件。
- 删除根包 `src/wiring/**` 与 `src/cli/logging/**`；root 只留 argv / profile。TUI designing 的 in-process 适配删除，改 attach-only（debug 预览路径不经 Host 装配）。
- 门禁 `check_crate_graph.py`：仅 sdk 与 kernel 可声明 `rutis`；仅 kernel 可声明 `rutis-loader`。
- 产品定调：插件生态是方向、未交付；现行扩展入口 = 官方插件 + 配置 + MCP / shell hook。
- 跨端信封仍是 fory-v3；不得换成 rutis-bridge。每 token `TextDelta` 仍走 `XyDriver::run` 流，不上 rutis 总线。

## Out of Scope

- 第三方插件市场、签名、dylib、多语言运行时嵌入（只留 Chain / admit 接缝）。
- 把产品跨端协议换成 rutis-bridge。
- LSP / DAP / Sub-Agent / gpui。
- rutis 0.14。

## Capabilities

- `workspace-assembly`：装配适配层与 rutis 依赖边界、未配置 MCP/hooks 零装配、ReAct 留在执行权威（改为核心插件、不可经 rows 替换）。
- `layer-architecture`：print 同进程、RPC 入口、单写者。
- `cli-entry`：argv → profile → `Kernel::boot`。
- `infra-mcp`：首轮门改为 watch+超时；reload 走 fiber update。
- `agent-hooks`：总线收编为瀑布事件。
- `server-core`：serve 作为 Host profile 的载体插件。
- `runtime-config`：配置由核心 config 插件加载。

## Impact

根包、sdk、runtime、serve、全部官方插件、BDD 对 `xylitol::wiring` 的引用、AGENTS.md 目标 crate 图。

## Further Notes

M0 spike 结论见 `research/m0-rutis-spike.md`。会话实例必须用 `TypeKey::instance`，全局 `provide` 第二份会 `ServiceExists`。

未收口（记入 `c2937-refactor-drop-mcpbootstate-and-xycomposeddriver-deref-move-occupancy`）：MCP 首轮门超时已归 tool-table，但 `McpBootState` 与 `XyComposedDriver` Deref 仍在；根包 `src/wiring` 只剩 occupancy 外观（kernel 不能依赖 `xylitol-app-tui`）。
