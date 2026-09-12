# 对照 Apache Maka Runtime Host：xylitol 内核差距调研（2026-09）

> **性质**：跨 change 主题级耐久底稿——对照 Apache Maka 的 Runtime Host / Event Log 内核，记录 xylitol 与 Maka 在**非 GUI/TUI 内核**上的差距、可学习的不变量，以及**若将来要对齐**必须先满足的前置。
> **不是**实现计划，**不是**「马上改 xylitol 内核」的提案，**不**改 MUST。产品心智以 `docs/architecture/` 为准；代码不变量以 `src/AGENTS.md` 为准。

## 1. 报告结论（先读）

**本报告不建议 xylitol 现在改成 Maka 式「全局唯一常驻 Runtime Host 进程 + 所有面恒为 thin client」模型。** 两者产品定调不同：Maka 是多 package、多面共享 State Root 的 ASF agent 平台；xylitol 是 Rust 单 crate、个人开箱 coding harness，Print/嵌入同进程是主线，Trust 与工具 permission 分道。

**可学习的是不变量与检查清单**（log-first、投影≠事实、Resume≠重试、Eval 不第二套 Runtime 等）。**很难直接转型**：多数 Maka 能力绑在独立 Event Log store、Hosted Execution 五 phase recovery、PermissionProfile + OS sandbox 等前置上；在 xylitol 未单独立项且前置未满足前，**任何代码级对齐均不应启动**。

---



## 2. 主对照表（九轴）


