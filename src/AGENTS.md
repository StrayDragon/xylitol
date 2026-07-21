# src/ 分层架构（代码架构 SSOT）

本文件是 `src/` **代码架构**的单一真值源：分层不变量、各层职责、应用面状态、seam、`Xy*` / 导出边界。全局工作方式与产品定调见根 `AGENTS.md`。高维 mermaid：`docs/architecture/`。子目录 `AGENTS.md` 与 skills 只引用本文件，不重复长文。

`xylitol` 主 crate：薄编排（`agent/`）+ 运行时域（`infra/`）+ 应用面（`app/`）+ 统一契约层（`protocol/` = `wire/` + `ports/` + 根上共享类型）。通用 TUI 库在 workspace 包 `packages/xylitol-tui`（不在本文件展开）。

## 主线 vs 后置（产品）

**主线（开箱即要好）**：ReAct 循环 · 内置工具（read/write/edit/bash + grep/find/ls）· 会话持久化与导出 · compaction · 用量 provenance · Print + 产品 TUI（TTY 默认）· OpenAI 兼容（Completions / Responses）+ Anthropic · 项目 Trust（对齐 pi：闸项目本地资源）· 工具侧 allow-all · 图像附件。

**后置 / 配置启用**：Server · MCP（见下）· 更多 provider 适配器 · 周边能力。未配置则不装配。独立远程薄端客户端未接线（线协议与 Server 已通）。

**产品 TUI（`src/app/tui`）**：**已开闸并可用**。会话树 / slash / bang / steer / `$skill` / `/reload` / `/trust` / 粘贴与 thinking UX 等走 XyDriver + `XyEvent`。后续打磨（reload UX、MCP 启动策略等）见 `llmanspec/do-not-read-me/` purpose-drafts；勿在本文件钉 change id。引擎能力仍可在 `packages/xylitol-tui` / `agent_demo` 先行验证。

共享流水线：`bootstrap` → `composition::build_agent` → `XyDriver::run` → ReAct → `XyEvent` → 应用面。库嵌入入口：`xylitol::embed`；矩阵与理想/现状：`docs/architecture/库与多客户端.md`。

## 分层不变量（normative）

单 crate 逻辑分层：约定写在本文件，靠 review 与缝/行为测守住。**禁止**用源码 grep 元测试卡 import 路径；**不**为分层拆 crate（编译产物膨胀）。代码结构与 seam 行为仍是真值。

- **组合根集中装配**：仅 `app/core/composition.rs` 与次级组合根 `app/cli/mod.rs`、`app/server/subcommand.rs`（及文档化的 `rpc` 等）可同时 import `agent` 与 `infra` 做装配。
- **agent 不依赖 infra**；**infra 不依赖 agent**（经 `protocol::ports`）。
- **protocol** MUST NOT 依赖 `agent` / `infra`；根上共享类型零内部层依赖（MAY 依赖 workspace 包 **DTO only**：`xylitol_ai_bridge::dto`）；`wire/` MUST NOT 依赖 `ports/`；`ports/` MAY 用根类型 + bridge DTO，MUST NOT 依赖 `wire/`。禁止再建 `domain/` / `vocab/` / `types/` 第三顶栏。
- **应用面走 seam、不 reach 内部**：禁止 `agent::session::*` / `agent::runtime::*` / `infra::*`；只从 `crate::agent`（mod 级）与 `crate::app::core` import。共享 seam：`composition::build_agent` → `XyDriver::run(prompt)` → `XyEvent` 流 → 该面渲染；不够就扩 seam，不绕过。方法论：`write-surface` skill。
- **`XyInProcessDriver` 表面 infra**：trust / clipboard / 必要 config 读可在 XyDriver 内调 `infra`（面仍禁止 reach）。provider / session / 默认工具集装配仍归 `composition`。内部搬家以 seam 行为保持绿为准。
- **流中改道（steer / follow-up）**：经 `XyDriver` 队列 API（c461），禁止应用面直接改 ReAct 内部队列。`abort` 清 steer、保留 follow_up（队列条可见；Alt+Up 还原编辑器）。详见 archive `c461-expose-steer-followup-seam/design.md`。

