---
depends_on: []
---

# c260-refactor-domain-architecture

## Why

xylitol 当前是"能跑但结构失衡"的状态。四层划分（`core ← {agent, infra} ← interface`）的边界守住了，但 `src/agent/` 已退化为杂物袋：单层塞了 9 类职责（provider HTTP / ReAct 循环 / 会话状态 / 工具 / prompt / 命令 / 压缩 / bash 执行 / 配置解析），`AgentSession` 是 1500+ 行的 god-object，`interface` 深入 agent 内部三个子模块，且出现"core/agent 同名模块遮蔽"与"`r#loop` 关键字转义"等长期 papercut。

本次重构确立项目本质的清晰定义：**xylitol = 为服务商提供的 model，装配一系列运行时（runtime），用最小编排循环 + hook 驱动它们**。由此推出三条硬约束：

1. **运行时住 `infra/`**（provider/tool/session/exec-env…）——infra 是大领域。
2. **`agent/` 必须薄**——只留 ReAct 循环 + hook 切入面 + 编排状态，通过 port 引用运行时，不直接持有运行时具体类型。
3. **交互形态（`interactive/`）是可替换表皮**——cli/tui/web 都只是 client，经统一 `protocol` + `Driver` 与"常驻工作的核心"对话，不直接依赖 `agent`/`infra`。

在此定义下补齐 **server 常驻模式**：核心常驻可水平扩展，交互可离线重连，本地与远程 client 代码逐字相同。

### 为何是单一 change 而非 A/B/C/D 四个 change

A+B+C+D 是一个连贯愿景：薄 agent（C）是 server 托管（D）的前提，port 收口（C）是 Driver 抽象（D）的前提，interface→interactive 改名（A）是 protocol 落位的词汇前提。拆成四个 change 会割裂上下文、产生中间态兼容 shim（违反"不留兼容"原则）。本次用 `tasks.md` 按 P0→Pn 阶段拆分，每阶段后全测试套件绿灯，整个 change 走完一次性 archive。

> 参照设计：`../kimi-code` 的 `agent-core`（引擎）+ `server`（托管 + REST/WS）+ `apps/*`（client，禁止直接依赖引擎层）+ `protocol`（统一线协议）四分。详细分阶段方案见本 change 内的参考材料 `refactor-notes/00-overview.md`~`04-server-and-protocol.md`（核心内容已融入 design.md，refactor-notes 保留为逐项细化参考，归档时随 change 一同冻结，不进主 `docs/`）。

## What Changes

### P0 — 收敛归位 + 词汇改名（行为不变）
1. 删除死代码 `agent/types.rs`（8 行 re-export，0 引用）。
2. 吸收 orphan manager：`agent/model_manager.rs`→`agent/model/manager.rs`、`agent/compaction_orchestrator.rs`→`agent/compaction/orchestrator.rs`、`agent/skill_manager.rs`→`agent/skills.rs`。
3. `agent/config_value.rs`（624 行，0 crate 依赖）→`infra/config/value.rs`（迁回运行时域）。
4. `interface/`→`interactive/`（"interface"与 trait 语义冲突；新名表达"交互形态支持"）。

### P1 — 杀 `r#loop` + runtime 目录（行为不变）
5. 建 `agent/runtime/`：`loop.rs`→`react.rs`、`queue.rs`、`retry.rs`、`output_guard.rs`→`stdout_guard.rs`。消灭全 crate 4 处 `r#loop` 转义。

### P2 — agent 瘦身 + facade（行为不变）
6. 拆 god-file `runtime/react.rs`→`react.rs`+`hooks.rs`+`event.rs`。
7. `bash_executor.rs`→`runtime/bash.rs`；合成 `agent/prompt/` 子层（commands/templates/system/skills）。
8. 新增 `agent/facade.rs`——交互层唯一入口（Driver 雏形）。
9. 收口 `interactive/{rpc,print,cli}` 依赖，仅认 facade。

### P3 — port 收口 + 运行时迁 infra（行为不变）
10. `core/traits.rs`→`core/ports.rs`（port 集中）。
11. `agent/provider/`整体迁`infra/provider/`（provider 是连接外部 API 的运行时）。
12. 内置工具 `agent/tools/{bash,read,write,...}`→`infra/tools/`（实现迁层，`ToolRegistry` 注册表留 agent）。
13. `agent/` 不得再 import 任何 infra 具体类型（架构断言固化）。

### P4 — HC-2 修正（行为不变）
14. `Agent::new(session)`/`run(prompt, session_id)`→`Agent::new(ports...)`/`run(prompt)`。agent 不强制 Session、不持有 session_id。新增 `SessionStore`/`EventSink` port（infra impl + test 内存双）。
15. 新增 `tests/architecture.rs`——编译期/CI 固化层边界 grep 规则（HC-1）。

### P5 — server 常驻 + protocol 统一交互（行为新增）
16. 抽 `interactive/rpc.rs` 的类型到 `protocol/`（命令 + 事件 + 信封 + 错误码，SSOT）。rpc.rs 退化为 stdio transport。
17. 新增 `interactive/driver.rs`（`Driver` trait + `InProcessDriver` + `RemoteDriver`）。
18. 新增 `server/`：常驻核心（runtime 装配 + REST `/api/v1` + WS + 单实例锁）。
19. 重连 journal + `resync_required`（交互可离线）。
20. 反向 RPC（approval/question gateway）。

### 收尾
21. 更新 `AGENTS.md` 的 Project Structure 段，同步新模块树；refactor-notes 保留在 change 内随归档冻结（不进主 `docs/`）。

## Capabilities

- `layer-architecture`（modify + add）：层方向扩展到 protocol/server/interactive；新增 HC-2/HC-3/HC-4 约束 + 架构断言。
- `agent-runtime`（modify）：薄化为循环 + hook；杀 r#loop；facade。
- `agent-session`（modify）：god-object 瘦身；去 session_id 耦合（HC-2）。
- `provider-integration`（modify）：迁入 infra；port 注入。
- `tool-system`（modify）：实现迁 infra，注册表留 agent。
- `cli-entry`（modify）：改 interactive；改认 facade/Driver。
- `session-persistence`（modify）：impl SessionStore port。
- `runtime-config`（modify）：吸收 config_value（→ infra）。
- `interactive-protocol`（**新增**）：统一交互契约（命令/事件/信封/错误码）。
- `server-runtime`（**新增**）：常驻核心 + REST/WS + 重连 + 反向 RPC。
- `interactive-client`（**新增**）：Driver 抽象 + cli/tui/web 表皮契约。

## Impact

- **编译期**：大量模块路径变更，零运行时行为变更（P0~P4）；P5 新增 server 进程能力。
- **测试**：现有 BDD 14 个 feature 必须全绿（行为不变护栏）；新增 architecture guard 测试；rpc.feature 需适配 protocol 演进。
- **依赖**：P5 引入 HTTP/WS 框架（候选 axum，与既有 tokio 契合），需更新 Cargo.toml。
- **文档**：主 `docs/` 保持干净（不含临时重构笔记）；架构核心内容进 design.md，逐项细化保留在 `refactor-notes/` 随 change 归档冻结；AGENTS.md 的 Project Structure 段需同步模块树。
- **风险**：高。god-object 拆分（borrow/生命周期）+ port 边界设计 + 重连状态机是三大难点。`tasks.md` 按 ≤2h 粒度拆分，每阶段全绿才进下一阶段。
