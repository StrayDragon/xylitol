# src/ 质量调优 TODO（临时协作文档）

> **性质**：临时交接 / 调优清单，**不是** AGENTS 规范，**不是** llmanspec 合约。
> **范围**：优先 `src/`；`packages/`（含 `xylitol-ai-bridge` / `xylitol-tui`）仅在本清单标明「后置」时再动。
> **流程**：默认 **不走 SDD**。仅当某项改动会破坏现有 live specs / BDD 场景的 MUST 行为时，才对该项切 `/llman-sdd-propose`。
> **用法**：完成一项就把对应 `- [ ]` 改成 `- [x]`，并在「进度日志」补一行（谁、何时、关联 commit/PR 可选）。
> **发现时间**：2026-07-21（代码质量探索会话）。

---

## 0. 协作者必读（防丢方向）

### 0.1 目标画像（验收北极星）

我们期望的代码质量：

| 维度 | 期望 | 当前粗判（2026-07-21） |
|---|---|---|
| SSOT / 少冗余 | 叶类型一份定义；业务用组合挂库类型 | 架构 SSOT 强；bridge↔domain 仍有同形孪生 |
| 全类型 | 少「受控 any」；边界外 JSON、边界内 struct/enum | 无 `dyn Any`；工具/钩子/`XyDriver` 仍多 `Value`/`String` |
| 现代 Rust | 2024 + RPITIT 等；少不必要宏/装箱 | Edition 2024 + let-else 已用；port 仍大量 `async_trait` |
| 可维护 | 模块可审阅；AGENTS 长期规则与易腐调音分离 | 纪律清晰；`react`/`driver`/`session` God 文件 |
| 性能 | 热路径克制 clone；端口 `Arc<dyn>` 合理 | 可用；未做针对性压榨（勿盲改） |
| 错误 / 观测 | 可匹配的 kind；anyhow 仅叶 source；trace 带 kind | `XyError` 有但 seam 少用；多 `to_string()` |

### 0.2 非目标（本清单不做）

- 不为「未来插件市场」加抽象。
- 不拆主 crate 为多 crate（编译产物膨胀；分层靠约定 + review）。
- 不在本轮改 `packages/xylitol-tui` 引擎/组件（除非 src 接线被挡）。
- 不把本文件内容抄进 AGENTS 正文（AGENTS 最多一行指针）。
- 不把行数进度表、commit 列表写进 AGENTS。

### 0.3 何时升级到 SDD

对每一项动手前自问：

1. 是否改变用户可见 / BDD 已钉死的 MUST 行为？
2. 是否改变 wire `protocol::Command`/`Event` 语义或序列化形状？
3. 是否改变 Trust / Permission / MCP 产品语义？

任一为 **是** → 该项走 SDD（propose → apply → verify），并在本文件该项下注明 change id。
纯内部类型抬升、拆文件、错误枚举替换且 `Display`/行为测仍绿 → **不走 SDD**，勾选推进即可。

### 0.4 推荐验证（每项合并前）

按改动面选择（不必每次满闸，但行为相关必须覆盖）：

- 相关 `cargo test` / `cargo test --test bdd -- --test-threads=1`（触及 agent/session/CLI 时）
- TUI 相关：`src/app/tui/harness.rs` / `tests.rs`；需要时 `just test-tui`
- 合并前习惯：`just qa`（满闸）
- 格式：`just fmt` + 相关 clippy

### 0.5 建议落地顺序

```text
A  AGENTS 调音规则 + RETUNE 指针文件     ← 立规矩，低风险
B  错误类型抬升（XyDriver / dispatch）     ← 横切收益最大
C  内置工具 Args 类型化                  ← 局部、类型可见
D  God 文件拆分（react / driver / session）
E  runtime_protocol RPITIT               ← 一批改 port+实现
F  孪生类型 SSOT（需动 packages）         ← 后置；先写归属、禁新增孪生
G  观测 kind 打尖                        ← 可与 B 并行或紧随
```

