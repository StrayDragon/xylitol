# Tasks

## 1. SDK 替换声明 + 运行时工具选择解析

- [x] 1.1 `xylitol-plugin-sdk`：`XyTool` 增加默认方法 `replaces() -> &'static [&'static str]`（默认 `&[]`），不破坏既有实现。
- [x] 1.2 `xylitol-runtime`：新增 `agent/tools/selection.rs`，实现 `resolve_tool_selection(tools, entries) -> (ToolSet, Vec<String>)`；按顺序应用 `+` / `-` / 裸名，按名去重，解析 `replaces` 冲突并生成 warning。
- [x] 1.3 单测：顺序覆盖、unknown 名称、`-ffgrep` 回落、显式 `+grep` 冲突 warning、双替换环保护。

## 2. 配置 `tools` 列表

- [x] 2.1 `AppConfig` 增加 `tools: Vec<String>`（`#[serde(default)]`）；`validate_tools_selection` 拒绝空串 / 仅前缀 / 空白词元。
- [x] 2.2 `loader` 调用校验；`configs/config.schema.json` 由生成器更新。
- [x] 2.3 单测：合法列表、空词元失败、未知工具名不在加载期失败。

## 3. fff 工具源插件

- [x] 3.1 新建 workspace crate `plugins/xylitol-official-plugin-tools-fff`（依赖 `fff-search`，启用 `definitions` feature）。
- [x] 3.2 工作区索引缓存：按 `ctx.workspace` 惰性建 `SharedFilePicker`，`watch=true`，`wait_for_scan(10s)`；搜索走 `spawn_blocking`。
- [x] 3.3 `FffGrepTool`：`query` / `path` / `glob` / `mode` / `definitionsOnly` / `context` / `limit` / `timeout`；输出 `path:line: content`，声明 `replaces = ["grep"]`。
- [x] 3.4 `FffFindTool`：`query` / `path` / `mode` / `limit` / `timeout`；输出相对路径，声明 `replaces = ["find"]`。
- [x] 3.5 插件 `apply` 向 `HostRegistries.tools` 注册两工具；单测用临时目录（`watch=false`）验证内容检索、路径检索、参数校验、超时语义。

## 4. 内核装配与 feature

- [x] 4.1 `xylitol-kernel` 增加可选依赖 + `fff` feature（默认开）；`official_builtins` 注册 `tools-fff`；`base_rows` 增加对应 row。
- [x] 4.2 `ToolFeatures` 增加 `tool_selection`（改 `Clone`）；`DriverContext` 增加 `static_tools`。
- [x] 4.3 `compose_tools` 以注册表静态工具为基座（空则回落 `core_tools()`），追加 session 工具后调用 `resolve_tool_selection` 并记录 warning。
- [x] 4.4 `ProductOccupancyFactory` 从 `HostRegistries` 取静态工具快照（过滤 `mcp__`）并传 `tools` 配置。
- [x] 4.5 `boot_with` 在 `boot_ctx` 后把注册表静态工具注入 bootstrap 路径；独立 `bootstrap()` 保持 `core_tools()` 回落。
- [x] 4.6 单测：插件工具可见、`-ffgrep` 回落内置、feature 关闭时仅内置。

## 5. BDD 步骤与绑定

- [x] 5.1 `agent-tools`：新增工具选择 / 替换 / 冲突 warning / 插件缺失回落场景的 step 与 binding。
- [x] 5.2 `infra-fff-search`：新增 ffgrep 内容检索、fffind 路径检索、索引复用 / 部分索引场景的 step 与 binding。
- [x] 5.3 `runtime-config`：新增 `tools` 列表解析、非法词元、未知工具名场景的 step 与 binding。
- [x] 5.4 `cargo test --all-features --test bdd` 全绿。

## 6. 文档与样例

- [x] 6.1 `configs/example.yaml` 增加 `tools` 列表注释样例（含 `-grep +ffgrep`）。
- [x] 6.2 README 增加「检索工具选择」与 fff 插件说明，注明 `fff-mcp` 旁路与关闭 feature / 回退写法。
- [x] 6.3 `just gen-config-schema` / `just gen-config-example` 后确认无漂移；`just qa` 全绿。