| 轴                             | xylitol（左）                                                                                                                                                                                         | Maka（右）                                                                                                                                                                                                      | 差距                                                                                                         | 前置（未满足则不转）                                                                                                                     |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **执行权威**                      | **host 角色**可同进程（Print/嵌入/`into_driver`）；产品 TUI **要求** attach 本机监听器；`serve` 才占端口。per-session **单写者** + `WriterLease`（`src/AGENTS.md`；`src/app/server/host.rs`）                                      | **唯一** Runtime Host 进程持 State Root 排他 lease；Desktop/TUI/CLI/Bot/Eval **均不**本地 compose Runtime（`../maka/ARCHITECTURE.zh-CN.md`；`../maka/docs/architecture/runtime-host-architecture.zh-CN.md`）                | **形态差距大、语义部分同构**：同 session 仅一个 writer；但 xylitol 允许多部署形态（同进程 vs attach），Maka 强制中心化 Host                     | 若要对齐 Host 中心化：**独立 change** 论证 Print/嵌入是否仍须同进程；State Root / lease 语义；**禁止**大爆炸拆进程。个人 harness 无此需求则**不转**                       |
| **事实源 / 投影**                  | **session JSONL**（`AgentMessage` + `CompactionEntry`）= 持久事实；`XyEvent` = live 用户可见投影；`project_for_llm` = 模型输入投影；Host `EventJournal` = WS 下行缓存，冷恢复**禁止** journal 回放（`docs/architecture/远程体验与线协议.md`） | **Runtime Event Log** = 唯一 canonical；UI / 模型历史 / Recovery / compaction 均为 **Project(events, policy)**（`../maka/docs/architecture/runtime-core-architecture-draft.zh-CN.md`）                                  | **存储形态不同、心智可近**：xylitol 已三分（磁盘 / 事件流 / LLM），但未形式化为「State=Project」；journal 与 JSONL 双轨需 operator 心智          | 若要对齐 log-first：**不变量文档化**即可，**无需** SQLite ledger 双写。代码级统一 Event 类型 = 需 session 格式 change + 迁移策略（Pre-0.0.1 禁止双路径）→ **前置未立项则不转** |
| **崩溃续跑**                      | 磁盘 **crash-atomic**（temp+rename）；`/session-resume` = 下次**加载**会话；**无** mid-run kill 后 repair / continuation / park（`docs/architecture/会话与持久化.md`）                                                   | **Resume≠重试**：repair 旧 Run 终态 → **RecoveryResolver** 解释工具 → host safety inspector → 安全则**新建 Run**；不确定 **park**（`../maka/docs/architecture/runtime-resume-architecture.zh-CN.md`）                             | **能力差距大**：xylitol 保文件完整，不保「工具副作用是否已知、能否自动续跑」                                                               | 工具 **T1/T2** 结算契约；terminal 事实写入 session；fail-closed park UX 产品决议。缺任一项 → **不做** Maka 式 auto-continuation                        |
| **Compaction**                | **追加** `CompactionEntry`（summary + `first_kept_entry_id`），不删前缀；`build_context_entries` 用摘要 + tail（`src/agent/compaction/mod.rs`；`docs/architecture/压缩与上下文.md`）                                     | Compaction = **有损投影**；canonical log 不改写；**HistoryCompactCheckpoint** 为 materialized view（`../maka/docs/architecture/llm-compaction-events-log-projection-draft.zh-CN.md`）                                    | **实现已部分同向**；差距在显式 checkpoint 表 / policy 指纹 / provider-native compact state                                 | architecture 写清「摘要非第二真相」= **文档即可**。独立 checkpoint store 或 V3 provider-native = **大 change** → 前置未满足则不转                          |
| **面与内核**                      | TUI/Print = **client**（键/画/TTY/剪贴板）；host 管模型/会话/MCP/工作区/trust；`XyDriver` 统一交互内核（`docs/architecture/库与多客户端.md`）                                                                                     | 全面 **thin client**；**Session Continuity** 供 snapshot + sequenced live updates；stream **不是** recovery authority（runtime-host 文）                                                                               | **角色已对齐**；Maka Continuity 协议更形式化（seq / bounded snapshot / Host Epoch）                                      | gpui 第二面开闸后复核跨面同源即可。**不必**为对齐引入 Host Epoch / Composition ID 全套                                                                 |
| **协议信封**                      | **四象限** RPC：POST unary + WS 下行；`server-request` 反向通道；gap → `session/resync_required`（`src/protocol/wire/envelope.rs`；`src/app/server/host.rs`）                                                     | Host protocol + **Client Capability** offers/bindings；local IPC 与 remote WS **同** State Root（runtime-host；`../maka/docs/runtime-host-remote-access.zh-CN.md`）                                                | **能力重叠**：反向调用、远程 attach 已有。**差距**：Maka operator 面（profile / credential / systemd service / peer transport） | 多 Host / 多机 State Root 共享成**真实需求**前，不引入 Maka 式 profile 栈。单用户 loopback attach → **不转**                                          |
| **工具生命周期**                    | ReAct 内批调度；`XyEvent` 工具起止；开箱 allow-all + 批并行策略（`docs/architecture/工具与权限.md`）                                                                                                                       | `ToolRuntime`：validate → permission → execute → classify → trace；Resume **Phase 2** T1/T2 ledger（`../maka/docs/archive/runtime-kernel.md`；runtime-resume 文）                                                  | **结算边界差距**：xylitol 无「派发已 durable / 结果未知 / 已 park」状态机                                                       | 见「崩溃续跑」前置；ToolRuntime 抽层 = ReAct 重构 change。**未立项则不转**                                                                          |
| **权限 / Trust**                | **Trust** 闸项目本地资源加载；工具 **allow-all**；可选 `GlobPolicy`；Trust ≠ 工具 popup（`docs/architecture/信任与项目门禁.md`）                                                                                               | **PermissionMode**（explore/ask/bypass）+ **ExecutionBoundary** 单调扩展 + **OS sandbox**（Seatbelt/bwrap/AppContainer）（`../maka/packages/core/src/permission.ts`；`../maka/packages/runtime/src/sandbox/README.md`） | **产品故意分道**：xylitol 不做默认工具审批与 sandbox 平台                                                                    | 产品定调变更（默认审批 / 多租户 sandbox）= **显式翻案**；否则 **永不为了对齐 Maka 而转**                                                                     |
| **Print / 嵌入 vs 第二套 Runtime** | Print / `embed::into_driver()` 同进程走 **同一** `XyDriver`；禁止调用点拥 `AgentRuntime`（`src/embed.rs`；`docs/architecture/库与多客户端.md`）                                                                          | 删 Headless；**Eval 不构造 Runtime**；Maka subject 仅经 Host client/protocol（`../maka/packages/eval/README.md`）                                                                                                      | **已对齐教训**：无第二套循环。差距在 Maka **强制**所有面经 Host，xylitol **允许** in-process                                        | 嵌入/Print 改强制 remote Host = **产品主线倒退** → **不建议、不转**                                                                             |


