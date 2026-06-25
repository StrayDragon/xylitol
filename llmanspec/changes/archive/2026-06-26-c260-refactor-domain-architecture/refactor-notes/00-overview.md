# xylitol 架构：核心概念与硬约束

> 状态：锚点文档（2026-06-26）
> 地位：本目录所有方案的**上位约束**。任何重构/新功能必须先与本文件的概念定义和硬约束对齐。
> 风格参照：把"项目地图 + 硬约束 + 工作流要求"限制在热路径上，详见各子目录的细化文档。

---

## 0. 本项目是什么（一句话）

> **xylitol = 为服务商提供的 model，装配一系列运行时（runtime），并用一个最小的编排循环 + hook 把它们驱动起来的工具集。**

一切设计服从这句话。它决定了：
- **`infra/` 是大领域**——所有"运行时"住在这里：model 运行时（provider）、tool 运行时、session 运行时、文件/进程/sandbox 执行环境运行时、config/hooks/mcp/skills 运行时。
- **`agent/` 必须薄**——只保留最基本的编排循环（ReAct loop）和 hook 切入面。它**不持有运行时实现**，只通过 `core` 的 port（trait）引用它们。
- **交互形态是可替换的表皮**——CLI/TUI/Web 都是 client，通过统一协议与"常驻工作的核心"对话。

---

## 1. 模块地图（Project Map）

| 模块 | 一句话职责 | 依赖方向 |
|---|---|---|
| `core/` | 词汇 + port（trait 契约）+ 纯算法。零 crate 依赖。 | 无 |
| `infra/` | **运行时域**：为 model 提供的全部运行时实现（provider/tool/session/exec-env/...）。实现 `core` 的 port。 | → `core` |
| `agent/` | **薄编排**：ReAct 循环 + hook 切入面 + 编排状态（当前用哪些运行时）。只通过 port 引用 `infra`。 | → `core`（不直接 import `infra` 具体类型） |
| `protocol/` | **统一交互契约**：命令（client→server）+ 事件（server→client）+ 信封 + 错误码。线协议的 SSOT。 | → `core`（仅类型） |
| `server/` | **常驻核心**：托管 `agent` + `infra` 运行时，通过 REST+WS 暴露 `protocol`。单实例、可重连、可水平扩展。 | → `agent` `infra` `protocol` |
| `interactive/` | **交互形态（client）**：`cli`/`tui`/`web`。只依赖 `protocol` + 一个 `Driver`，**禁止直接依赖** `agent`/`infra`。 | → `protocol`（+ 可选本地 `Driver`） |

> NOTE: `interface/` 将重命名为 `interactive/`——"interface"与编程语义（trait/接口）冲突，且新名准确表达"代码对交互形态的支持"。

### 依赖方向图

```
                        core  (词汇 + port)
                      ↗  ↑  ↖
            infra ───┘  │   └───→ protocol (仅类型)
           (运行时域)    │            │
                        │            ↓
                     agent ───────→ server ── REST+WS ──→ ┐
                   (薄编排+hook)     (常驻核心)             │
                        │                                  │
                        └── InProcessDriver ──→ interactive (client)
                                                   cli / tui / web
```

硬规则：箭头不可反向。`infra` 永远不依赖 `agent`；`interactive` 永远不直接依赖 `agent`/`infra`。

---

## 2. 部署形态（两种，共享同一 protocol）

| 形态 | 核心 | 交互 | 适用 |
|---|---|---|---|
| **本地单进程**（当前） | `agent`+`infra` 在 `interactive/cli` 进程内直接装配 | `InProcessDriver` 直接调用 agent | 单机、无网络、零延迟 |
| **server + 多 client**（新增） | `server` 常驻，托管 `agent`+`infra` | `interactive/*` 通过 `RemoteDriver`（WS/REST）连接 | 核心常跑、交互可离线、水平扩展、多控一/一控多 |

**关键不变量**：两种形态下，`interactive/*` 的代码**完全相同**——它们只认 `protocol` + `Driver`。本地与远程的切换只是换一个 Driver 实现。这是"统一交互方式"的兑现。