可并行：A ∥ 开写 F 的「归属表」段落；C 与 D 不同文件时可并行。
串行更稳：B 完成后再大面积改 XyDriver 调用方；E 最好独立 PR。

### 0.6 关键路径速查

| 主题 | 真值 / 入口 |
|---|---|
| 分层与 seam | `src/AGENTS.md` |
| TUI 面边界 | `src/app/tui/AGENTS.md` |
| Provider / 消息投影 | `src/AGENTS.md`「Provider 适配」；`src/infra/provider/map.rs`；`src/domain/llm_project.rs` |
| 工具 port | `src/runtime_protocol/tool.rs`；实现 `src/infra/tools/` |
| 钩子 | `src/runtime_protocol/hook.rs`；`src/agent/runtime/hooks.rs`；`src/infra/hooks/` |
| XyDriver seam | `src/app/core/driver.rs`；`src/app/core/dispatch.rs` |
| 领域错误 | `src/domain/error.rs` |
| bridge 包边界（后置） | `packages/xylitol-ai-bridge/AGENTS.md` |
| 根 AGENTS 临时文档约定 | 根 `AGENTS.md`「编写与维护 AGENTS.md」→ 临时交接用 `_TODO` / `_HANDOFF` |

---

## A. AGENTS「调音」：长期 rule vs 短期清单

### 背景

- 根 / `src` AGENTS 已规定：写稳定边界，不写易腐进度。
- `src/app/tui/AGENTS.md` 写了「单文件逼近 ~1200 行视为硬味」，但生产文件已远超（见 D），规范滞后。
- 需要像钢琴调音：周期性校准预算与超标清单，且 **清单不进 AGENTS 正文**。

### 要做

- [x] **A1** 在根或 `src/AGENTS.md`「维护习惯」增加一条**长期** rule（短）：
  - 模块体量 / 超标清单 / 调音日期 → 独立易腐文件；AGENTS 只保留一行指针。
  - 触发：合并后超硬顶，或约定周期（建议季度）。
- [x] **A2** 新增易腐文件（二选一命名，选定后改本 TODO 内链接）：
  - 推荐：`src/QUALITY_RETUNE.md`
  - 内容最少包含：软顶/硬顶行数、当前超标文件表、上次/下次调音、拆分目标指针（链到本 TODO 的 D）。
- [x] **A3** AGENTS **不要**粘贴超标表；只写：`体量调音见 src/QUALITY_RETUNE.md`。
- [x] **A4** 明确测试 harness（如 `harness.rs`）预算可另计，避免与生产模块同一硬顶。

### 验收

- 新人只读 AGENTS 能找到 RETUNE；改行数清单不必 PR 改 AGENTS 长文。

### 风险 / SDD

- 无行为合约变更 → **不走 SDD**。

---

## B. 共享应用协议进品牌（路线 β）+ 错误抬升

### 背景（事实）

- 产品主线是「装配 → XyDriver → 循环 → 事件流」；XyDriver **多面共享**，不是内部实现。
- 旧分档把 `Xy*` 只留给端口、把 XyDriver 留在「无前缀缝」，造成「没 Xy = 不对外」的误读。
- `XyDriver` trait 大量 `Result<T, String>`；`XyDriverError(pub String)` 无结构。
- domain 已有 `XyError` / `XyToolError`（热路径）；与整机遥控器失败语义不完全重合。

### 决议（2026-07-21 · 冻结）

**路线 β + 方案 3（一步到位，不接受「先 DriverError 再升格」中间态）：**

