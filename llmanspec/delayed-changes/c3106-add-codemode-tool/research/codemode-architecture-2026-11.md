# Codemode 架构调研（2026-11）

> 性质：change 附属调研，不是 live spec。
> 范围：pi codemode 的工具暴露模型；无 Node 的内嵌语言/沙箱候选；xylitol 的落点与缺口。
> 决策（2026-11-04）：本 change 选 Boa 0.22；WASM/多语言不在本 change。

## 结论

- pi codemode 的核心不是沙箱，而是“工具暴露 + 声明生成 + 动态发现”：exposure modes、namespace、TypeScript declarations、deferred、searchTools/describeTool、codemode.mode on/only。
- xylitol 已有工具注册表、ToolSet、selection/replaces、MCP/外进程插件工具；缺 exposure 元数据、catalog/declaration 生成、dispatch 端口、沙箱运行时、输出 schema/规范化。
- 语言候选：JS（QuickJS WASM / Boa / rquickjs）；Python（Eryx / Monty）；Rust DSL/DAG 不作为主语言。
- 建议先把“工具暴露/catalog”层做出来，再把 runtime 放到 trait 后面；不要先锁死 DSH 的 run_code executor 形态。

## pi codemode

来源：[pi codemode 文档](https://pi.dev/docs/latest/codemode)、[pi-codemode README](https://github.com/earendil-works/pi/blob/main/packages/codemode/README.md)、[pi v0.99 release notes](https://newreleases.io/project/github/earendil-works/pi/release/v0.99.0)。

- 工具输入是原始 JavaScript 源码，作为 async function body 在 QuickJS WASM sandbox 中运行；每个执行一个 worker；没有 Node API、文件系统、网络、timers。
- 脚本只能经 tools 和 models 接触外部；嵌套工具调用不进入模型上下文，只有脚本输出/返回值进入。
- Globals：tools.<name>(args)、ALL_TOOLS、searchTools(query)、describeTool(name)、describeNamespace(name)、text()、image()、console、return、exit()、store/load、models。
- 工具名转 JS identifier：非法字符变 _；同时支持 tools["原始名"]；MCP 工具 mcp__dev-radius__search → tools.mcp__dev_radius__search。
- codemode description 列 TypeScript declarations，按 namespace 分组；inline budget 3000 tokens；deferred 工具不列出，靠 searchTools/describeTool/ALL_TOOLS 发现。
- codemode.mode：on（默认）保留 direct 工具声明；only 隐藏 direct 工具，全部经脚本调用。
- 工具 exposure：direct、model-only、codemode、deferred、hidden；namespace、annotations、outputSchema、prepareLoadout、ctx.executeTool（v0.99 extension tool APIs）。
- 嵌套调用经 runToolCall()（共享 invocation pipeline）；失败调用 reject；mcp-codemode 示例把 MCP 工具和 codemode 组合。
- 限额：max_output_tokens、输出字符上限、timeout、memoryLimitBytes、worker admission；失败保留 partial output；未 await 的 promise 丢弃。

## 语言/沙箱候选

### JavaScript

| 方案 | 隔离 | 集成 | 依赖 | 备注 |
|---|---|---|---|---|
| QuickJS WASM | 高（WASM worker） | 中高（wasmtime/wasmer + quickjs-wasi ABI） | 无 Node | pi 的选择；无 Node API；需 WASM host 集成 |
| Boa | 中（同进程） | 低（纯 Rust） | 无 C | NativeFunction::from_async_fn 已合并；Promise/async 可用；覆盖率/性能取舍 |
| rquickjs | 中（同进程） | 低 | C（libquickjs） | async context；快；静态 musl 需 C 工具链 |
| deno_core/V8 | 高 | 高 | V8 | 体积大；不建议 |

### Python

| 方案 | 隔离 | 集成 | 依赖 | 备注 |
|---|---|---|---|---|
| Eryx | 高（wasmtime + CPython 3.14 WASI） | 低中（Rust crate） | wasmtime + CPython WASM | async callbacks；resource limits；sessions/snapshots/pool；VFS/network config；生产用于 Grafana；二进制大 |
| Monty | 中高（同进程 Rust） | 低（Rust crate） | 纯 Rust | Python 子集；async external functions；<1ms；MIT；~4.5MB；无 classes/match/context managers；无 event loop；asyncio 有限 |
| RustPython | 中 | 低 | 纯 Rust | 覆盖/async 不完整；不建议 |
| Pyodide | 高 | 高（JS glue） | WASM + JS | 不适合纯 Rust 嵌入 |

### Rust DSL / DAG

- Rhai/Starlark：LLM 知识低；同步 host 调用；不适合 async 工具编排。
- DAG DSL：安全、结构化；表达力有限；workflow 插件已有 DAG；可作为受限模式，不作为主语言。

## xylitol 映射

已有：

- HostRegistries.tools / ToolSet / resolve_tool_selection / replaces / freeze（crates/xylitol-runtime/src/agent/tools）。
- MCP 工具 mcp__{server}__{tool}（plugins/xylitol-official-plugin-mcp）。
- 外进程插件工具 {plugin}__{tool}（crates/xylitol-kernel/src/external_plugin.rs）。
- prepare_call / run_one 的 hooks、中间件、权限、事件（crates/xylitol-runtime/src/agent/runtime/tool_exec.rs）。
- permission_router 的 read/write/network 能力（crates/xylitol-runtime/src/agent/runtime/permission_router.rs）。

缺：

- XyTool 没有 exposure/namespace/output_schema/codemode identifier 元数据。
- 没有 tool catalog/declaration 生成服务。
- 没有 dispatch 端口让工具调用其他工具。
- 没有程序运行时/沙箱。
- 没有输出 schema/规范化（bash/MCP/read image 的 structured result）。
- 没有嵌套事件 parent/root id（lifecycle.rs 的 ToolExecutionStart/Update/End 只有 id）。

## 选项矩阵

| 方案 | 隔离 | 集成成本 | 二进制 | LLM 熟悉度 | 适用 |
|---|---|---|---|---|---|
| QuickJS WASM | 高 | 中高 | 中 | 高 | 想要 pi 同级隔离、无 Node |
| Boa | 中 | 低 | 中 | 高 | 想最快嵌入、可接受同进程 |
| rquickjs | 中 | 低 | 中 | 高 | 想快、可接受 C |
| Eryx | 高 | 低中 | 大 | 高 | 想要真 Python/stdlib |
| Monty | 中高 | 低 | 小 | 高 | 想要轻量 Python 子集 |
| Rust DSL | 高 | 低 | 小 | 低 | 受限编排 |
| DAG DSL | 高 | 低 | 小 | 中 | 受限编排/工作流 |

## 建议

1. 先做工具暴露/catalog：exposure、namespace、declaration、deferred、search/describe；这是“所有工具都能被理解”的关键。
2. 再做 dispatch 端口：嵌套调用复用 prepare_call/run_one；能力限制在 catalog + dispatch 两侧都生效。
3. runtime 后置为 trait；MVP 选一个语言（JS 或 Python），不要同时做多语言。
4. 不建议 Rust DSL/DAG 作为主语言；DAG 可作为单独的受限模式。

## 待确认

- 语言：JS（QuickJS WASM / Boa / rquickjs）vs Python（Eryx / Monty）。
- 隔离：WASM（crash-isolated）vs 同进程（简单，非 crash-isolated）。
- 工具暴露：direct/model-only/codemode/deferred/hidden 的默认分配；MCP 默认 deferred？
- codemode.mode：on/only 是否需要；首切片是否只做 tool。
- 输出：是否引入 output_schema；MCP structuredContent 如何规范化。

## 来源

- [pi codemode 文档](https://pi.dev/docs/latest/codemode)
- [pi-codemode README](https://github.com/earendil-works/pi/blob/main/packages/codemode/README.md)
- [pi v0.99 release notes](https://newreleases.io/project/github/earendil-works/pi/release/v0.99.0)
- [Pydantic Monty](https://pydantic.dev/articles/pydantic-monty.md) / [Monty repo](https://github.com/pydantic/monty)
- [Eryx repo](https://github.com/eryx-org/eryx) / [Eryx docs](https://docs.rs/eryx/latest/eryx/)
- [Boa async native function PR](https://github.com/boa-dev/boa/pull/2542)
- [rquickjs async context](https://docs.rs/rquickjs-core/latest/rquickjs_core/context/async.rs.html)
- [starlark-rust](https://github.com/facebook/starlark-rust)
- [juglans DAG DSL](https://github.com/juglans-ai/juglans)