---



## 3. 可学习的不变量（心智 / 检查清单）

以下**可直接引用**于 xylitol 设计评审或 future change 背景，**不要求**改代码：


| #   | 不变量（检查句）                                   | Maka 来源                         | xylitol 今日近似                    |
| --- | ------------------------------------------ | ------------------------------- | ------------------------------- |
| 1   | 执行面**不**拥有第二套 agent 循环                     | `ARCHITECTURE.zh-CN.md`         | `XyDriver` 统一入口 ✅               |
| 2   | **State = Project(事实, policy)**；UI 文本不是事实源 | runtime-core 第一章                | JSONL + `XyEvent` 投影 ✅（未形式化）    |
| 3   | **Compaction 改投影不改历史**                     | compaction 第三章                  | 追加 `CompactionEntry` ✅          |
| 4   | **Resume ≠ 重试**；不复活旧进程/旧 socket            | runtime-resume 第八章              | 仅 load session；无 auto-resume ⚠️ |
| 5   | **缺 tool result ≠ 未执行**；不能证明则 park         | runtime-resume Phase 0–2        | 未建模 ⚠️                          |
| 6   | **Stream / journal ≠ recovery authority**  | runtime-host Session Continuity | 冷恢复快照 + resync ✅                |
| 7   | **Admission**：同 session 顶层 work 不并发        | Hosted Execution                | 单写者 + Busy 拒绝 ✅                 |
| 8   | **Eval / 嵌入不 compose Runtime**             | `packages/eval/README.md`       | `embed` → `into_driver` ✅       |
| 9   | **Trust（资源加载）与 tool permission（执行审批）分开**   | sandbox README Boundaries       | 产品 MUST 已分道 ✅                   |


**用法**：新 change 涉及持久化、远程、压缩、崩溃时，用上表做 **review checklist**，而非当作「应对齐 Maka 第 N 条」的工单。

---



## 4. 转型所需前置（与「可学习」拆开）

**直接转型 Maka 式内核不可行**，因下列前置彼此依赖；**全部未满足前，禁止启动任何「对齐 Maka」的代码迁移**。


| 前置域     | 具体条件                                                          | 若缺失则                                        |
| ------- | ------------------------------------------------------------- | ------------------------------------------- |
| **产品**  | 书面确认：Print/嵌入是否仍可同进程；是否接受全局常驻 Host；是否接受默认工具审批 / OS sandbox    | 不讨论 Host 中心化或 permission 对齐                 |
| **事实层** | session 格式 change 立项；单一 canonical 写入路径；禁止 JSONL + ledger 长期双写 | 不引入独立 Runtime Event Log store               |
| **续跑**  | 工具 T1/T2 或等价 terminal 契约；park UX；fail-closed 产品 MUST          | 不做 auto-continuation / RecoveryResolver     |
| **压缩**  | checkpoint 策略版本化需求被证伪或立项                                      | 不建独立 checkpoint 表 / provider-native compact |
| **协议**  | 多 Host、远程 operator、跨机 State Root 成真实场景                        | 不引入 Host profile / lease / Epoch 全套         |
| **组织**  | 单 change 原子边界 + Pre-0.0.1 无兼容债纪律                              | 禁止大爆炸 refactor                              |


