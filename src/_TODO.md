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
| 现代 Rust | 2024 + RPITIT 等；少不必要宏/装箱 | Edition 2024；port **因 dyn 全覆盖**默认保留 `async_trait`（§E α） |
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
   └─ driver D5–D7 已完成
   └─ react / session 大拆：默认不做（见 §D 决议）；D11 已分诊待确认
E  runtime_protocol RPITIT               ← **已关闭（α）**：dyn 全覆盖，维持 async_trait
F  孪生类型与 domain↔packages 边界     ← **c1210**：compose bridge DTO；消息孪生已删
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
| XyDriver seam | `src/app/core/driver/`；`src/app/core/dispatch.rs`；心智见 `docs/architecture/库与多客户端.md` |
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
| `src/agent/runtime/react.rs` | ~2762 | ReAct 核心（其中 `#[cfg(test)]` ~1500） |
| `src/app/core/driver/` | 拆后见 RETUNE | trait / in-process / remote / types |
| `src/infra/session/manager.rs` | ~2191 | session 持久化与树 |
| `src/app/tui/harness.rs` | ~4135 | 测试；预算可另计 |

`src/app/tui/AGENTS.md` 已禁止继续堆 God 文件；需拆现有存量。

### 决议：XyDriver 命名心智（2026-07-21）

- **代码不改名**（保持 `XyDriver` / `XyInProcessDriver` / `XyRemoteDriver`）。
- **产品心智**：多 client 共用的统一交互内核（整机遥控器）；已写入 `docs/architecture/库与多客户端.md`。
- **`XyAppCore`**：若将来出现，更宜指整个 `app/core`（装配 + Driver + dispatch），**不要**只把 trait 改名成 Core，以免与目录边界糊在一起。
- 产品面（TUI / Server 等）**不必**现在品牌化为 `XyTuiApp` / `XyWebAppServer`。

### 建议切面（实现时可按 PR 切开）

**react.rs**（硬约束：`run_react_loop` 的 `async_stream` 宏块是原子单元，跨函数 `yield` 不可行）

体量切面（记账用）：

| 区段 | 约行 | 备注 |
|---|---|---|
| prelude + 小 helper | ~100 | streaming tool upsert 等 |
| `AgentRuntime` facade | ~380 | 可整文件搬走（D0b） |
| `run_react_loop` + 尾部 helper | ~780 | 真·剧本；拆法受限 |
| `#[cfg(test)]` | ~1500 | 几乎全是经 `AgentRuntime` + Mock 的**循环行为测**，不是可抽 helper 的单元测 |

原 D1–D4 在「可 yield 切片」意义上**不可直接执行**；曾改写为软项（见下）。

#### 决议：react 拆分 / 外置测试——低优先级（2026-07-21）

**事实**

- 同文件测试约 19 个 `async` 用例，关键词命中：`XyEvent`/`Mock`/`AgentRuntime`/`abort`/`steer`/`follow_up`；**零**直接测 `run_react_loop` / `call_with_retry` / `partial_assistant_*`。
- 即：这不是「一堆游离单测堵在生产文件里」，而是 **ReAct 行为回归套件与剧本同居**——外置只换文件边界，不增加可测性、不降低 loop 认知复杂度。
- 真难读的是 ~780 行 `async_stream`；外置 ~1500 行测试只让 `wc -l` / RETUNE 好看，对改 loop 的人帮助有限（仍要跨文件对照事件序）。
- P3 状态机已评估并冻结（见下）；软 D1–D4 亦不能绕过 yield 原子性。

**结论**

- **不必为「拆单测」而拆。** D0（外置 tests）标为 **won't / 除非编辑器或审阅痛到受不了**。
- **不必为行数而拆 `AgentRuntime`（D0b）或软 D1–D4**，除非某次改 loop 时自然顺手、且行为测全绿。
- RETUNE：`react.rs` 可视为「生产剧本 + 同居行为测」；超硬顶保留，但 **不要** 用「先拆测试」当伪进度。
- 真正值得再开闸的仍是：**功能逼出 P3**，或 loop 生产段本身再显著膨胀。

勾选状态（冻结意向，非待办压力）：

