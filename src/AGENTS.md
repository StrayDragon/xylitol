# src/ 分层架构（代码架构 SSOT）

本文件是 `src/` **代码架构**的单一真值源：分层不变量、各层职责、应用面状态、seam、`Xy*` / 导出边界。全局工作方式与产品定调见根 `AGENTS.md`。高维 mermaid：`docs/architecture/`。短索引 / 交接：`_HANDOFF.md`。子目录 `AGENTS.md` 与 skills 只引用本文件，不重复长文。

`xylitol` 主 crate：薄编排（`agent/`）+ 运行时域（`infra/`）+ 应用面（`app/`）+ 线协议（`protocol/`）+ 领域词（`domain/`）+ ports（`runtime_protocol/`）。通用 TUI 库在 workspace 包 `packages/xylitol-tui`（不在本文件展开）。

## 主线 vs 后置（产品）

**主线（开箱即要好）**：ReAct 循环 · 内置工具（read/write/edit/bash + grep/find/ls）· 会话持久化 · compaction · Print 面 · OpenAI 兼容 + Anthropic · 项目 Trust（对齐 pi：闸项目本地资源）· 工具侧 allow-all。

**后置 / 配置启用**：Server · MCP（见下）· 更多 provider 适配器 · Export / 周边能力。未配置则不装配。

**产品 TUI（`src/app/tui`）**：**已开闸（2026-07-11）**。轨 B 至 **c493** 已归档；包侧 D08（**c575**）已归档；**c615** MessageHistory 活树已接线。引擎能力仍可在 `packages/xylitol-tui` / `agent_demo` 先行验证。

共享流水线：`bootstrap` → `composition::build_agent` → `Driver::run` → ReAct → `XyEvent` → 应用面。库嵌入入口：`xylitol::embed`；矩阵与理想/现状：`docs/architecture/库与多客户端.md`。

## 分层不变量（normative）

由 `src/tests.rs::arch_guard` 强制（代码是真值）：

- **组合根集中装配**：仅 `app/core/composition.rs` 与次级组合根 `app/cli/mod.rs`、`app/server/subcommand.rs` 可同时 import `agent` 与 `infra`。
- **agent 不依赖 infra**；**infra 不依赖 agent**。
- **domain** 零 crate 内依赖；**runtime_protocol** 只依赖 `domain`（可依赖已接受的契约级外部类型，见下「取消」）。
- **应用面走 seam、不 reach 内部**：禁止 `agent::session::*` / `agent::runtime::*` / `infra::*`；只从 `crate::agent`（mod 级）与 `crate::app::core` import。共享 seam：`composition::build_agent` → `Driver::run(prompt)` → `XyEvent` 流 → 该面渲染；不够就扩 seam，不绕过。方法论：`write-surface` skill。
- **流中改道（steer / follow-up）**：经 `Driver` 队列 API（c461），禁止应用面直接改 ReAct 内部队列。`abort` 清 steer、保留 follow_up（供 UI restore）。详见 `llmanspec/changes/c461-expose-steer-followup-seam/design.md`。

```text
app → agent → runtime_protocol → domain
  ↓     ↑
  └──── infra ───────────────────┘
protocol ───────────────────────→ domain
```

## 各层职责（摘要）

- `domain/` — 纯领域词汇与 `XyEvent` 等；零内部层依赖。
- `runtime_protocol/` — agent↔infra ports（`XyModel`/`XyTool`/`XySessionStore`/…）；近期精选 `pub use` 的主要来源。
- `infra/` — ports 的实现（provider、tools、session、config、…）；vendor 类型（async-openai、rmcp、…）关在本层。
- `agent/` — ReAct / session / model / tools 编排；公共入口为 mod 级 re-export。
- `protocol/` — `Command` / `Event` 线协议，传输无关。
- `app/` — 应用面 + `core/` seam。状态：`cli/print` ✅；`server/` ✅（Driver + REST 命令面）；`tui/` 🟢 已开闸（先 c465 bridge）；`gui` 🔴。跨面：`core/{bootstrap,dispatch,composition,driver}`。落地顺序 print → server → TUI，禁止并行铺无关骨架。

模块级文件地图以目录与代码为准；本文件不维护易变文件清单。

## `Xy*` 命名与库导出（normative）

近期要做**精选 `pub use`**（清单与注释在 `src/lib.rs`）+ 嵌入缝 `src/embed.rs`。`Xy*` 标记的是**库入口级契约**，不是所有类型都加前缀。