| 项 | 决定 |
|---|---|
| 命名 | `XyDriver` → **`XyDriver`**；错误 → **`XyDriverError`**；进程内实现 → **`XyInProcessDriver`**；远程实现 → **`XyRemoteDriver`** |
| 分层 | **`XyError`**：ReAct / provider / tool 热路径（可继续打尖）。**`XyDriverError`**：整机缝（NotFound / Unsupported / InvalidInput / Io / Remote / `Agent(XyError)` 等） |
| 关系 | `XyDriverError: From<XyError>`（及必要的 `From<XyToolError>` via XyError） |
| 导出 | `XyDriver` / `XyDriverError` / `XyInProcessDriver` 进 **`embed` + crate 根精选 `pub use`**（与 β「共享应用协议进品牌」一致） |
| `XyDriverError` | **删除**；dispatch 直接用 `XyDriverError` |
| `BootstrapError` | **本轮保留**（启动期）；不强制并入 |
| AGENTS / 产品文 | 更新：`Xy*` 含「共享应用协议（`XyDriver` + 事件）」；删「XyDriver 不加 Xy」旧表 |
| 兼容 shim | **不加** `type XyDriver = XyDriver` 之类别名（一次性改完调用点） |
| SDD | Display/行为等价 → **不走 SDD**；若 REST/BDD 钉死错误字符串再评估 |

三圈契约（防再混）：

```text
① 线协议     protocol::Command / Event
② 应用协议   XyDriver + XyEvent 流 + XyDriverError     ← β 收进品牌
③ 可替换口   XyModel / XyTool / XySessionStore / …
```

### 设计意向

```text
XyError              // 库/agent 热路径
XyDriverError        // 整机缝；含 Agent(XyError)
dispatch             // Result<_, XyDriverError>
anyhow               // 仅叶 #[source]
```

- UI 继续 `Display`；类型上可 `match` / `kind()`。
- 观测 `error.kind`（§G）紧随本项。

### 要做

- [x] **B1** 定案：路线 β + 方案 3（本决议）。
- [x] **B2** 新增 `XyDriverError`（+ `kind()`）；按需打尖 `XyError`；单测。
- [x] **B3** 重命名 `Driver`→`XyDriver` 等；签名 `Result<T, XyDriverError>`；改 `XyInProcessDriver` / `XyRemoteDriver` / stub / harness。
- [x] **B4** `dispatch` 删除原 `DispatchError` String newtype；TUI effects / server / cli 调用方。
- [x] **B5** `composition::build_agent` 等能抬则抬到 `XyDriverError` 或保留 `BootstrapError` 边界清晰。
- [x] **B6** 更新 `src/AGENTS.md` / 根与 tui AGENTS / `embed` / `lib.rs` / 产品文「库与多客户端」命名表；禁止新的 seam `Result<_, String>`。
- [x] **B7** `just fmt` + 相关测 / `just qa` 触及面绿。

### 验收

- 源码无旧 trait 名 `XyDriver`（文档历史/archive 除外）；公开 seam 无 `String` 错误。
- embed / crate 根可 `use xylitol::{XyDriver, XyDriverError, …}`。
- BDD / harness / qa 绿；用户可见文案不无故变差。

---

## C. 工具 / 钩子：边界 JSON，内部全类型

### 背景

- `XyTool::execute(..., args: Value)` + `parameters_schema() -> Value`（`runtime_protocol/tool.rs`）——MCP/动态工具需要。
- 内置工具（`infra/tools/{read,write,edit,bash,grep,find,ls}.rs`）用手写 `args["path"].as_str()` 解析。
- 脚本 hook 合约是 JSON stdin/stdout → **必须**保留 `Value`（`XyHookBus`）。
- `infra::hooks::HookEvent` 已部分 typed；`AgentHooks` before/after 仍是 `Value`。

### 原则（冻结）

```text
跨进程 / MCP / 脚本 hook  = Value（或 JSON map）
crate 内热路径 / 内置工具 = struct + serde
```

**不要**为了「全类型」改掉 MCP 口或脚本 hook 的 JSON 协议。

### 要做 — 工具