- [ ] **D0** 外置 tests — **低优先级 / 默认不做**
- [ ] **D0b** `AgentRuntime` 分文件 — **低优先级 / 默认不做**
- [ ] **D1–D4**（软）— **低优先级 / 有改动时顺手，不单开 PR**
- [x] **P3 评估** — 已评估，先不改（正文见下）

#### P3 评估：sub-turn state machine（2026-07-21，**先不改**）

**动机**：文件头已写「升级路径 = sub-turn state machine」。只有状态机（或等价：步进返回 `Vec<XyEvent>` + 下一状态）才能把「会 yield 的阶段」拆出单一宏块，而不靠 `include!` 假拆。

**现状剧本里与状态机强相关的交织点**（均在 ~780 行 loop 内）：

1. `outer`（follow-up 续跑）× inner（tools 后续轮）× 单 turn 生命周期事件序
2. 模型 `connect/retry` 与 mid-stream chunk 上的 `tokio::select!` + cancel（c680：drop stream 关 HTTP）
3. 流式 `MessageUpdate` / tool_call 累积与 `partial_assistant_message`
4. tool 批：permission → 串/并执行 → live output `try_recv` 上行 → hooks 看到的 preview vs Image parts
5. turn 后 poll steer；将停时 drain follow-up 决定是否再进 `outer`

**一种可行形状（评估用，非设计定稿）**：

```text
enum TurnPhase { InjectPending, ModelConnect, StreamChunks, AssistEnd, ToolBatch, PollSteer, FollowUpOrSettle }
step(phase, ctx) -> (next, Vec<XyEvent>)   // 或 mpsc，由薄 async_stream 只负责 yield
```

**收益**

- 阶段可单测（不必跑完整 stream）
- 主文件可长期压在软顶内且结构清晰
- 新能力（新 turn 钩子 / 新 abort 语义）有明确插入点

**成本 / 风险**

- ~780 行行为保持重写；`XyEvent` **顺序**被 TUI / BDD 钉死，回归面大
- cancel 竞态、并行 tool + uplink、hook 短路，在「纯步进」里都要重新表达，易静默改语义
- 若事件序或 abort/steer 可见行为有任何漂移 → 可能升级 SDD；即便号称纯内部，也需满闸 `qa` + 相关 BDD
- 对「行数超标」的 ROI 低于「什么都不做并承认同居测」；也**不高于**瞎外置测试

**结论（冻结）**

- **现在不做 P3。**
- 触发再议：① loop **生产段**持续膨胀且审阅痛；或 ② 某功能无法在单一 `async_stream` 剧本里干净落地、且需要按 phase 单测。
- **不要**把 D0/D0b 当 P3 的前置伪进度。