**无「阶段 1→N 迁移路线图」**：前置满足一项，才允许**该项**独立 change 引用 Maka 对照；不满足则只保留第 3 节检查清单。

---



## 5. 差距性质汇总


| 差距性质           | 轴                                                | 说明                          |
| -------------- | ------------------------------------------------ | --------------------------- |
| **已同构（形态不同）**  | 执行权威、面与内核、Print/嵌入、Trust≠permission              | 不必为「像 Maka」改部署              |
| **心智可近、文档即可**  | 事实源/投影、compaction、journal≠recovery               | 可吸纳 = 写清 invariant，非改 store |
| **能力缺、需前置**    | 崩溃续跑、工具 T1/T2、Run 基线冻结                           | 代码对齐 **前置未满足则不转**           |
| **产品冲突、不建议对齐** | 全局常驻 Host、默认 sandbox/审批、多 package 边界、Agent Graph | 见第 6 节                      |


---



## 6. 明确不对齐项（即使用 Maka 作参照）


| Maka 做法                                     | 为何不转                                    |
| ------------------------------------------- | --------------------------------------- |
| 所有面必须连 Runtime Host 服务                      | xylitol Print/嵌入同进程是主线（`src/AGENTS.md`） |
| host = 必须占端口 / systemd 常驻                   | host ≠ 监听器是硬约束                          |
| 默认 PermissionMode + 工具审批 UI                 | 开箱 allow-all + Trust 闸资源                |
| OS 级 sandbox 平台                             | 个人 harness；可选 GlobPolicy 够用             |
| core/runtime/storage/runtime-host 多 package | Rust 单 crate 已分层                        |
| Agent Graph / Eval 第二套 Runtime              | 产品克制；embed 已禁止旁路                        |


---



## 7. 开放问题（可证伪，供 future change 背景）

1. mid-run kill 后，「加载会话 + 标中断、不自动续跑」是否满足个人用户？（证伪：重复手工重试同一 tool）
2. `CompactionEntry` 无 policy 指纹时，换模型/ summarizer 是否引发 overflow 回归？
3. T1/T2 最小粒度：session header vs per-tool-call 条目，崩溃在批内第二工具时何者够用？
4. 同进程 Print 与 attach TUI 并发写同 session 是否真实场景？（writer lease 日志）
5. 单用户 loopback 下，Maka 式 State Root lease 相对 xylitol `WriterLease` 是否过度？

---



## 8. 来源表


| 资料                       | 路径 / URL                                                                                                |
| ------------------------ | ------------------------------------------------------------------------------------------------------- |
| Maka 总览                  | `../maka/ARCHITECTURE.zh-CN.md`                                                                         |
| Log Is the Runtime       | `../maka/docs/architecture/runtime-core-architecture-draft.zh-CN.md`                                    |
| Runtime Host             | `../maka/docs/architecture/runtime-host-architecture.zh-CN.md`                                          |
| Resume                   | `../maka/docs/architecture/runtime-resume-architecture.zh-CN.md`                                        |
| Compaction               | `../maka/docs/architecture/llm-compaction-events-log-projection-draft.zh-CN.md`                         |
| 远程 Host                  | `../maka/docs/runtime-host-remote-access.zh-CN.md`                                                      |
| Runtime / Sandbox / Eval | `../maka/packages/runtime/README.md`；`src/sandbox/README.md`；`../maka/packages/eval/README.md`          |
| RFC one Host             | [https://github.com/apache/maka/issues/853](https://github.com/apache/maka/issues/853)                  |
| xylitol SSOT             | `src/AGENTS.md`                                                                                         |
| xylitol 产品架构             | `docs/architecture/库与多客户端.md`、`远程体验与线协议.md`、`会话与持久化.md`、`插话续跑与中止.md`、`工具与权限.md`、`信任与项目门禁.md`、`压缩与上下文.md` |
| xylitol 代码锚点             | `src/agent/compaction/mod.rs`；`src/app/server/host.rs`；`src/embed.rs`；`src/protocol/wire/envelope.rs`   |