| 用 `Xy*` | 不用 `Xy*` |
|---|---|
| 可替换端口：`XyModel` / `XyTool` / `XySessionStore` / `XyEventSink` / … | 应用面缝：`Driver`、bootstrap、dispatch |
| 跨面生命周期：`XyEvent`、流式 `XyChunk`（及与之绑定的库级错误/模式类型） | 内部协作者、薄包装、单处 DTO、测试类型 |
| 未来进精选 `pub use` 的稳定 API | `packages/xylitol-tui`（保持零 `Xy`） |

**外部库包装**：会出现在库入口或多方言统一 → 包一层；纯内部 → 直接用 crate 类型（reqwest、glob、uuid、chrono、similar、`serde_json::Value` 等）。禁止为包而包。

**取消（CancellationToken）**：`XyToolCtx` / bash 端口直接使用 `tokio_util::sync::CancellationToken`。接受 **tokio_util 作为库公开契约依赖**（与 tokio 异步运行时绑定已成事实）；不优先自造 cancel trait。

**schemars**：配置/settings 的 `JsonSchema` derive 放在 **infra（或 config 面）**；`domain` 领域类型默认只保留 serde，避免把 schema 生成依赖绑进领域层。若某 domain 类型确需 schema，先论证是否应下沉为 config DTO。

**队列运行时**：产品语义见 `docs/architecture/插话续跑与中止.md`；实现见 archive c525。QueueUpdate MUST 进活跃 EventStream。

**EventBus / `XyEventSink`**：装配时注入的 sink 用于侧路生命周期（如 compaction），**不是**多 client 的 turn 总线；turn 进度走 `Driver::run` 的 `XyEvent` 流。可经 `BuildAgentOptions.event_sink` 替换默认 EventBus。


## Trust / Permission / MCP

- **Trust（对齐 pi）**：决定是否加载**项目本地**资源（settings、prompts、skills、themes、append system 等）。未信任则忽略项目侧配置。工具调用**不做** permission popup 平台。产品语义：`docs/architecture/信任与项目闸.md`。
- **Permission**：开箱 **allow-all**。`XyPermission` / GlobPolicy 可作可选增强，不是默认交互路径。产品语义：`docs/architecture/工具与权限.md`。
- **MCP**：`mcp_servers`（或等价）**有配置才装配**；无配置则不创建 client/工具（zero-cost）。须支持**动态配置与重载**（改配置后可热更新工具集，无需重启进程为硬性目标；实现可分阶段）。默认不进「未配置也加载」路径。产品语义：`docs/architecture/扩展能力-MCP.md`。

## Provider 适配

业务 / agent 只认 `XyModel`。infra 内用适配器族收束 OpenAI 兼容、Anthropic 及未来端点。禁止双路径包装。Pre-1.0 **交付**范围见根 `AGENTS.md`。

**HTTP 传输 vs hook 缝（隔离）**：
- 脚本 hook 三缝（`before_provider_headers` / `before_provider_request` / `after_provider_response`）只认**可移植**载荷：`infra::hooks::http::HeaderBag`（JSON map）与 `serde_json::Value` body —— **不**依赖 reqwest / 某一 vendor SDK。
- 当前传输实现（reqwest）经 `infra::provider::reqwest_bridge` 在适配器边缘转换；换 SDK = 加/换 bridge，不改 hook 合约。
- 禁止为包而包：不另造全局 `XyHttpClient`，除非出现跨方言共享且要进库入口的传输端口。
- 原始 SSE / 通道错分诊断：**优先进程内 raw provider trace**（fastrace Event + `provider-trace.jsonl`；与映射后 `XyChunk` 对照；debug 默认、release 经 `XYLITOL_PROVIDER_TRACE` —— **不**塞进 hook）。
- 观测栈：**仅 fastrace**（时间线）+ **`log`**（级别日志）；禁止 `tracing` / 双栈。外挂 MITM 提案已暂停：`llmanspec/do-not-read-me/c999-add-infra-provider-traffic-capture/`。
- **读 trace 要省 token**：禁止整文件 `Read` JSONL/log；用 `tail`/`rg`/短 `python -c`（见 `.cursor/rules/provider-trace-efficiency.mdc`）。

## 跨层测试与守卫

- `arch_guard`；BDD（`tests/features` + `tests/bdd.rs`）；回归（`tests/regression/`）。
- 设计史：`llmanspec/changes/archive/<变更>/design.md`。