---

## 3. 硬约束（Hard Constraints）

> 这些是"防混乱"的护栏。任何 PR 违反即应被 review 拒绝。

### HC-1 · 层依赖方向不可逆

- `core` 零 crate 内部依赖。
- `infra` 不依赖 `agent`。`grep -rn "crate::agent::" src/infra/` 必须为 0。
- `agent` 不直接 import `infra` 的具体类型（只通过 `core` port）。`grep -rn "crate::infra::provider::[A-Z]\|crate::infra::tools::[A-Z]" src/agent/` 必须为 0（装配在 server/cli 组合根发生）。
- `interactive` 不直接依赖 `agent`/`infra`，只依赖 `protocol` + `Driver`。

### HC-2 · `agent` 必须可独立使用

- `agent` 的编排入口（`Agent`/loop）构造**不强制**创建 Session，**不持有** `session_id`，**不依赖** Session 持久化生命周期。
- Session 等运行时作为 port（`Arc<dyn Port>`）注入；agent 只编排，不拥有后端。
- 这是 kimi-code 同名硬规则的直接借鉴——保证 agent 可脱离文件系统单测、可在 server 中被自由托管。

### HC-3 · 交互必须经统一 protocol

- 所有 client（cli/tui/web）与核心的交互，走 `protocol/` 定义的命令与事件。
- 线类型（wire types）定义在 `protocol/`，**客户端本地也可重实现**（不强制从 core 派生），以彻底切断编译期耦合。
- 现有 `interactive/rpc.rs`（JSONL over stdio）是 protocol 的**胚胎**，应演进为正式 `protocol/` 模块。

### HC-4 · 新增能力 = 新文件 impl port（开闭）

- 新增 LLM 厂商 = 在 `infra/provider/` 加一个文件 `impl XyModel` + 组合根注册。
- 新增工具 = 在 `infra/tools/`（或 agent 内置工具区）加一个文件 `impl XyTool` + 注册。
- 新增交互形态 = 在 `interactive/` 加一个目录（`tui/`、`web/`），`impl Driver` 适配。
- **零改动既有编排代码**。做不到这一点，说明接缝（port）画错了。

### HC-5 · Trait 只存在于真接缝（抗过度抽象）

- trait 立项判据：存在 ≥2 实现（含 test double），或跨层解耦确实承重。
- 单实现的 trait = 过度抽象，内联。`BashOperations`/`ModelConfigExt`/`SettingsStorage` 当前疑似单实现，需核实后处理。

### HC-6 · 有意识的简化必须留 NOTE

- 任何为求简洁留下的天花板，用工具无关标注：
  ```
  // NOTE: <做了什么简化>. 天花板: <何时失效>. 升级: <触发条件>.
  ```
- 便于全仓库 `grep -rn "NOTE:"` 回收，避免简化静默腐化。

---

## 4. 核心概念词汇表（防概念漂移）

| 术语 | 定义 | 不可混淆为 |
|---|---|---|
| **runtime（运行时）** | 为 model 提供的某项能力实现（provider/tool/session/sandbox/...），住 `infra/`，impl `core` port | ≠ trait；trait 是契约，runtime 是实现 |
| **port（端口）** | `core` 中定义的反向 trait 契约，表达"agent 需要外部提供什么"。如 `XyModel`/`XyTool`/`SessionStore` | ≠ Rust `port`；≠ `interface`（已弃用名） |
| **adapter（适配器）** | `infra` 中 port 的具体实现 | — |
| **agent（编排）** | 驱动运行时的最小循环 + hook。**薄** | ≠ 整个项目；≠ Session |
| **interactive（交互形态）** | client 表皮：cli/tui/web。只认 protocol+Driver | ≠ agent；不可含业务逻辑 |
| **server（常驻核心）** | 托管 agent+infra、暴露 protocol 的进程 | ≠ interactive |
| **protocol（统一交互契约）** | 命令+事件+信封的 SSOT，本地与远程共用 | ≠ 任意 RPC；它是**唯一**交互形态 |
| **Driver（驱动）** | interactive 侧的抽象：`InProcessDriver`（本地）/ `RemoteDriver`（WS） | — |