- [x] **C1** 为每个内置工具增加 `XxxArgs`（`Deserialize`），入口 `serde_json::from_value` → `XyToolError::InvalidArgs`。
  - 顺序完成：`read` → `grep` → `find`/`ls` → `write`/`edit` → `bash`；共享 `infra/tools/args.rs::parse_tool_args`。
- [x] **C2** schema 与 Args 同构策略二选一（写入决议）：
  - **选定 (a)**：手写 schema 与 Args 字段并置（LLM schema 形状稳定）；Args 用 `serde`/`rename_all = "camelCase"` 对齐。
- [ ] **C3**（可选后置）`TypedTool` associated type + blanket 擦成 `dyn XyTool`——仅当 C1 完成后仍痛再做。

### 要做 — 钩子

- [x] **C4** 保持 `XyHookBus::dispatch(..., Value)`；调用方用 typed 组装再 `to_value`。（确认：本轮不改）
- [ ] **C5** 继续扩展 `HookEvent` 变体，减少 dispatcher 内临时 `json!` 散落。
- [x] **C6** `AgentHooks`：评估 before/after 是否改为更窄类型；**刻意保留 `Value`**（嵌入回调少；与脚本 JSON 同形）。

### 决议（填写）

- Schema 策略：**`(a)` 手写 schema + typed Args**
- `AgentHooks` 是否改签名：**否（刻意保留 Value）**
- 模块放置：`args.rs` **暂留** `infra/tools/` 顶层（与 `path_utils`/`truncate` 同档）。日后若整理共享模块，倾向整批迁入 `infra/tools/support/`（勿单独塞进泛称 `utils/`）。

### 验收

- 内置工具执行路径不再直接 `args["…"]` 散落（集中在 `from_value` 一层）。
- MCP / 脚本 hook 行为不变。

### 风险 / SDD

- 仅解析路径重构 → **不走 SDD**。
- 若改变工具参数名/默认值/错误文案且有场景钉死 → **走 SDD** 或改测。

---

## D. God 文件拆分（可维护性）

### 背景（约行数，2026-07-21 探测，易腐——权威表以 A2 的 RETUNE 为准）

| 文件 | 约行数 | 备注 |
|---|---|---|
| `src/agent/runtime/react.rs` | ~2762 | ReAct 核心 |
| `src/app/core/driver.rs` | ~2384 | trait + in-process + remote |
| `src/infra/session/manager.rs` | ~2191 | session 持久化与树 |
| `src/app/tui/harness.rs` | ~4135 | 测试；预算可另计 |

`src/app/tui/AGENTS.md` 已禁止继续堆 God 文件；需拆现有存量。

### 建议切面（实现时可按 PR 切开）

**react.rs**

- [ ] **D1** `loop` / 步进与 turn 边界
- [ ] **D2** tool 批处理 / permission / streaming update
- [ ] **D3** model call + retry（已有 helper 可下沉）
- [ ] **D4** 加深已有 `obs.rs`，避免 loop 内嵌观测细节

**driver.rs**

- [ ] **D5** `trait XyDriver` + 报告/DTO 类型 → 独立模块
- [ ] **D6** `XyInProcessDriver`
- [ ] **D7** `XyRemoteDriver`

**session/manager.rs**

- [ ] **D8** persist / load
- [ ] **D9** tree / travel / list
- [ ] **D10** mutate（rename/delete/switch…）

### 约束

- 拆分 **行为不变**；优先 `pub(crate)` 边界清晰。
- 每个子 PR 保持可审阅（避免一次 2k 行大搬家无结构）。
- 拆完更新 `QUALITY_RETUNE.md` 超标表。

### 验收

- 生产模块回到软顶附近（RETUNE 定义的数）；`just qa` 绿。

### 风险 / SDD

- 纯搬家 → **不走 SDD**。

---

## E. 现代 Rust：`async_trait` → RPITIT（限 src）

### 背景