```text
app → agent → protocol/{wire,ports + root types}
  ↓     ↑
  └──── infra ───────────────────┘
```

## 各层职责（摘要）

- `protocol/` — 方案 B：`wire/`（client↔core `Command`/`Event`）+ `ports/`（agent↔infra `XyModel`/`XyTool`/…）+ 根上共享类型（`AgentMessage`/`XyEvent`/`XyChunk`/session entries…）。进根门槛 = 出现在 port/wire 签名或跨 agent/infra 共享。
- `infra/` — ports 的实现（provider、tools、session、config、…）；vendor 类型（async-openai、rmcp、…）关在本层。
- `agent/` — ReAct / session / model / tools 编排 + `project_for_llm`；公共入口为 mod 级 re-export（含 `AgentMessage`/`XyEvent` 语义再导出）。
- `app/` — 应用面 + `core/` seam。状态：`cli/print` ✅；`server/` ✅；`tui/` ✅ 已开闸可用；`gui` 🔴。跨面：`core/{bootstrap,dispatch,composition,driver}`。

模块级文件地图以目录与代码为准；本文件不维护易变文件清单。

## `Xy*` 命名与库导出（normative）

近期要做**精选 `pub use`**（清单与注释在 `src/lib.rs`）+ 嵌入缝 `src/embed.rs`。`Xy*` 标记的是**库入口级契约**，不是所有类型都加前缀。

| 用 `Xy*` | 不用 `Xy*` |
|---|---|
| 可替换端口：`XyModel` / `XyTool` / `XySessionStore` / `XyEventSink` / … | bootstrap / dispatch 的装配细节（非协议类型名） |
| **共享应用协议**：`XyDriver` / `XyInProcessDriver` / `XyDriverError` | 内部协作者、薄包装、单处 DTO、测试类型 |
| 跨面生命周期：`XyEvent`、流式 `XyChunk` | `packages/xylitol-tui`（保持零 `Xy`） |
| 库级错误：`XyError` / `XyToolError`；整机缝错误：`XyDriverError` | |
| 未来进精选 `pub use` 的稳定 API | |

**外部库包装**：会出现在库入口或多方言统一 → 包一层；纯内部 → 直接用 crate 类型（reqwest、glob、uuid、chrono、similar、`serde_json::Value` 等）。禁止为包而包。

**取消（CancellationToken）**：`XyToolCtx` / bash 端口直接使用 `tokio_util::sync::CancellationToken`。接受 **tokio_util 作为库公开契约依赖**（与 tokio 异步运行时绑定已成事实）；不优先自造 cancel trait。

**schemars**：配置/settings 的 `JsonSchema` derive 放在 **infra（或 config 面）**；protocol 根共享类型默认只保留 serde，避免把 schema 生成依赖绑进契约层。若某类型确需 schema，先论证是否应下沉为 config DTO。

**队列运行时**：产品语义见 `docs/architecture/插话续跑与中止.md`；实现见 archive c525。QueueUpdate MUST 进活跃 EventStream。

**EventBus / `XyEventSink`**：装配时注入的 sink 用于侧路生命周期（如 compaction），**不是**多 client 的 turn 总线；turn 进度走 `XyDriver::run` 的 `XyEvent` 流。可经 `BuildAgentOptions.event_sink` 替换默认 EventBus。

## 扩展决策准则（normative）

产品心智（可插拔 ≠ 插件市场）：`docs/architecture/扩展与开闭.md`。此处只写**实现侧**何时开闭、开哪一层。

### 三层扩展手段（按优先序）