> 新概念引入必须先在本表登记 + 在模块地图补一句话职责。未登记的概念不得进代码。这是"不让后续概念把项目弄失控"的执行机制。

---

## 5. 目标模块树（终态）

```
src/
├── core/           词汇 + port + 纯算法（零依赖）
├── infra/          运行时域（大领域）
│   ├── provider/   model 运行时（OpenAI/Anthropic…，impl XyModel）  [从 agent/ 迁入]
│   ├── tools/      内置工具运行时（bash/read/write…，impl XyTool）   [从 agent/ 迁入，后期]
│   ├── session/    session 运行时（文件后端，impl SessionStore）
│   ├── sandbox/ process/ fs_watch/ git/   执行环境运行时
│   └── config/ mcp/ hooks/ skills/ resource/ trust/ event/ …
├── agent/          薄编排（只循环 + hook + 编排状态）
│   ├── react.rs    ReAct 循环
│   ├── hooks.rs    AgentHooks 切入面
│   └── (编排状态：当前 model/tools 选择，仅持有 Arc<dyn Port>)
├── protocol/       统一交互契约（命令 + 事件 + 信封 + 错误码）
├── server/         常驻核心（托管 agent+infra，REST+WS 暴露 protocol）
├── interactive/    交互形态（client）  [由 interface/ 重命名]
│   ├── cli/        命令行 + print（本地，InProcessDriver）
│   ├── tui/        （未来）终端 UI client
│   ├── web/        （未来）web client
│   └── driver.rs   Driver 抽象（InProcess / Remote）
```

---

## 6. 重构方案索引

| 文档 | 方案 | 服从的硬约束 | 风险 |
|---|---|---|---|
| [01-plan-a-consolidate.md](01-plan-a-consolidate.md) | A — 收敛归位（含 `interface→interactive` 改名） | HC-1, HC-6 | 极低 |
| [02-plan-b-decouple.md](02-plan-b-decouple.md) | B — agent 瘦身 + facade/Driver 雏形 | HC-1, HC-2 | 低-中 |
| [03-plan-c-ports-adapters.md](03-plan-c-ports-adapters.md) | C — port 收口 + 运行时迁入 infra | HC-1, HC-2, HC-4, HC-5 | 中-高 |
| [04-server-and-protocol.md](04-server-and-protocol.md) | D — server 常驻核心 + protocol 统一交互 | HC-2, HC-3, HC-4 | 中-高 |

### 推荐执行顺序

```
A（清理 + 改名）→ B（agent 瘦身）→ C（port/运行时迁层）→ D（server + protocol）
                                                          ↑
                        D 依赖 C 的"agent 只认 port"成果，故排在最后
```

> NOTE: D 的 protocol 部分可**提前到 B 之后**启动（把 rpc.rs 演进为 protocol/），与 C 并行。但 server 的常驻托管依赖 agent 足够薄（C 成果），故 server 进程化排最后。

---

## 7. 工作流要求（落地护栏）

- **PR 前自检**：改动是否引入新概念？是→先登记词汇表+地图。改动是否破坏层方向？跑 HC-1 的 grep 断言。
- **层边界编译期断言**：建议加一个 `tests/architecture.rs`（或 `build.rs` grep 断言），把 HC-1 的 grep 固化为 CI 红线。
- **新模块落地三问**（写进 AGENTS.md）：
  1. 它是 port 实现（运行时）吗？→ `infra/`，加文件 impl trait。
  2. 它是纯编排/hook 切入面吗？→ `agent/`，且必须薄。
  3. 它是交互表皮吗？→ `interactive/`，且只认 protocol+Driver。
  4. 三者都不是 → 质疑存在性（HC-5 第一级：YAGNI）。
- **保持变更聚焦**：重构 PR 不夹带无关功能改动；功能 PR 不夹带架构重构（kimi-code 同款原则）。
