# Macro Registration — Design Evaluation

> 评估用 Rust 宏系统替换当前运行时注册模式的可行性。
> 产出时间：2026-06-23
> 状态：设计评审，未实施

## 背景

当前 xylitol 使用**运行时注册**模式：
- `ToolRegistry::register()` — 运行时 push 到 `Vec<Arc<dyn XyTool>>`
- `ModelConfigExt::build()` — 运行时 match ModelKind → 构造 provider
- `get_all_commands()` — 运行时拼接 Vec<SlashCommandInfo>
- `EventBus` — 运行时订阅/分发

这些模式灵活（支持动态加载），但牺牲了编译期检查。

## 候选分析

### 1️⃣ `#[tool]` 过程宏 — 编译期工具注册

**方案**：
```rust
#[tool(name = "read_file", description = "Read a file")]
async fn read_file(ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> { ... }
```
生成静态 `TOOL_REGISTRY` 表（`&[&dyn XyTool]`），编译期确定。

**优点**：
- 零运行时注册开销
- 工具实现与声明同处一处（当前分开在 `tools/` 模块 + `ToolRegistry::register()`）
- 编译器验证 name/description 非空

**缺点**：
- 需要引入 proc-macro crate 或 `inventory` crate
- 无法动态加载工具（扩展、MCP 工具）
- proc-macro 增加编译时间

**ROI**: 中等。工具数量固定（7 个内置），动态加载场景（MCP）存在但少用。
**建议**：暂不实施。如果工具数量 > 15 或启动注册成为瓶颈时重新评估。

### 2️⃣ `#[command]` 声明宏 — 静态命令表

**方案**：
```rust
#[command]
const BUILTIN_COMMANDS: &[(&str, &str)] = &[
    ("model", "Select model"),
    ...
];
```
当前已用 `const BUILTIN_COMMANDS` 实现，不需要宏。
`SlashCommandInfo` 的结构可以改为 `const` 优先。

**优点**：零开销，当前就接近静态
**缺点**：扩展命令（skill/template）仍需运行时
**ROI**: 低。当前实现已足够高效。
**建议**：不实施。保持 `const BUILTIN_COMMANDS` + 运行时 extension_commands。

### 3️⃣ `#[provider]` 过程宏 — ModelKind 匹配

**方案**：
```rust
#[provider(kind = "openai")]
struct OpenAIProvider { ... }

#[provider(kind = "anthropic")]
struct AnthropicProvider { ... }
```
生成 `fn build_provider(kind: ModelKind, config: ModelConfig) -> Result<Arc<dyn XyModel>>` 的 match。

**优点**：
- 新增 provider 只需加结构体 + `#[provider]`，无需编辑 match 分支
- 编译期验证 ModelKind 有对应 provider

**缺点**：
- provider 构造涉及 async API key 解析、client 初始化，不是纯工厂
- 需要 proc-macro crate
- OpenAi 和 Anthropic 的构造参数不同（base_url、headers 等）

**ROI**: 中-高。Provider 数量会增长（DeepSeek、Gemini、Ollama 等）。
**建议**：可考虑在 provider 数量 > 5 时实施。当前 2 个内置 + 1 个 fake 还不需要。

### 4️⃣ 其他候选

| 场景 | 当前机制 | 宏候选 | ROI | 建议 |
|---|---|---|---|---|
| Hook 注册 | EventBus.on() | `#[hook]` 声明 | 低 | 否——hooks 需要动态条件 |
| Skill 发现 | FileSystem 扫描 | `include!()` | 低 | 否——文件在编译期不可知 |
| Trust 检查 | 运行时 resolve | 无 | — | 不适合宏 |

## 决策总结

| 候选 | ROI | 是否实施 |
|---|---|---|
| `#[tool]` | 中 | ❌ 暂不，等内置工具 > 15 |
| `#[command]` | 低 | ❌ 不实施，当前已足够 |
| `#[provider]` | 中-高 | ❌ 暂不，等 provider > 5 |
| 其他 | 低 | ❌ 不实施 |

**结论**：当前**不需要**宏系统替换。运行时注册模式在 xylitol 的当前规模下足够高效，且保持了动态扩展的能力。建议在以下触发条件满足时重新评估：
- 内置工具 > 15 个
- 内置 provider > 5 个
- 启动时间中 tool/provider 注册占比 > 5%
