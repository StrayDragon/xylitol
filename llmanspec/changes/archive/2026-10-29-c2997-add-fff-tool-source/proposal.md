---
depends_on: []
title: 检索工具插件化：fff 工具源与工具选择列表
needs_specs_change: true
branch: sdd/c2997-add-fff-tool-source
base_branch: main
base_sha: 67105fb0a2d79412917f155f7f367335cdb73d69
---

# 检索工具插件化：fff 工具源与工具选择列表

## Why

- #5 要求支持 [fff](https://github.com/dmtrKovalenko/fff) 作为 grep / find 类检索工具的替代，并覆盖其 MCP server 形态。
- 现行 `grep` / `find` 由 `xylitol-official-plugin-tools` 的 `core_tools()` 硬编码装配；`HostRegistries.tools` 只被 MCP 动态写入，静态工具没有真正的插件接缝。内核 `compose_tools` 直接调用 `core_tools()`，第三方/官方工具插件无法贡献静态工具。
- 内置 provider preset（c2995）回滚后，扩展方向转向「插件 + 内核基础接缝」（#35 / #29）。fff 是一个合适的首个工具源插件样板：它验证「插件注册静态工具 + 用户用工具列表选择 + 内核只做通用解析」。
- `fff-search` 0.11.0 已在本机工具链（nightly-2026-10-09）下 `cargo check` 通过；MIT 许可；进程内索引 + watcher，适合长期会话内多次检索。

## What Changes

- 新增官方插件 `xylitol-official-plugin-tools-fff`：
  - 提供 `ffgrep`（内容检索）与 `fffind`（路径检索）两个 `XyTool`。
  - 每个工作区惰性建立 fff 索引；索引在进程内复用，watcher 随插件生命周期存在。
  - 不持久化 frecency / query DB（MVP 不引入 LMDB 文件）；搜索在线程池中执行，避免阻塞 async runtime。
  - 默认装配；内核 feature `fff` 默认开启，可关闭。
- `XyTool` 端口新增默认方法 `replaces() -> &'static [&'static str]`：工具可声明自己替代哪些同类工具（`ffgrep` → `grep`，`fffind` → `find`）。默认空切片，不破坏既有实现。
- 新增通用配置 `tools: Vec<String>`（工具选择列表）：
  - `+name` 启用、`-name` 禁用、裸 `name` 等同启用；按顺序应用，后者覆盖前者。
  - 未知名在装配期 warning 并忽略，不使配置加载失败。
  - 替换关系解析：若替换工具与被替换工具同时启用，替换工具胜出、只保留一个；被替换工具由显式 `+` 启用时 warning。
  - 默认静态工具集来自插件注册表；插件缺失时回落内置 `core_tools()`。
- 内核装配改造：
  - `DriverContext` 携带从 `HostRegistries.tools` 取得的静态工具快照（过滤 `mcp__` 前缀）；`compose_tools` 以该快照为基座，插件缺失时回落 `core_tools()`。
  - `ToolFeatures` 携带 `tools` 配置；`compose_tools` 统一调用工具选择解析并输出 warning。
  - `boot_with` 在 `boot_ctx` 之后把注册表静态工具注入 bootstrap 路径，使 Print / 初始 driver 与 TUI / Host 新 driver 行为一致。
- 规格：
  - `agent-tools` 增加工具选择列表、替换语义、插件静态工具源规则；调整默认闭集规则以说明「闭集不变，可见集可替换」。
  - 新增 `infra-fff-search` capability，约束 fff 工具面、索引生命周期与回落行为。
  - `runtime-config` 增加 `tools` 列表解析与校验规则。
- 文档 / 样例：
  - README 增加「检索工具选择」与 fff 插件说明，并注明 `fff-mcp` 作为 MCP 旁路。
  - `configs/example.yaml` 增加 `tools` 列表注释样例。
  - `configs/config.schema.json` 由 schema 生成器更新。

## Capabilities

- `agent-tools`（工具选择、替换、静态工具源）
- `runtime-config`（`tools` 列表解析）
- `infra-fff-search`（新增：fff 工具面与索引生命周期）

## Impact

- **默认行为变化**：启用 fff feature 时，模型默认看到 `ffgrep` / `fffind`，内置 `grep` / `find` 被替换隐藏；关闭 feature 或显式 `-ffgrep -fffind` 时回落内置。
- **模型工具名变化**：从 `grep` / `find` 变为 `ffgrep` / `fffind`；参数 schema 不同（`query` 而非 `pattern`，支持 `definitionsOnly` / fff 模式）。
- **构建成本**：新增 `fff-search` 依赖（含 LMDB sys 编译依赖，即使 MVP 不打开 frecency DB）。
- **可回退**：配置 `-ffgrep -fffind` 或关闭 feature `fff` 即回到内置检索；MCP `fff-mcp` 仍可作为旁路，不依赖本插件。
- **不做**：不在内核写 fff 特判；不引入 fff 的 MCP preset / 安装器；不做 frecency 持久化、multi_grep、cursor 分页（后续按需）。

## Non-Goals

- 不替换 `read` / `write` / `edit` / `bash` / `ls`。
- 不改变 ReAct 循环、工具批调度或权限裁决。
- 不把 fff 索引写入工作区（不产生仓库内文件）。
- 不把未交付的第三方插件市场写成现行能力（见 #29）。