- `src` + 实现侧约有大量 `#[async_trait]`（探测约百级属性；以 `rg '#\[async_trait\]' src` 为准）。
- Port 集中在 `src/runtime_protocol/`；实现在 `infra` / `agent` / 测试 stub。
- Edition 已是 2024；QA 足够则适合整批，但应 **独立 PR**，避免与 B/D 缠在一起难回滚。

### 要做

- [ ] **E1** 盘点：列出 `runtime_protocol` 中所有 async 方法的 trait。
- [ ] **E2** 改为 `-> impl Future<Output = …> + Send`（注意生命周期：`+ Send` / 必要时显式）。
- [ ] **E3** 更新全部实现与 mock/stub。
- [ ] **E4** 确认 `dyn Trait` 对象安全：若某处需要 `dyn XyTool`，RPITIT 在 dyn 上有限制——**先核实再改**；不能 dyn 的保持 `async_trait` 或改设计（文档记入决议）。

### 决议（填写）

- 可 `dyn` 的 port 名单与策略：_（待填；可能部分保留 async_trait）_

### 验收

- 编译 + 相关测绿；无无故新增 `Box<dyn Future>` 热路径回归（对比前后需有意识）。

### 风险 / SDD

- 公开库 port 形变：若视为嵌入 API 破坏 → 评估是否记入 changelog；行为不变则 **通常不走 SDD**。

---

## F. 孪生类型 SSOT（组合；packages 后置）

### 背景

- `src/infra/provider/map.rs` 对 `TokenProvenance` / `ContextTokenEstimate` / `Diagnostic` / Usage / Part / StopReason / Message 做近 1:1 `From` 映射。
- 同形双份定义在 `src/domain` 与 `packages/xylitol-ai-bridge/src/dto/`。
- **产品约束**：不要把 xylitol **业务**字段塞进 bridge；用 **组合**。

### 归属表（冻结意向）

| 类型 | 归属 | 说明 |
|---|---|---|
| `TokenProvenance` / `ContextTokenEstimate` | 库（bridge） | 计量来源 |
| `Diagnostic` | 库 | provider 诊断 |
| StopReason / Usage / UsageCost | 库 | LLM 通用 |
| Part（Text/Image/Thinking/ToolCall） | 库 | LLM 可见 |
| `LlmMessage`（user/assistant/toolResult） | 库 | 仅 LLM 角色 |
| `EnvMessage`（bash/compact/branch/custom…） | **业务 domain** | 不进模型 |
| `AgentMessage = Llm \| Env` | **业务 domain** | 组合：`Llm(库类型)` |
| `AgentState` / `AgentContext` / session / trust / queue | **业务** | 与 bridge 无关 |

目标形态：

```text
bridge::dto::{LlmMessage, Part, Usage, …}
domain::AgentMessage::Llm(bridge::LlmMessage) | Env(EnvMessage)
map.rs → 变薄：project_for_llm + 少量边界转换
```

关键抉择：SSOT 落地需要 `domain` **依赖** `xylitol-ai-bridge`（仅 `dto`，禁止 `provider::*`）。与「domain 零 crate **内层**依赖」不冲突，但是架构一步，需在动手前确认。

### 要做（分阶段）

**现阶段（仅 src，不动 packages）**

- [ ] **F1** 把「归属表」写入 `src/AGENTS.md`「Provider 适配」（长期 rule；可压缩成表 + 指针）。
- [ ] **F2** 约定：禁止在 domain 再新增与 bridge 平行的叶类型；新叶先问归属。
- [ ] **F3** 在 `map.rs` 顶部注释标出「待删除的手写字段映射清单」（为后置搬迁做索引）。

**后置（允许改 packages 时）**

- [ ] **F4** 叶类型 SSOT 迁入 bridge；domain 改为组合。
- [ ] **F5** 删冗余 `From` 字段表；测与 provider 路径回归。
- [ ] **F6** 更新 `packages/xylitol-ai-bridge/AGENTS.md` 与 `src/AGENTS.md` 中过时「MUST NOT 被 domain 内嵌」表述（改为「Env 不得进入 bridge DTO」）。