| 手段 | 何时用 | 代价 |
|---|---|---|
| **配置 / 会话开关 + composition 接线** | 单一（或极少）实现；启用才装配 | 最低；未启用须 zero-cost |
| **既有 port**（`XyTool` / `XyModel` / `XyHookBus` / …） | 新能力可映射到已有契约 | 中；禁止旁路第二套语义 |
| **新 `protocol::ports` trait** | 已有**真实第二实现**，或测试/嵌入方必须替换 | 最高；须论证为何不能复用上两行 |

**禁止**：Extension Host、插件清单市场、为未交付能力预挖空 `Xy*`、「已是统一口再包一层」双路径。

### 升格 / 降级触发

- **升格为 port**：出现第二个生产实现，或嵌入方/测试必须注入替换，且复用 `XyTool`/`XyModel`/缝会扭曲语义。
- **保持具体类型 + 组合根**：仅 bootstrap/composition 知道具体类型；agent 只拿投影后的数据或既有 port（例：资源发现结果经 options 注入，不必强行 `dyn XyResourceLoader`）。
- **`XyReloadable`**：编译期刷新约束（关联 `Outcome`），**不是** dyn 插件注册表；`/reload` 按固定顺序编排具体 reload。
- **进精选 `pub use`**：仅库嵌入方稳定需要的端口/事件；`XyReloadable` / `XyResourceLoader` / `XyTrustStore` 等可留在 `protocol::ports` 而不进 crate 根，直到嵌入契约需要。

### 端口健康（审计快照 · 2026-07-21）

| 端口 | 角色 | 备注 |
|---|---|---|
| `XyModel` / `XyModelBuilder` | 热路径可替换 | 开闭范例：ai-bridge adapter |
| `XyTool` | 内置 + MCP 同口 | 新外部能力优先落此 |
| `XySessionStore` / `XyEventSink` / `XyExportIo` / `XyBashExecutor` | agent 注入 | 组合根装配 |
| `XyPermission` | 建议性门控 | 开箱 allow-all；非 popup 平台 |
| `XySecretResolver` | 模型注册表 | |
| `XyHookBus` | 脚本/库钩子 | 空配置 = 不装配 |
| `XyTrustStore` | Trust 读写 | `&dyn` 调用；未进精选 `pub use` |
| `XyResourceLoader` | 资源发现抽象 | **暂无生产 `dyn` 消费者**；具体 loader + inherent 为主；升格前先接线再谈精选导出 |
| `XyReloadable` | 热重载约束 | 非 dyn 注册表 |

后置（LSP / DAP / Sub-agent / 即时设置）认领时：默认走「开关 + `XyTool`/`XyDriver`/`XyEvent`」，**先证明**需要新 port 再提案；产品轴见 `docs/architecture/扩展与开闭.md`。

## Trust / Permission / MCP

- **Trust（对齐 pi）**：决定是否加载**项目本地**资源（settings、prompts、skills、themes、append system 等）。未信任则忽略项目侧配置。工具调用**不做** permission popup 平台。产品语义：`docs/architecture/信任与项目闸.md`。
- **Permission**：开箱 **allow-all**。`XyPermission` / GlobPolicy 可作可选增强，不是默认交互路径。产品语义：`docs/architecture/工具与权限.md`。
- **MCP**：`mcp_servers`（或等价）**有配置才装配**；无配置则不创建 client/工具（zero-cost）。须支持**动态配置与重载**（改配置后可热更新工具集，无需重启进程为硬性目标；实现可分阶段）。默认不进「未配置也加载」路径。产品语义：`docs/architecture/扩展能力-MCP.md`。

## Provider 适配

业务 / agent 只认 `XyModel`。厂商 HTTP/SSE **优先经官方 SDK Client**（OpenAI：`async-openai`；Anthropic：官方 Rust SDK 未成熟前 **reqwest 兜底**）落在 workspace 包
`packages/xylitol-ai-bridge`；主仓分工：

