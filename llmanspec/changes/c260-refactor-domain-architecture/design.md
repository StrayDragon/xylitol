# c260-refactor-domain-architecture — Design

> 本文档记录本次架构重构的核心定义、关键权衡、决策依据与迁移路径，自洽可读。详细的分阶段细化方案（含逐项 `git mv` 命令、每阶段验证断言全集）见本 change 内的参考材料 `refactor-notes/00..04`（归档时随 change 一同冻结，不进主 `docs/`）。

## 0. 项目本质与模块地图

**项目本质**：xylitol = 为服务商的 model 装配一系列运行时（runtime），用最小编排循环（agent）+ hook 驱动它们。一切设计服从这句话。

### 模块地图

| 模块 | 一句话职责 | 依赖方向 |
|---|---|---|
| `core/` | 词汇 + port（trait 契约）+ 纯算法。零 crate 依赖。 | 无 |
| `infra/` | **运行时域**：为 model 提供的全部运行时实现（provider/tool/session/exec-env/...）。实现 `core` port。 | → `core` |
| `agent/` | **薄编排**：ReAct 循环 + hook 切入面 + 编排状态。只通过 port 引用 `infra`。 | → `core`（不直接 import `infra` 具体类型） |
| `protocol/` | **统一交互契约**：命令（client→core）+ 事件（core→client）+ 信封 + 错误码。线协议 SSOT。 | → `core`（仅类型） |
| `server/` | **常驻核心**：托管 `agent` + `infra`，通过 REST+WS 暴露 `protocol`。单实例、可重连、可水平扩展。 | → `agent` `infra` `protocol` |
| `interactive/` | **交互形态（client）**：`cli`/`tui`/`web`。只依赖 `protocol` + 一个 `Driver`，禁止直接依赖 `agent`/`infra`。 | → `protocol`（+ 可选本地 `Driver`） |

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

### 部署形态（两种，共享同一 protocol）

| 形态 | 核心 | 交互 | 适用 |
|---|---|---|---|
| **本地单进程**（当前） | `agent`+`infra` 在 `interactive/cli` 进程内直接装配 | `InProcessDriver` 直接调用 agent | 单机、无网络、零延迟 |
| **server + 多 client**（新增） | `server` 常驻，托管 `agent`+`infra` | `interactive/*` 通过 `RemoteDriver`（WS/REST）连接 | 核心常跑、交互可离线、水平扩展、多控一/一控多 |

关键不变量：两种形态下 `interactive/*` 代码**完全相同**，只认 `protocol` + `Driver`；本地/远程切换只是换 Driver 实现。

## 1. 核心定义与三条硬约束

由此本质推出（详见 `layer-architecture` spec 的 HC 约束）：

| 约束 | 内容 | 验证 |
|---|---|---|
| HC-1 层方向不可逆 | `core` 零内部依赖；`infra` 不依赖 `agent`；`agent` 不直接 import infra 具体类型；`interactive` 只依赖 protocol+Driver | `tests/architecture.rs` grep 断言 |
| HC-2 agent 可独立使用 | `Agent::new` 不强制 Session、不持有 session_id、不依赖 Session 生命周期 | facade 签名 + 单测 |
| HC-3 交互必经 protocol | 所有 client↔核心交互走 `protocol/`；线类型在 protocol 定义 | rpc/tui/web 共用同一 Driver |
| HC-4 开闭 | 新增 provider/tool/后端/交互形态 = 新文件 impl port/Driver，零改既有编排 | 新增厂商测试 |
| HC-5 trait 只在真接缝 | trait 立项需 ≥2 实现或承重解耦；单实现内联 | review 时填自检表 |
| HC-6 NOTE 标注 | 有意简化用 `// NOTE: <做了什么>. 天花板: <>. 升级: <>.` | grep 回收 |

## 2. 关键权衡

### 2.1 单一 change vs 四个 change

**决策**：单一 change，tasks 分阶段。

**理由**：A+B+C+D 互为前提（薄 agent 是 server 托管前提、port 收口是 Driver 前提、改名是 protocol 词汇前提）。拆四个 change 必然产生中间态兼容 shim（如"facade 暂留 session_id 等下个 change 修"），违反用户"不留兼容、一步到位"要求。SDD 的 change 是规划/归档单元，tasks.md 天然支持阶段拆分。