### 决议（填写）

- 是否接受 `domain → xylitol-ai-bridge(dto)`：_（待填 yes/no）_
- 若 no，备选（共享极薄 types crate / 保持孪生+生成）：_（待填）_

### 验收

- 叶类型一份定义；`AgentMessage` 仍为 session 真源；Env 永不出现在 bridge DTO。

### 风险 / SDD

- F1–F3 文档/纪律 → **不走 SDD**。
- F4+ 若序列化形状 / session 文件格式变化 → **必须**评估兼容与 SDD/迁移。

---

## G. 错误与观测更尖

### 背景

- 观测栈已收敛：仅 **fastrace** + **log**（禁止 tracing 双栈）。见 `src/AGENTS.md`。
- 错误路径常 `map_err(|e| e.to_string())`，trace/log 缺少稳定 `error.kind`。
- `XyError` 大口袋仍包 `anyhow::Error`——叶可以，对外匹配要靠更细变体（与 B 联动）。

### 要做

- [ ] **G1** 为错误类型提供稳定 `kind()` / `as_str()`（风格对齐 `TokenProvenance::as_str`）。
- [ ] **G2** 关键失败路径：fastrace / log 带 `error.kind=…`（及已有 turn/request id 时一并带上）。
- [ ] **G3** 禁止新的 seam `map_err(|e| e.to_string())`（clippy lint 或 code review 清单；可选 `#[deny]` 难做则靠 RETUNE/review）。
- [ ] **G4** 文档：排障仍走 skill `xylitol-inspect-runtime-logs`，本 TODO 不复制长 how-to。

### 验收

- 抽几条失败路径能在 trace/log 用 kind 过滤；与 B 的类型抬升一致。

### 风险 / SDD

- 观测字段增强通常 **不走 SDD**（除非合约钉死 log 文案）。

---

## 进度日志

| 日期 | 作者 | 项 | 说明 |
|---|---|---|---|
| 2026-07-21 | （探索会话） | — | 创建本文件；尚未开工 |
| 2026-07-21 | agent | §A | 落地 `QUALITY_RETUNE.md`；根/`src`/tui AGENTS 指针；A1–A4 勾选 |
| 2026-07-21 | agent | §B | 决议：路线 β + 方案 3（`XyDriver` / `XyDriverError`）；开工实现 |
| 2026-07-21 | agent | §B | 落地重命名 + `XyDriverError` + 导出/AGENTS；dispatch/harness/driver 测绿 |
| 2026-07-21 | agent | §B | commit `57efe1a7` |
| 2026-07-21 | agent | §C | 内置工具 `*Args` + `parse_tool_args`；schema 策略 (a)；hooks 签名刻意保留 |
|  |  |  |  |

---

## 附录：探索时的量化快照（易腐，仅供对照）

> 数字会变；以 RETUNE + `wc`/`rg` 现场为准。

- `serde_json::Value` 在 `src/` 多处出现；hooks / react / driver 为热点。
- 无 `dyn Any` / `as_any` / `TypeId` 逃逸（加分）。
- `Result<…, String>` 在 `driver.rs` / `dispatch` / `session` 等密集。
- `async_trait` 属性约百级（全仓探测；实施 E 前重跑 `rg`）。

---

## 附录：与「不走 SDD」的协作约定（给后人）

1. 开 PR 标题建议前缀：`refactor(src): …` / `fix(agent): …`（Conventional Commits）。
2. PR 描述链到本文件章节（如 `src/_TODO.md` §B）。
3. 勾选与进度日志 **同一 PR 或紧随 PR** 更新，避免文档漂。
4. 若某项中途发现破坏 spec → **停手**，开 SDD change，并在该项下标注 change id 与「已升级 SDD」。
5. 本文件全部勾选且稳定一段时间后：可删或缩成 RETUNE 历史一段；**不要**升格为永久百科。