1. **agent**：调用前 `project_for_llm`（`AgentMessage` → `Vec<LlmMessage>` / `AiBridgeMessage`；Llm 臂 passthrough，Env 折叠）
2. **infra/provider**：装配 `XyModel` / 配置（base_url、密钥）；chunk / tool-schema / error 边界映射（`map.rs`）。**MUST NOT** 再对 `AgentMessage` 做 Env 折叠主路径。

**概念分层（MUST）**：

| 类型 | 含义 |
|---|---|
| `AgentMessage`（`protocol/message`；agent 再导出） | session **真源** = `Llm(AiBridgeMessage)` \| `Env(EnvMessage)`（`LlmMessage` / Part / Usage… 为 bridge DTO 的 `pub use` 别名） |
| bridge LLM DTO（`AiBridge*`） | LLM 叶 SSOT；`XyModel::generate_stream` **只吃** `Vec<LlmMessage>` |
| protocol → bridge | **MAY** 依赖 **DTO only** 完成组合；**MUST NOT** 依赖 bridge HTTP / vendor SDK |

**叶类型归属（c1210 / c1220）**：

| 叶 / 组合 | 归属 | 说明 |
|---|---|---|
| `AiBridgeMessage` / Part / StopReason / Usage | **bridge**（protocol 别名） | LLM 叶 SSOT |
| `EnvMessage` / `AgentMessage` / `XyEvent` | **protocol 根** | 跨 agent/infra；agent 语义再导出 |
| `project_for_llm` | **agent** | 投影 MUST 在 agent |
| chunk / provenance 等真边界差 | `infra/provider/map` | 无 AgentMessage 折叠路径 |

调优史：`src/_TODO.md` §F。设计史：**c1070** / **c1210** / **c1220**。

**开闭**：新 OpenAI-like / Anthropic-like 兼容端 = 新 adapter 或配置；**MUST NOT** 为网关改 `AgentMessage` / ReAct。禁止 Completions「已是 `XyModel` 再包一层」双路径。Pre-1.0 **交付**范围见根 `AGENTS.md`。细则与包边界（含 Responses 流式 BYOT/`Value`）：`packages/xylitol-ai-bridge/AGENTS.md`。

**HTTP / SDK vs hook 缝（隔离）**：
- 脚本 hook 三缝只认**可移植**载荷：`HeaderBag`（JSON map）与 `serde_json::Value` body —— **不**把 reqwest / 某一 SDK 类型泄漏进 hook 合约。
- 传输实现经 SDK **middleware**（或薄 bridge）在适配器边缘对接；换 SDK = 换 middleware，不改 hook 合约。
- 禁止为包而包：不另造全局 `XyHttpClient`，除非出现跨方言共享且要进库入口的传输端口。
- 原始 SSE / 通道错分诊断：**优先进程内 raw provider trace**（fastrace Event + `provider-trace.jsonl`；与映射后 `XyChunk` 对照；debug 默认、release 经 `XYLITOL_PROVIDER_TRACE` —— **不**塞进 hook）。
- 观测栈：**仅 fastrace**（时间线）+ **`log`**（级别日志）；禁止 `tracing` / 双栈。外挂 MITM 提案已暂停：`llmanspec/do-not-read-me/c999-add-infra-provider-traffic-capture/`。
- **读 trace 要省 token**：禁止整文件 `Read` JSONL/log；用 skill **`xylitol-inspect-runtime-logs`** → `scripts/inspect_provider_trace.py` / `just obs-*`（summary / lag / lifecycle / turns / channel；过滤 `--since` / `--turn-id` / `--request-id`）。

## 跨层测试

- BDD（`tests/features` + `tests/bdd.rs`）；回归（`tests/regression/`）；应用面 / XyDriver 行为测。
- 设计史：`llmanspec/changes/archive/<变更>/design.md`。

## 体量调音（指针）

生产模块行数预算、超标表、下次调音日 → [`QUALITY_RETUNE.md`](./QUALITY_RETUNE.md)。质量调优待办（临时）→ [`_TODO.md`](./_TODO.md)。根 AGENTS「维护习惯」含长期调音 rule。