**代价**：单 change 体量大，archive 时一次性合并 11 个 capability 的 spec delta。用"每阶段全绿才进下一阶段"对冲。

### 2.2 运行时迁 infra vs 留 agent

**决策**：provider 和内置工具的实现整体迁 `infra/`；`ModelRegistry`/`ToolRegistry` 注册表（纯数据结构）留 agent。

**理由**：provider/tools 是"连接外部世界"的运行时，符合 infra 大领域定位。注册表是"当前编排状态"（`Vec<Arc<dyn Port>>`），属编排范畴。

**对照方案**（已否决）：全部留 agent——违反"agent 薄"和 HC-1（agent 不持有运行时具体类型）。

### 2.3 新增 port 的范围（抗 HC-5 过度抽象）

**决策**：仅立 3 个新 port（`SessionStore`/`EventSink`/`SecretResolver`），且 `SecretResolver` 设触发条件。

| Port | 第二实现 | 承重 | 立项 |
|---|---|---|---|
| `SessionStore` | test 内存后端 | agent 脱离文件系统单测 + server 托管 | ✅ |
| `EventSink` | test 事件收集器 | loop 单测事件流 | ✅ |
| `SecretResolver` | env/cmd/static | 仅当 agent 内部需解析时 | ⚠️ 触发型 |

**已否决**：`BashOperations`（单实现，过度抽象，内联）、`ModelConfigExt`（单实现，改普通 fn）、`SettingsStorage`（单实现，内联）。

### 2.4 protocol 形态：enum vs 命令对象

**决策**：命令/事件用 `enum`（SSOT），而非每命令一 struct。

**理由**：一个 enum 一眼看全交互面，新增能力 = 加 variant。wire 兼容靠 `#[serde(other)]` 兜底或版本号。比"一堆命令对象"更利于审计交互面。

### 2.5 server 重连：journal + resync

**决策**：server 每 session 维护单调 `seq` + 环形/落盘 journal；client 重连 `subscribe(sid, last_seq)` 回放；journal 溢出推 `resync_required` 触发全量重建。

**天花板**：`// NOTE: journal 默认保留最近 N 条，溢出触发 resync. 升级: 当 resync 频率超阈值时扩容或持久化.`

## 3. 反向 RPC 多控一歧义

**决策**（v1）：多 client 连同一 session 时，approval/question 的 `ApprovalRequired` 广播给所有持有者，**任意一个应答即生效**。

**天花板**：`// NOTE: 多控一时 approval 取首个应答，无冲突仲裁. 升级: 引入持有者锁或投票.`

## 4. 迁移路径要点

- **不留兼容 shim**：所有 `pub use old as new` 的过渡 re-export 一律不做（AGENTS.md 规则）。`git mv` + 全局 `use` 路径替换一次到位。
- **provider 迁层零摩擦**：已 grep 确认 `agent/provider/*.rs` 仅依赖 `crate::core::{traits,error,message,types}`，对 agent 内部零依赖。
- **rpc.rs 是 protocol 胚胎**：演进而非推倒。其 `RpcCommand`/`RpcEvent` 类型搬到 `protocol/`，rpc.rs 退化为 stdio transport + InProcessDriver 包装。
- **BDD 全绿是行为不变护栏**：P0~P4 任何阶段 BDD 失败 = 行为被破坏，立即停下排查，不调测试迁就实现。

## 5. 不做的事（YAGNI 边界）

- 不做 server 间分布式协调（每个 server 是独立核心，扩展 = 多起实例）。
- 不做 protocol 版本协商机制（v1 冻结现有 rpc 命令集，新命令随需求加）。
- 不做 web/tui client 实现（本次只立 Driver 抽象 + cli 落地，tui/web 留 feature flag 占位）。
- 不做 OAuth/provider 归因头（项目 1.0 前不支持，见 AGENTS.md Provider Scope）。

## 6. 核心概念词汇表（防概念漂移）

新概念引入必须先在本表登记 + 在模块地图补一句话职责，未登记的概念不得进代码。

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