**driver/**（原 `driver.rs`）

- [x] **D5** `trait XyDriver` + 报告/DTO 类型 → `driver/{proto,types}.rs`
- [x] **D6** `XyInProcessDriver` → `driver/in_process.rs`
- [x] **D7** `XyRemoteDriver` → `driver/remote.rs`（顺手修 `--all-features` 下 `XyDriverError` 映射遗漏）

**session/manager.rs**

体量（2026-07-21）：总 ~2191；其中同文件 `#[cfg(test)]` ~538；**生产 ~1653**（仍超软顶、低于若去掉测试后的「硬顶错觉」——连测则超硬顶）。目录内已有 `cwd.rs` / `types.rs`；`tests.rs` **未挂进 `mod.rs`**，且引用不存在的 `SnapshotManager` 等——**孤儿死文件**，与 D8–D10 无关。

现成注释切面（同一 `impl SessionManager`）：Leaf / CRUD(~450) / Tree nav(~285) / Change tracking / Branch summary / Fork(~175) / Tree ops / Label / CWD / `XySessionStore`(~149)。文件末尾另有 **`XyEventSink for EventBus`**——与 session 无关，属错置。

#### 决议：D8–D10 是否必要（2026-07-21）

**与 driver 拆分的差别**

| | `driver`（已做） | `session/manager` |
|---|---|---|
| 切分轴 | trait + 两套实现（in-process / remote）+ feature | **同一类型**上的方法清单 |
| 编译/feature 收益 | remote 可闸 | 几乎无 |
| 认知收益 | 读协议 vs 读实现分离 | 多文件仍要在脑中拼回一个 `SessionManager` |

**结论**

- **不值得为 RETUNE 行数单开 D8–D10 搬家 PR。** 方法本就按域分了注释区；再切成 `persist.rs` / `tree.rs` / `mutate.rs` 只是 `impl SessionManager` 分散，审阅收益有限，还增加跳转成本。
- **值得记的卫生**已升格为 **D11 分诊**（见下）：D11a 错置 ≠ D11b 真死；**禁止**「接线救活」孤儿 snapshot 测。
- **再开闸条件**（D8–D10）：某域（如 fork / tree travel / deferred persist）要**独立演进或独立测**、且改动频繁撞 CRUD 大段时，再按**那一域**抽 `pub(crate)` 协作模块——而不是一次性按 D8/D9/D10 三切。

勾选：

- [ ] **D8** persist / load — **默认不做**（见上再开闸条件）
- [ ] **D9** tree / travel / list — **默认不做**
- [ ] **D10** mutate — **默认不做**
- [ ] **D11** — **已拆分为 D11a / D11b，见下「D11 分诊」；勿当作单一必做项**

#### D11 分诊（2026-07-21，严谨；**先不改代码**）

先前把「挪 EventBus impl」与「孤儿 tests.rs」捆成一个「可选卫生」，过于粗糙。按 `audit-dead-code` 三类分开裁决。

##### D11a — `XyEventSink for EventBus` 落在 `session/manager.rs`

| 项 | 事实 |
|---|---|
| 代码 | 文件末 ~8 行：`emit` → `self.emit_lifecycle(event)` |
| 归属 | `EventBus` 定义在 `infra/event/mod.rs`；port 在 `runtime_protocol::XyEventSink` |
| 存活 | **活的**：`composition` / BDD / `react` 测等大量 `Arc::new(EventBus::new()) as Arc<dyn XyEventSink>` 依赖此 impl（同 crate 内任一模块提供即可） |
| 来历 | c295（`ec7b0c32`）把 `EventSink` 重命名为 `XyEventSink` 时留在 manager；更早则是洋葱重构期「端口 impl 就近堆」的惯性，**不是** session 领域逻辑 |
| `manager` 耦合 | `use XyEvent` / `XyEventSink` 在 manager 中**仅服务该 impl**；与 `XySessionStore for SessionManager` 无关 |
| 分层味道 | session 模块为 **event 模块的类型** 实现 port → 读 session 文件会撞上无关适配器；`event` 已 re-export `XyEvent`，具备承接条件 |
| 若挪到 `infra/event` | 纯搬家；行为不变；可顺手从 manager 去掉仅为此存在的 import；**不走 SDD** |
| 若不挪 | 功能零损失；仅审阅噪音 |

**裁决（冻结）**

- **不是死代码，不是功能债，是低成本错置。**
- **默认不单开 PR。** 触发再做：下次改 `infra/event` 或动到 manager 尾部时顺手挪；或与其它 event 整理捆一小 commit。
- **不要**把它当 D8–D10 的「拆文件进度」；挪完 manager 只少约 8 行，不改变超标结论。

##### D11b — `src/infra/session/tests.rs`（296 行）

| 项 | 事实 |
|---|---|
| 模块树 | **未**出现在现行 `session/mod.rs`；`cargo` **从不编译**该文件 |
| 内容 | 测 `SnapshotManager` / `SnapshotBuilder` / `ContextFilter` / spawn / prune 等 |
| 符号现状 | 上述类型在全仓 **仅出现在此文件**（外加 `_TODO` 提及）；`session::{config,gc,storage}` 已删 |
| 来历 | `a0ec28bc` 引入 snapshot 系并 `#[cfg(test)] mod tests`；`781d5e8b`（c05 pi 对齐重建）改掉 `mod.rs` **漏删文件**；`8f9ec6b0` 再删 config/gc/storage 后文件仍在盘上 |
| 与现行 `SessionManager` | **无关**——现行是 JSONL `SessionManager` + manager 内联测；此文件测的是已废弃 snapshot/CoW 产品面 |
| 激活代价 | 等于复活整套 Snapshot API + 产品语义；与当前会话真值冲突；**不在本调优范围** |
| llmanspec | 无仍有效的 snapshot manager 合约依赖此文件 |

**按 audit-dead-code 归类：真死（编译期零引用）。默认处置：删。无例外。**

**裁决（冻结）**

- **删除是正确且安全的**（不改任何可执行行为、不碰 BDD）。
- **仍建议单独确认后再删**（本会话先分析）：删除是「清考古残骸」，不是「session 重构」。可与 D11a 解耦——**删 tests.rs 不必等 EventBus 搬家**。
- **禁止**为「救活」此文件去接线或改 `mod tests`；那是开新功能，不是卫生。
- **不走 SDD**（无运行时行为）。

##### 捆绑建议

| 做法 | 评价 |
|---|---|
| D11a + D11b 一个 commit | 可，但信息混杂（错置 vs 真死） |
| **只删 D11b** | 最高 ROI、零行为风险；推荐若只做一件 |
| 只做 D11a | 收益小，宜顺手 |
| 都不做 | 可接受；在 RETUNE/TODO 保留结论即可，避免遗忘再误判为「预留」 |

勾选（分析完成；实施另议）：

- [x] **D11 分诊**（本小节）
- [x] **D11a** 挪 `XyEventSink for EventBus` → `infra/event`（2026-07-21）
- [x] **D11b** 删除孤儿 `infra/session/tests.rs`（2026-07-21）

### 约束

- 拆分 **行为不变**；优先 `pub(crate)` 边界清晰。
- 每个子 PR 保持可审阅（避免一次 2k 行大搬家无结构）。
- 拆完更新 `QUALITY_RETUNE.md` 超标表。
- `react`：**禁止**为拆而 `include!` / 宏切片假拆；P3 未开闸前不要把 yield 阶段硬拆成多函数。

### 验收

- 生产模块回到软顶附近（RETUNE 定义的数）；`just qa` 绿。

### 风险 / SDD

- 纯搬家 → **不走 SDD**。
- P3 若改变事件序 / abort / steer 可见语义 → **走 SDD**。

---

## E. 现代 Rust：`async_trait` → RPITIT（限 src）

### 背景

- `src` 内 `#[async_trait]` **约 37 处**（2026-07-21 `rg`；早先「约百级」偏高，以现场为准）。
- Port 集中在 `src/runtime_protocol/`；实现在 `infra` / `agent` / 测试 stub；另有 `XyDriver`（`app/core/driver`）亦 `dyn` + `async_trait`。
- Edition 2024 + rustc 1.95：AFIT / RPITIT 可用，但 **带 `async fn` / `-> impl Future` 的 trait 默认仍非 dyn-compatible**。

### 要做

- [x] **E1** 盘点：列出 `runtime_protocol` 中所有 async 方法的 trait（见下表）。
- [ ] **E2** 改为 `-> impl Future<Output = …> + Send` — **默认不做**（见决议；与 dyn 冲突）。
- [ ] **E3** 更新全部实现与 mock/stub — 随 E2；未开闸。
- [x] **E4** 确认 `dyn Trait`：七个 async port **全部**以 `Arc<dyn …>` / `&dyn …` 注入（见下）；朴素 RPITIT **不可行**。

### E1 盘点（2026-07-21）

| Trait | async 方法（摘要） | `dyn` 使用（约） | 结论 |
|---|---|---|---|
| `XyModel` | `generate_stream` | ~26 / 10 files | **必须**保持 dyn 友好 |
| `XySessionStore` | exists/load/append/create/fork/… | ~48 / 14 files | 同上（最重） |
| `XyEventSink` | `emit` | ~20 / 8 files | 同上 |
| `XyTool` | `execute` / `execute_as_parts` | ~15 / 5 files（ToolSet） | 同上 |
| `XyHookBus` | `dispatch` | ~12 / 5 files | 同上 |
| `XyBashExecutor` | `execute` | ~11 / 6 files | 同上 |
| `XyExportIo` | `write_text` / `read_bytes` | ~11 / 6 files | 同上 |

同步 port（本项无关）：`XyPermission` / `XySecretResolver` / `XyTrustStore` / `XyResourceLoader` / `XyReloadable` — 无 `async_trait`。

`runtime_protocol` 内 **不存在**「有 async、却从不用 dyn」的 port → 没有「只改这一处 RPITIT」的甜区。

### 决议（2026-07-21，E4 驱动）

朴素 E2（trait 上 `async fn` / `-> impl Future + Send`）会使上述 trait **不能**再写 `dyn XyModel` 等，与现行组合根 / ReAct / ToolSet 架构冲突。

可选升级路径（**均未开闸**）：

| 方案 | 做法 | 利 | 弊 |
|---|---|---|---|
| **α 维持** | 保留 `async_trait` 于全部 dyn async port | 零风险；宏已承担 `Pin<Box<dyn Future>>` | 依赖宏；「现代感」不足 |
| **β 手写 box** | trait 方法改为显式 `-> Pin<Box<dyn Future<Output=…> + Send + '_>>`，去宏 | dyn 安全；无 async_trait 依赖 | 签名吵；实现处处 `Box::pin`；**热路径 box 次数与今日宏路径同级**，无性能胜负 |
| **γ 消 dyn** | 改成泛型 / enum 分发，去掉 `Arc<dyn Port>` | 才真正吃到 AFIT/RPITIT | 等于重写装配与 ToolSet；远超本项 |
| **δ 双轨** | 静态泛型 trait + dyn 擦除包装 | 理论完美 | 双倍 API；不值得为 7 个 port |

**冻结默认：α。** §E 的「整批 RPITIT」目标关闭，除非将来显式选 β/γ。

- 嵌入 API：今日已是 `async_trait` 形变的 `dyn` port；α 不破坏。若选 β，视为公开签名风格变更，评估 changelog，**通常仍不走 SDD**（行为不变）。
- `XyDriver`：同属 dyn async，**同样适用 α**；不要单独拿 Driver 做 RPITIT 试点。

### 验收

- E1/E4 文档结论进本文件即本阶段验收。
- 若开 β：编译 + 相关测绿；确认未额外叠一层无意义 box。

### 风险 / SDD

- α：**不改代码**。
- β/γ：行为不变则通常不走 SDD；γ 触及架构时另议。

---

## F. 孪生类型与 domain↔packages 边界

### 背景

- 曾有 `domain::LlmMessage` ∥ `AiBridgeMessage` 近 1:1 孪生 + `infra/provider/map.rs` 叶映射。
- **c1210** 解冻：LLM 叶 SSOT 迁 bridge；domain **组合**之；消息孪生表删除。

### 边界心智（c1210 后）

| 侧 | 职责 | 开闭 |
|---|---|---|
| **`src/domain`** | session 真源：`AgentMessage = Llm(AiBridgeMessage) \| Env`；Env / session / trust | MAY 依赖 bridge **DTO only**；禁 HTTP/SDK |
| **`packages/xylitol-ai-bridge`** | LLM 叶 SSOT + 方言 adapter / 计量 | 对厂商开闭；不依赖主仓 |
| **`infra/provider/map`** | chunk / tool-schema / provenance 等真边界缝；**消息 = identity** | 薄 |

```text
domain::AgentMessage::Llm(AiBridgeMessage) | Env(EnvMessage)
        │  project_for_llm（passthrough Llm；fold Env）
        ▼
Vec<AiBridgeMessage> → dialect adapters
```

### 归属表

| 类型 | 归属 | 说明 |
|---|---|---|
| `AiBridgeMessage` / Part / StopReason / Usage | **bridge**（domain `pub use` 别名） | LLM 叶 SSOT |
| `EnvMessage` / `AgentMessage` | **domain** | 组合；Env 不进 bridge |
| session / trust / queue / AgentState | **domain** | 与 bridge 无关 |
| chunk / provenance 若仍双份 | **map 缝** | 仅真边界差 |

### 要做

- [x] **F1–F3** 旧「禁 domain→bridge + 孪生留缝」文档（已被 c1210 取代）。
- [x] **F4** domain 组合 bridge DTO + 删消息孪生 — **c1210**。
- [x] **F5** `map.rs` 消息路径 identity；chunk/tool 保留薄 map。
- [x] **F6** bridge / 主仓 AGENTS 与本决议一致。

### 决议

- `domain → xylitol-ai-bridge(dto)`：**yes（仅 DTO）**。
- 拒绝抽第三 `*-types` crate（除非另议迁出 `xylitol-domain`）。
- bang-bash 新写：`type=message` + `role=bashExecution`；旧顶层 bash 读提升。

### 验收

- 无平行 domain `LlmMessage` enum 体；`project_for_llm` → `Vec<AiBridgeMessage>`；Env 永不进 bridge DTO；相关单测绿。

### 风险 / SDD

- 行为合约 → **c1210-refactor-compose-bridge-llm**。

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
| 2026-07-21 | agent | §C | commit `73d2aec6`；`args.rs` **暂留** tools 顶层（倾向日后整批迁 `support/`，现保持方案 1） |
| 2026-07-21 | agent | §D5–D7 | `driver.rs` → `driver/{mod,types,proto,in_process,remote}.rs`；修 remote/rest `XyDriverError` 映射 |
| 2026-07-21 | agent | 命名 | 决议：代码保留 `XyDriver`；心智「多 client 统一交互内核」写入 `docs/architecture/库与多客户端.md` |
| 2026-07-21 | agent | §D react/P3 | 评估 sub-turn state machine：**先不改**（当时仍写 D0 路径；已被同日「默认不做」决议取代） |
| 2026-07-21 | agent | docs | commit `04afab14`（Driver 心智 + P3 冻结写入） |
| 2026-07-21 | agent | §D react/session | 再分析：外置 react 测 / D8–D10 **默认不做**；可选卫生 D11（EventBus 错置 + 孤儿 tests.rs） |
| 2026-07-21 | agent | docs | commit `ac1899b6`（react/session 默认不拆写入 TODO/RETUNE） |
| 2026-07-21 | agent | §D11 | 严谨分诊：D11a=活着的错置（顺手挪）；D11b=真死孤儿 tests（建议删、待确认）；二者解耦 |
| 2026-07-21 | agent | docs | commit `5ae1ae8f`（D11 分诊入 TODO） |
| 2026-07-21 | agent | §D11a+b | 实施：EventBus `XyEventSink` 归 `infra/event`；删除孤儿 `session/tests.rs` |
| 2026-07-21 | agent | §E | E1+E4：7 个 async port 全 `dyn`；朴素 RPITIT 不可行；**默认 α 维持 async_trait**；β/γ 未开闸 |
| 2026-07-21 | agent | docs | commit `11eacb8c`（§E α 决议）；确认维持 α，转入 §F |
| 2026-07-21 | agent | §F1–F3 | AGENTS 叶归属表 + 禁新增孪生；`map.rs` 待删映射索引；**F4 domain→bridge 仍待决** |
| 2026-07-21 | agent | docs | commit `bf7c7a45`（F1–F3） |
| 2026-07-21 | agent | §F 决议 | **拒绝** `domain→bridge`；domain 业务自洽/准迁出；bridge 通用适配；孪生留在 `infra/map` 缝；F4 取消 |
| 2026-07-21 | agent | §F / c1210 | **解冻**：compose `Llm(AiBridgeMessage)`；消息孪生 map 删除；bash 嵌 message |
|  |  |  |  |

---

## 附录：探索时的量化快照（易腐，仅供对照）

> 数字会变；以 RETUNE + `wc`/`rg` 现场为准。

- `serde_json::Value` 在 `src/` 多处出现；hooks / react / driver 为热点。
- 无 `dyn Any` / `as_any` / `TypeId` 逃逸（加分）。
- `Result<…, String>` 在 session / 部分 infra 仍密；driver seam 已抬到 `XyDriverError`。
- `async_trait`：`src` 约 37 处；因 dyn port 默认维持（§E α）。

---

## 附录：与「不走 SDD」的协作约定（给后人）

1. 开 PR 标题建议前缀：`refactor(src): …` / `fix(agent): …`（Conventional Commits）。
2. PR 描述链到本文件章节（如 `src/_TODO.md` §B）。
3. 勾选与进度日志 **同一 PR 或紧随 PR** 更新，避免文档漂。
4. 若某项中途发现破坏 spec → **停手**，开 SDD change，并在该项下标注 change id 与「已升级 SDD」。
5. 本文件全部勾选且稳定一段时间后：可删或缩成 RETUNE 历史一段；**不要**升格为永久百科。
