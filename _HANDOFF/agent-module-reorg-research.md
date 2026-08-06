# xylitol `agent/` 模块重组调研（临时 SSOT）

> **地位**：会话临时真值源；路径 `.tmp/`（被 `*.tmp` ignore），**不进仓库**。
> **日期**：2026-08-06
> **范围**：以 `src/agent/` 为主；旁及 `protocol` / `infra` / `app::core` 边界与公开面。
> **约束**：跳过 llman SDD；代码重构完成后再统一修正/压缩 specs。
> **守恒钉**（重构不得破坏）：c1925 thinking full-replay + encrypted backfill；c1930 resume/import 前缀幂等 + 稳定折叠串；Trust≠工具 popup；工具 allow-all；MCP 配置驱动；Pre-1.0 provider；`agent` ↛ `infra`；Env 折叠只在 `project_for_llm`；Responses body 经 `ResponsesAssembler`。
> **本版重点**：§3 概念表；§4 Layout A–E；§5 微优化分轨；§14 对照 ai-agent-book；**§15 `agent::core` 渐进核**（推荐迁移策略）。

---

## 0. 一句话诊断

`agent/` 名义上是「薄编排」，实际上是 **God Aggregate（`AgentCapabilities` 挂在错名模块 `session/`）+ God Loop（`react.rs` 生产 ~1.6k / 测试 ~1.9k）+ 扁平底层杂项**；模块名按技术文件堆叠，不按领域边界，导致阅读成本高、并行改文件冲突面大、Rust 模块树无法表达「谁拥有什么」。

---

## 1. 现状地图（代码事实）

### 1.1 顶层树与体量

```text
src/agent/                         ~15k LOC（含测）
├── mod.rs                         入口叙事：Runtime + Capabilities（尚可）
├── builder.rs                     装配（好）
├── llm_project.rs                 Env→LLM 投影（领域正确；文件名偏技术）
├── context_policy/                请求布局钩子（小、清晰）
├── text.rs                        xml_escape（误层：通用工具）
├── tool_result_quiet.rs           工具结果静默化（应归 tools/history）
├── model/                         registry/manager/resolver/manifest
├── prompt/                        system/skills/commands/fragments
├── tools/                         ToolSet + freeze + validate
├── compaction/                    编排 + cut + estimate + LLM summarize…
├── runtime/                       react(3576) + hooks/retry/tool_*/obs…
└── session/                       ★ 名实最大漂移：AgentCapabilities God object
```

| 文件 | 行数 | 备注 |
|---|---:|---|
| `runtime/react.rs` | 3576 | 生产 ~1666 + 内联测 ~1910；头注释自称天花板 ~500 |
| `session/mod.rs` | 1449 | 生产 ~1110 + 测 ~339；`AgentCapabilities` + 61 个 `pub` 方法 |
| `compaction/mod.rs` | 1049 | 含大量测；与 `orchestrator.rs` 职责重叠感强 |
| `llm_project.rs` | 682 | 含 c1930 golden；职责正确 |
| `runtime/obs.rs` | 673 | 观测；可与 loop 解耦 |
| `model/resolver.rs` | 567 | 可接受 |
| 其余 | <500 | 多数合理 |

`src/AGENTS.md` 默认豁免「不为行数拆 ReAct / session manager」——本报告主张的是 **按领域边界重组命名与所有权**，不是「为行数而拆」。豁免不保护 **名实不符** 与 **错误聚合**。

### 1.2 入口叙事 vs 代码落点

`agent/mod.rs` 正确说了：

- `AgentRuntime` = 跑 ReAct
- `AgentCapabilities` = 被驱动的能力体
- 面只应从 `crate::agent::*` import

但落点是：

| 概念 | 声明位置 | 问题 |
|---|---|---|
| `AgentCapabilities` | `session/mod.rs` | **不是**「会话」；是运行时能力聚合 |
| `ModelRegistry` | `model/` 却 `session` 再 `pub use` | 外部经 `agent::session::ModelRegistry` 到达 |
| `QueueMode` / `QueueStats` | `session/queue.rs` | 队列属 **turn 改道**，不是持久化 session |
| `cancel_hook` / `observe_hook` | `session/mod.rs` 尾部 | hook 总线助手；driver 直接 reach `agent::session::*` |
| `save_trust_decision` | `session/trust.rs` | 自承无生产调用方；Trust 属 infra/产品面 |
| `text::xml_escape` | `agent/text` | 与编排无关 |
| `tool_result_quiet` | 顶层孤儿 | 属工具→历史投影 |

### 1.3 `AgentCapabilities`：浅模块（大接口）

字段 ~20，公开方法 **61**，按职责粗分：

| 簇 | 约方法数 | 领域 |
|---|---:|---|
| model / thinking / active turn | 13 | 模型绑定 |
| tools / freeze / gating | 11 | 工具表生命周期 |
| prompt / skills / fragments | 8 | 系统提示装配 |
| queue / steer / follow-up | 8 | 插话续跑 |
| session id / store / fork | 6 | 会话身份与持久化端口 |
| bash / export / compact | 9 | 旁路能力 |
| hooks / permission / 其它 | 6 | 横切 |

这是典型 **Facade 膨胀成 God Object**：调用方（尤其 `react.rs`）必须认识整张表；测试装配成本高；任意能力改动都碰同一文件 → **并行冲突热点**。

### 1.4 子模块耦合（`crate::agent::X` 边）

高频边（生产+测合计，量级）：

```text
runtime  → session     ★ 最强（Capabilities 是 loop 的环境）
session  → runtime     ★ 反向（hooks / script_hook_ctx）——环
session  → compaction / context_policy / prompt / model / tools
runtime  → compaction / model / tools / prompt / llm_project
compaction → llm_project
```

**环 `runtime ↔ session`** 是结构味最冲的信号：说明「session」装的不是会话，而是「runtime 的搭档杂物箱」。

### 1.5 外部 reach（分层违规味道）

`app/core` 多处 `crate::agent::session::*`（QueueMode、ModelRegistry、QueueStats、cancel/observe_hook），与 `agent/mod.rs`「面只从 mod 级进」及 `lib.rs`「勿把 `agent::session::*` 当稳定面」互相打架——**稳定面未收口，错误路径已长成习惯**。

### 1.6 分层抽检（仍健康的部分）

- 生产路径 `agent` ↛ `infra`（仅 `#[cfg(test)]`）
- Env 折叠只在 `llm_project`
- Responses body 业务入口 `ResponsesAssembler`（另：`messages_to_responses_input*` 仍 pub，叙事张力，属 bridge，本报告次要）

---

## 2. 病根分类（为何「slop」）

| 病 | 表现 | 根因 |
|---|---|---|
| **名实漂移** | `session` ≠ 会话；像 `AgentHost` / `AgentKernel` | 历史从 pi 能力袋迁来，目录未随概念改名 |
| **浅模块** | 61 方法 Facade | 「一个 struct 搞定所有 slash」压过「深模块」 |
| **错误归属** | trust / hook helpers / xml_escape / quiet | 就近丢进当前最大袋子 |
| **环依赖** | runtime ↔ session | 能力体与循环未用端口切开 |
| **文件内测膨胀** | react 一半以上是测 | 测与剧本同居；编辑/导航痛，但 AGENTS 曾豁免 |
| **公开面泄漏** | app 直达 `agent::session::*` | re-export 纪律写了没执行 |
| **双词汇** | Capabilities / Session / Runtime / Context | `AgentContext`（protocol）vs Capabilities 需靠注释防撞 |

这不是「缺抽象层」，而是 **缺清晰的 bounded context 目录 + 小而深的对外 seam**。禁止为「未来插件」再堆 `Xy*`。

---

## 3. 概念表（词汇 SSOT · 临时）

> 读代码 / 改目录时以本表为准。产品文用中文心智；代码标识符用右列英文。
> **易撞名**单独标出——重组时宁可改模块名，也不再靠注释防撞。

### 3.1 核心名词

| 概念（中） | 建议代码名 | 今日落点 | 是什么 | 不是什么 |
|---|---|---|---|---|
| 对话核心 / 编排层 | `crate::agent` | `src/agent` | ReAct + 能力装配 + 投影 | HTTP、JSONL 落盘实现 |
| 一轮对话循环 | `AgentRuntime` + `turn`/`runtime` | `runtime/react.rs` | 问模型→工具批→写回直到结束/中止 | 会话树 UX、持久化格式 |
| 能力聚合体 | **待定类型名**（见 §3.3） | `session::AgentCapabilities` | Runtime 持有的可变能力袋 | `protocol::AgentContext`（LLM 请求快照） |
| 会话（持久） | `XySessionStore` + protocol session types | `infra/session` + `protocol/session` | JSONL / fork / leaf | 能力聚合体所在目录名 |
| 会话身份槽 | `session_id` / SessionSlot | Capabilities 字段 | 当前绑定的 session id + ensure/fork | 整袋 Capabilities |
| 历史投影 | `project_for_llm` | `llm_project.rs` | `AgentMessage`→LLM 叶；Env 折叠 | Responses `input` 组装 |
| 请求布局策略 | `ContextPolicy` | `context_policy/` | tools/status_bar/date 钩子默认 | 状态栏完整实现（deferred） |
| 工具表 | `ToolSet` + freeze | `tools/` | 本 turn 可见工具 + 首轮闸 | 具体 read/bash 实现 |
| 插话/续跑队列 | steer / follow-up queue | `session/queue.rs` | 改道消息缓冲 | 持久化 session |
| 压缩 | compaction | `compaction/` | cut + 摘要 + 触发 | 发明旧 thinking signature |
| 组合根 | composition / bootstrap | `app/core` | 同时碰 agent+infra 装配 | 面随意 reach infra |
| 库入口契约 | `Xy*` | `protocol` + `lib.rs` | 跨层精选口 | 内部协作前缀 |

### 3.2 关系（一句话）

```text
面/Driver
  → AgentRuntime.run(…)          // 循环
       持有  能力聚合体            // 模型/工具/prompt/队列/store…
            → project_for_llm     // 历史→LLM 叶
            → XyModel             // port；真 HTTP 在 bridge/infra
            → XySessionStore      // port；真 JSONL 在 infra
```

### 3.3 能力聚合体：类型命名备选（替代「AgentHost」）

`AgentHost` 在 Rust/agent 圈不常见，也易跟「进程宿主 / extension host」混淆。下面按推荐度排序；**目录名可与类型名解耦**（例如类型 `Agent`、目录 `capsule/`）。

| 候选类型名 | 语感 | 优点 | 风险 | 推荐度 |
|---|---|---|---|---|
| **`Agent`** | 最常见 | 短；「Runtime 驱动 Agent」直觉好 | 与 crate/层名 `agent`、产品「agent」撞词；文档要写清 | ★★★★★ |
| **`AgentCore`** | 常见变体 | 强调「循环外的核心状态」 | 略空 | ★★★★ |
| **`AgentState`** | 偏函数式 | 强调可变状态袋 | 易被当成纯数据/无行为；实际有大量方法 | ★★★★ |
| **`AgentCapsule`** | 少见但准 | 「能力胶囊」；不暗示 OS host | 需团队习惯 | ★★★★ |
| **`AgentWorkspace`** | IDE 语感 | 含模型/工具/cwd/session 的工作台 | 与「磁盘 workspace / 项目根」撞 | ★★★ |
| **`AgentSession`** | 表面贴切 | — | **强烈不推荐**：与持久化 Session、protocol session 三撞 | ★ |
| **`AgentCapabilities`** | 现状 | 零改名成本 | 长；已证明诱导「什么都往里塞」 | ★★（仅过渡） |
| `AgentHost` | 初稿 | 表达「被驱动的宿主」 | 不常见；extension-host 联想 | ★★ |
| `AgentFacade` | 模式名 | 诚实说是门面 | 贬义「浅模块」；不领域 | ★ |
| `AgentHandle` | 句柄 | 像 Arc 包装 | 暗示廉价间接层 | ★★ |

**目录名备选**（与类型可混搭；**不用 capsule**）：

| 目录 | 配什么类型名 | 备注 |
|---|---|---|
| `state/` | `AgentState` / `Agent` | **Layout A 首选目录** |
| `kit/` | `AgentKit` | 能力套件隐喻 |
| `hub/` | `AgentHub` | 汇合点 |
| `capabilities/` | 保留旧类型名 | 过渡友好、零发明 |
| `inner/` | 对外 `Agent` | Rust 惯用内核模 |
| `control/` | `AgentControl` | 偏 Driver API |
| `host/` / `capsule/` | — | **不推荐**（怪 / 易混） |
| `session/` | — | **淘汰**（名实已证伪） |

**初选建议（可推翻）**：类型 **`AgentState`**（或过渡期仍叫 `AgentCapabilities`）+ 目录 **`state/`**；对外叙事：「`AgentRuntime` 驱动 `AgentState`」。

### 3.4 Bounded context ↔ 目录职责（逻辑，不绑死一种 layout）

| BC | 负责 | 不负责 |
|---|---|---|
| 循环（turn/runtime/loop） | ReAct、事件流、retry、tool batch/exec | 持久化格式、HTTP |
| 能力聚合（capsule/…） | 窄 facade + 子协作 | trust/xml/通用 escape |
| 历史投影（history/project） | `project_for_llm`、tool quiet、折叠串钉 | Assembler body |
| 请求布局（context） | ContextPolicy 默认 | 状态栏完整实现 |
| 模型（model） | registry/select/thinking | provider HTTP |
| 工具表（tools） | ToolSet/freeze/validate | 具体工具实现（infra） |
| 提示（prompt） | system/skills/commands | 资源发现（infra loader） |
| 压缩（compaction） | cut/触发/摘要编排 | 发明旧 signature；JSONL 细节 |
| 队列（queue） | steer/follow-up | UI 还原编辑器 |
| 会话身份（session slot） | id/ensure/fork via port | God 聚合整袋 |

---

## 4. 多种 `agent/` Layout 备选

> **共同原则**：单 crate；不抽 domain 包；对外一个深入口；对内少环；孤儿归位；`session/` 不再装能力袋。
> **差异**：顶层按「领域名词」还是按「流水线阶段」切；能力袋目录叫什么；queue 挂循环还是挂胶囊。

### Layout A — 领域并列（结构保留；**目录名不用 capsule**）

**隐喻**：Runtime 驱动一颗「能力聚合」；历史/压缩/模型是并列领域包。
**结构**与初版 A 相同，仅替换原 `capsule/` 目录名（见下表）。

```text
src/agent/
├── mod.rs
├── builder.rs
├── <AGG>/                   # 原 session/ 能力袋 —— 名见命名表
│   ├── mod.rs
│   ├── model_bind.rs
│   ├── tool_table.rs
│   ├── prompt_slot.rs
│   ├── session_slot.rs      # 仅会话身份
│   ├── queue.rs
│   ├── bash.rs
│   ├── export.rs
│   └── stats.rs
├── turn/                    # 原 runtime/（产品「一轮对话」）
│   ├── mod.rs
│   ├── react.rs | loop.rs
│   ├── config.rs
│   ├── hooks.rs, retry.rs, tool_batch.rs, tool_exec.rs
│   ├── permission_router.rs, script_hook_ctx.rs
│   ├── event.rs, obs.rs
│   └── tests/ …
├── history/
│   ├── project_for_llm.rs
│   └── tool_quiet.rs
├── context/                 # 原 context_policy/
├── model/
├── prompt/
├── tools/
└── compaction/
```

**<AGG> 目录命名（取代 capsule）**

| 目录 | 建议类型名 | 语感 | 优点 | 风险 | 推荐 |
|---|---|---|---|---|---|
| **`state/`** | `AgentState` | 最常见 | Rust/状态机语感自然；不造词 | 易被当成「纯数据、无方法」——靠 `mod.rs` 叙事补上 | ★★★★★ |
| **`kit/`** | `AgentKit` | 能力套件 | 短；暗示工具+模型+提示一整套 | 略产品卡通 | ★★★★ |
| **`hub/`** | `AgentHub` | 汇合点 | 表达「多协作汇于此」 | 略框架腔 | ★★★★ |
| **`control/`** | `AgentControl` | Driver 控制面 | 和面侧 API 同向 | 易与 permission/控制论混淆 | ★★★ |
| **`capabilities/`** | `AgentCapabilities` 或缩短 | 零发明 | 与现状类型一致，迁移最顺 | 长；曾诱导塞东西——靠内拆文件克制 | ★★★★ |
| **`inner/`** | 对外 `Agent`，模内 `Inner` | Rust 惯用 | 极惯用拆法 | 对外若叫 `Agent` 需防与层名口头撞 | ★★★★ |
| `capsule/` | — | — | — | **已弃用**（生造、怪） | ✗ |
| `host/` | — | — | — | 不常见 / extension-host 联想 | ✗ |

**类型 × 目录 混搭示例（均可）**

| 组合 | 读法 |
|---|---|
| `state/` + `AgentState` | 「Runtime 持有 AgentState」 |
| `state/` + `Agent` | 目录表状态、类型表实体（短） |
| `kit/` + `AgentKit` | 整齐对称 |
| `capabilities/` + 暂留旧名 | 先搬目录、后改类型 |
| `inner/` + `Agent` | `agent::inner` 藏字段，根上 `pub use Agent` |

**初选（可推翻）**：目录 **`state/`** + 类型 **`AgentState`**（或对外仍短期叫 `AgentCapabilities` 再改）。叙事：「`AgentRuntime` 驱动 `AgentState`」。

| | |
|---|---|
| 优点 | 名实清晰；并行按 BC；与产品「一轮对话」对齐（`turn`） |
| 缺点 | 顶层目录偏多；`<AGG>` 名需一次拍板 |
| 并行 | state/kit 内拆 ∥ compaction ∥ history 高 |

#### Layout A 修正：`bash` / `export` **不进** `<AGG>`

初稿把它们画进能力袋，是沿袭今日 God 聚合——**那正是要拆的 slop，不是目标**。

| 文件 | 代码事实 | 属 Agent core？ | 应去哪 |
|---|---|---|---|
| `session/bash.rs` | 用户 **`!`/`!!` bang**：持 `XyBashExecutor`，写 Env 进 store；TUI→Driver→`execute_bash`，**不经** ReAct | **否**（≠ 工具表里的 bash） | Driver/`app`，或薄模 `session_io`；勿进 state 热字段 |
| `session/export.rs` | `/session-export|import` HTML/JSONL + `XyExportIo` | **否** | 同上 |
| `session/stats.rs` | 数 user/assistant | 弱相关 | 可旁置；勿膨胀 facade |
| `queue.rs` | steer/follow-up | **是** | 留 `<AGG>` 或 `turn` |
| model/tool/prompt/session_slot | 循环环境 | **是** | `<AGG>` 核心 |

误放根因（现注释）：「Capabilities 做 session context 唯一 holder」——用袋子省传参，把**产品旁路**焊进对话核心。

**修订 `<AGG>`（推荐）**：

```text
<AGG>/                    # 只服务 ReAct / 改道 / 会话身份
├── mod.rs
├── model_bind.rs
├── tool_table.rs
├── prompt_slot.rs
├── session_slot.rs
└── queue.rs

session_io/  或  app/core 接线   # bang + export/import（stats 可选）
```

对齐书中公式：**core = 循环 + 上下文投影/装配 + 工具表**；bang/导出是 harness/产品面。

### Layout B — 流水线阶段（Pipeline）

**隐喻**：目录 = 数据流阶段，而不是「名词袋子」。

```text
src/agent/
├── mod.rs
├── builder.rs
├── assemble/                # prompt + context_policy + 资源应用到提示
│   ├── prompt/ …
│   └── context.rs
├── bind/                    # model/tools/session_id/queues（勿塞 bang/export）
│   ├── mod.rs               # 类型 Agent / AgentState
│   ├── model.rs, tools.rs, session.rs, queue.rs
│   # bash/export → session_io 或 app，见 Layout A 修正
├── project/                 # 历史→LLM（原 llm_project + quiet）
├── loop/                    # ReAct（原 runtime）
├── compact/                 # 压缩
└── model/                   # 或并入 bind/model —— 二选一避免双分
```

| | |
|---|---|
| 优点 | 新同学按「请求怎么走」找代码；和 sequence 图同构 |
| 缺点 | `model` 既是阶段又是领域，易双分；`assemble` vs `project` 需纪律 |
| 适合 | 强文档/教学向；流水线评审 |

### Layout C — 少顶栏（合并大目录）

**隐喻**：顶层只留 4～5 个大盒，盒内再领域拆。

```text
src/agent/
├── mod.rs
├── builder.rs
├── runtime/                 # 保留名：loop + hooks + tool_exec + obs
├── state/                   # 全部可变能力（原 session God → 内再拆文件）
│   ├── mod.rs               # AgentState facade
│   ├── …协作文件
├── view/                    # 「给模型看的」：project_for_llm + context + tool_quiet
├── support/                 # model + prompt + tools + compaction
│   ├── model/, prompt/, tools/, compaction/
```

| | |
|---|---|
| 优点 | 顶层极简；改 import 面小 |
| 缺点 | `support/` 易变新杂物箱；`view` 名偏 UI |
| 适合 | 想先纠偏、少动心理模型 |

### Layout D — 双核（Loop ∥ World）

**隐喻**：只有两个一等公民——循环与世界状态；其余都是 world 的子模。

```text
src/agent/
├── mod.rs
├── builder.rs
├── loop_/                   # 或 `react/`（避免 raw identifier 也可用 `runloop/`）
│   └── …现 runtime 全套
└── world/                   # 原 Capabilities + model + prompt + tools + history + compaction + context
    ├── mod.rs               # Agent / AgentWorld facade
    ├── history/
    ├── model/
    ├── prompt/
    ├── tools/
    ├── compaction/
    ├── context/
    ├── queue.rs
    └── session.rs           # 仅身份
```

| | |
|---|---|
| 优点 | 环依赖最好杀（loop 依赖 world，world 禁止依赖 loop）；心智两极 |
| 缺点 | `world/` 一时仍大；需强 `pub(crate)` 纪律防再次 God |
| 适合 | 优先破 `runtime↔session` 环 |

### Layout E — 保守改名（过渡态，可作 Phase 0）

```text
src/agent/
├── …现结构…
├── capabilities/            # 仅 session/ 改名
├── history/                 # llm_project + tool_quiet 归位
├── runtime/                 # 暂不改名
└── （删 text、trust 错置）
```

| | |
|---|---|
| 优点 | 冲突最少 |
| 缺点 | 不解决 God 接口 |
| 定位 | **过渡**，不是终局 |

### Layout 对照

| | A 领域并列 | B 流水线 | C 少顶栏 | D 双核 | E 过渡 |
|---|---|---|---|---|---|
| 名实纠偏 | 强 | 强 | 中 | 强 | 弱 |
| 破环 | 中（需纪律） | 中 | 中 | **最强** | 弱 |
| 顶层噪音 | 中高 | 中 | **低** | **最低** | 低 |
| 并行切片 | **好** | 中 | 中 | 中（world 易抢） | 差 |
| 与产品词对齐 | turn + state/kit | 阶段词 | 弱 | loop/world | 弱 |
| 迁移成本 | 中 | 中高 | 低中 | 中 | **最低** |

**组合策略（推荐讨论）**：结构终局偏 **A（`<AGG>`=`state/` 等）或 D**；若怕一次到位 → **E → A** 或 **E → D**。B 适合流水线教学。C 仅当「顶层必须很短」。**不用 capsule/host 作目录名。**

### 4.x 对外深入口（各 Layout 共用）

**稳定 re-export 白名单**：`AgentBuilder`、`AgentRuntime`、能力聚合类型、`project_for_llm`、`ContextPolicy`、`ToolSet`/freeze、`Queue*`、`AgentHooks`、可选 `AgentMessage`/`XyEvent`。

**禁止**：`agent::session::*` 作稳定路径；app 一律 `crate::agent::…`。

### 4.y 能力袋接口深化（各 Layout 共用设计，不绑 Host 名）

目标：把 61 方法拆成 **内部协作结构体**，Facade 变深——**不是**再造一排 `Xy*` port。

```text
Agent（示意）
  // loop 热路径
  prepare_model / tools / system_prompt / context_policy
  queues / hooks / permission / store / sink / session_id
  bind_active_turn / clear_active_turn / maybe_auto_compact

  // Driver / 面
  select_model / set_thinking* / set_tools* / freeze*
  steer* / follow_up* / apply_prompt* / export* / bash*
```

内部分支：`ModelBind`、`ToolTable`、`PromptSlot`、`SessionSlot` = `pub(crate)`。
破环：hook helpers 进 loop 侧；能力袋 **禁止** 依赖 loop 模块。

---

## 5. 微优化轨（与结构重组 **严格分轨**）

> 用户诉求：统计等「走两次迭代」可收成一次；**可放在结构变更前或后，但不要与搬模块混在同一批提交/同一 PR 心智里**。

### 5.1 已证实热点

`src/agent/session/stats.rs` · `compute`：

```rust
// 今日：messages 上两次 filter().count()
user_messages = iter.filter(role==user).count();
assistant_messages = iter.filter(role==assistant).count();
```

应改为 **单次扫描**累加 `user` / `assistant`（及其他若需要），`total_messages = len` 仍 O(1)。

同类可扫（另开 commit，勿与改名绑）：

| 位置 | 现象 | 建议 |
|---|---|---|
| `session/stats.rs` | user/assistant 双遍 | 单遍计数 |
| `prompt/skill_expand.rs` | 两处 `for msg in messages` | 评估合并是否语义允许 |
| `compaction/token_estimator.rs` | 多段 iter | 仅当 profile 证明热再动 |
| 其它 `filter(role==…).count()` | 模式重复 | 小工具 `count_roles(&msgs) -> RoleCounts` 放 **stats/history 旁**，勿新建 util 杂物箱 |

### 5.2 分轨纪律

| 轨 | 做什么 | 不做什么 |
|---|---|---|
| **结构轨** | 改目录/类型名/`pub use`/拆协作对象 | 不改循环算法、不合并 iter |
| **微优化轨** | 单遍计数、减少重复扫描、局部热路径 | 不改模块树、不顺手改名 |
| **行为钉轨** | c1925/c1930 等 | 与上两者隔离 |

提交信息建议前缀：`refactor(agent): …`（结构）vs `perf(agent): single-pass session stats`（微优化）。

### 5.3 时机

- **结构前做**：stats 单遍 —— 文件还在 `session/stats.rs`，diff 极小，验证简单。
- **结构后做**：等迁到 `capsule/stats.rs` 再优化 —— 也行，但多一次路径噪音。
- **禁止**：同一 commit 既 `session→capsule` 又改计数逻辑。

---

## 6. 与「高质量 Rust / 低耦合 / 可扩展」诉求的对齐

### 6.1 模块与 crate 实践

| 做法 | 本仓库裁决 |
|---|---|
| 按层拆 workspace crate | **否**（`src/AGENTS.md` 禁止为分层再抽 domain crate） |
| `agent` 内目录 = bounded context | **是** |
| `pub use` 瀑布 | **收敛**；子模默认 `pub(crate)` |
| 测试与生产文件 | 大测迁 `src/agent/turn/tests/` 或 `tests/agent/`；golden 留近代码 |
| 新 port / `Xy*` | 仅真实第二实现时；能力袋内拆 **不要** 升 port |

### 6.2 避免垃圾与 N+1 类问题

本层主要不是 DB N+1，而是 **编排重复扫描 / 双路径**：

| 反模式 | 现况风险 | 纪律 |
|---|---|---|
| 同切片多遍 `filter(role)` | `stats::compute` 已证实双遍 | **微优化轨**单遍；勿混进改名 PR |
| 每 tool 重复投影/校验 | validate 已集中；quiet 分散 | quiet 归 history/view |
| 每 turn 重建超大 prompt | fragment id 跳过已有 | 留在 PromptSlot |
| 双路径 Env 折叠 | 目前单路径 | **禁止**第二折叠 |
| 测试里拼完整能力袋 | session 测装配重 | `pub(crate)` fixture builder |

### 6.3 并行开发可行性（worktree / 多人）

| 切片 | 触碰 | 可并行？ |
|---|---|---|
| 微优化 stats 单遍 | 仅 `stats.rs` | **高**；与结构轨错开 commit |
| P0 孤儿归位 | history/、删 text/trust | 高 |
| P1 目录改名 + re-export | 依 Layout：capsule/state/world… + app/core | 中 |
| P2 能力袋内拆协作 | 仅该目录 | 高 |
| P3 runtime→turn/loop 改名 | 全仓 import | 低——单独窗口 |
| P4 react 测外迁 | loop 侧 | 中 |
| P5 compaction | compaction/ | 高 |

硬规则：多 worktree **禁止共用 `CARGO_TARGET_DIR`**（`just cargo-wt-env`）。

---

## 7. 实施顺序（跳过 SDD；代码优先）

> **先选定 Layout（A/B/C/D）+ 类型名（§3.3）**，再开刀。E 可作跳板。

### 轨 0（可选独立）— 微优化

- `perf(agent): single-pass role counts in session stats`
- 不改路径、不改类型名。结构前或后均可。

### Phase 0 — 消毒（低风险）

1. 撒谎注释纠偏（可另 commit）
2. `tool_result_quiet` / `llm_project` 归入选定 Layout 的 history/view/project
3. 删顶层 `text`；挪走 `trust`
4. hook helpers 迁到 loop 侧，准备破环

### Phase 1 — 名实 + 公开面（依 Layout）

1. `session/` → `capsule/` | `state/` | `world/` | `capabilities/` …
2. 类型改名（若选）：`AgentCapabilities` → `Agent` / `AgentState` / …
3. `agent/mod.rs` 白名单；消灭 app 的 `agent::session::`
4. `ModelRegistry` 不再经能力袋跳板

### Phase 2 — 能力袋内拆（行为守恒）

`ToolTable` / `ModelBind` / `PromptSlot` / `SessionSlot` 委托；对外语义冻结。

### Phase 3 — 循环目录改名（机械）

`runtime` → `turn` | `loop` | `runloop`（按 Layout）；单独立窗。

### Phase 4 — Loop 可读性（可选）

生产分段 + 测外迁；**不**上状态机（除非另开激进波）。

### Phase 5 — compaction / model 打磨（可并行）

### Phase 6 — specs 统一修正/压缩（后置）

---

## 8. 风险与守恒清单

| 风险 | 缓解 |
|---|---|
| 改名导致全仓 import 海啸 | Phase 1/3 单独提交；机械 `rg` + 编译驱动 |
| 拆能力袋改变方法可见性 | 先委托、后删；对外签名冻结一波 |
| 微优化与改名混提 | **禁止**；分轨分 commit（§5） |
| 碰 `project_for_llm` 折叠串 | **禁止**改字符串；测：`llm_project` golden + prefix idempotency |
| 碰 thinking replay | **禁止**改 bridge assemble 语义；agent 只搬文件 |
| 并行 worktree 错二进制 | `eval "$(just cargo-wt-env)"` |
| 测外迁漏跑 | 搬迁后 `just qa`；关注 slow `test_bash_cancel` |

---

## 9. 明确不做什么

- 不抽第三个 LLM/domain crate
- 不把 `LlmAdapter` 升成产品 `Xy*`
- 不为插件预挖 Extension Host
- 不把 `infra/session/manager` 塞进 agent（持久化仍 infra）
- 不在本波改 provider dialect / Assembler 行为（可另开 bridge 叙事切片）
- 不把本报告写入 `docs/research/` 或 llmanspec

---

## 10. 建议的「完成定义」

1. 概念表（§3）与选定 Layout 一致；目录能回答领域问题；无 `session/` 装能力袋
2. `rg 'agent::session'` → 零（或仅历史注释）
3. 能力聚合类型生产 facade &lt; ~400 行委托；子协作清晰
4. `agent` ↛ `infra`；循环目录 ↛ 能力袋 的生产环消失
5. `just qa` 绿；c1925/c1930 相关测绿
6. 微优化（若做）有独立 `perf` commit，不混在结构 PR
7. 本文件仅 `.tmp/`；落地后只把**最终模块树摘要**写入 `src/AGENTS.md`

---

## 11. 附录：关键误置速查

| 路径 | 问题 | 迁往（随 Layout） |
|---|---|---|
| `session/mod.rs` `AgentCapabilities` | 名实不符 | `state/` / `kit/` / `hub/` / `capabilities/` / `inner/` |
| `session/queue.rs` | 非持久化会话 | 能力袋内或 loop 侧 queue |
| `session/trust.rs` | 无生产调用；错层 | app/infra |
| `session` hook helpers | driver 依赖 | loop/hooks |
| `session/stats.rs` 双遍 count | 微优化轨 | 同文件单遍（先/后结构均可） |
| `text.rs` | 通用 escape | 调用方旁路 |
| `tool_result_quiet.rs` | 顶层孤儿 | history/view/project |
| `llm_project.rs` | 名偏技术 | history/project_for_llm |
| `runtime/react.rs` 头注释 | 自称 500 vs 3.5k | 改叙事或真拆 |
| app → `agent::session::*` | 稳定面泄漏 | `crate::agent::*` |

---

## 12. 待你拍板（阻塞开干）

1. **Layout**：A 领域并列 / B 流水线 / C 少顶栏 / D 双核 / 先 E 再 A或D
2. **若选 A，`<AGG>` 目录**：`state/` / `kit/` / `hub/` / `capabilities/` / `inner/` / `control/`（**不用 capsule**）
3. **类型名**：`AgentState` / `Agent` / `AgentKit` / `AgentCore` / 暂留 `AgentCapabilities`
4. **循环目录名**：`turn` / 保留 `runtime` / `runloop`
5. **微优化**：结构前先做 stats 单遍 / 结构后 / 先不做

---

## 14. 对照 `ai-agent-book`：agent 模块该有什么 / 不该有什么

> 参考：`/home/l8ng/Projects/__straydragon__/ai-agent-book`（李博杰等《深入理解 AI Agent》）。
> 用途：用书中**稳定原则**给 xylitol `agent/` 划界，并锚定 **Coding Agent 主形态**；不把书中后置章（训练/多 Agent 社会等）塞进本仓主线。

### 14.1 书中核心公式（两层）

| 层次 | 公式 | 含义 |
|---|---|---|
| 组成（Demo） | **Agent = LLM + 上下文 + 工具** | 大脑 + 眼睛 + 手脚 |
| 生产（Harness） | **Agent = Model + Harness** | Harness ≈ 上下文 + 工具 + **约束 + 验证 + 纠正** |

Harness 五功能（书 ch1）：

| 功能 | 职责 | Coding 场景例子 |
|---|---|---|
| Context | 决策点看到什么 | 系统提示、AGENTS.md、轨迹、压缩、状态栏 |
| Tools | 能做什么 | read/write/edit/bash/grep/glob/（可选 interpreter） |
| Constrain | 能做/不能做的边界 | 权限、沙盒、危险命令语义闸 |
| Verify | 结果对不对 | 测试/CI/lint、工具 JSON 校验、完成标准 |
| Correct | 错了怎么补 | 重试、熔断、回滚、压缩后重试 |

工程演进（书）：提示 → 上下文 → Harness → Loop；原则：**保持简单、保持透明、设计好 ACI（Agent-Computer Interface）**。

### 14.2 Coding Agent 主形态（书 ch5）

**范式**：**Coding Agent + 文件系统** = 开放任务型通用 Agent 的技术内核（Manus / OpenClaw 同构）。

**七个核心工具**（感知+执行；协作/事件/用户沟通多在**框架**而非工具表）：

1. Code Interpreter（可与 Bash 合并）
2. Bash
3. Read
4. Write
5. Edit
6. Glob
7. Grep

**文件系统中枢**：记忆/产物/中间态以工作区文件为真（如 MEMORY.md）；进程态可重建，**文件态持久**。
**交互现实**：生产 Coding Agent 多为 **反应式 ReAct**，按需裁剪「文档化→设计→实现→测试」瀑布；完成标准宜为「验证通过」而非「代码写完」。
**项目指令文件**（AGENTS.md / CLAUDE.md）：稳定前缀，KV Cache 友好——属**上下文**，不是新业务子系统。
**适用边界**：开放任务 / 产物多样 → Coding 为架构中枢；封闭客服等 → Coding 仍是能力底线，但不一定是中枢。

→ **xylitol 产品定调**（个人开箱 coding harness）与书完全同构：主形态就是 Coding Agent + 仓库文件系统；TUI/Print 是面，不是另一套 Agent。

### 14.3 映射到 xylitol 分层：什么进 `agent/`

书把「Model 之外」统称 Harness。xylitol **禁止**把整个 Harness 塞进一个 God 目录，而是：

```text
Model（HTTP/SSE）     → packages/xylitol-ai-bridge + infra adapter
Harness 编排核         → src/agent/          ★ 本报告重组对象
Harness 执行/落盘实现  → src/infra/          （工具实现、JSONL、MCP 客户端、trust store…）
契约 / 线协议          → src/protocol/
面 / Driver            → src/app/
```

**应在 `agent/` 的（编排 / 循环 / 给模型看什么）**

| 书中概念 | xylitol 落点（目标） | 今日大致 |
|---|---|---|
| ReAct 循环 | `turn/` / `runtime/` | `runtime/react.rs` |
| 轨迹→模型可见上下文 | `history/project_for_llm` | `llm_project.rs` |
| 系统提示 / Skills / 碎片 | `prompt/` | `prompt/` |
| 请求布局策略 | `context/`（ContextPolicy） | `context_policy/` |
| 上下文压缩编排 | `compaction/` | `compaction/` |
| 本轮工具表 + 冻结/闸 | `tools/` + state 内 tool_table | `tools/` + Capabilities |
| 模型/thinking 绑定（选谁，不发 HTTP） | state 内 model_bind + `model/` | `model/` + Capabilities |
| 插话/续跑队列 | state 或 turn 侧 queue | `session/queue.rs` |
| 循环级 retry / 工具批调度 | turn 内 | `retry` / `tool_batch` |
| 循环钩子（停轮、before tool） | turn/hooks | `runtime/hooks` |
| 能力袋 facade | **`<AGG>`=`state/` 等** | 错挂在 `session/` |

**不应在 `agent/` 的（书里属 Harness 执行面或后置章）**

| 概念 | 应在 | 理由 |
|---|---|---|
| LLM HTTP/SSE / dialect | bridge + infra | 公式里的 Model；业务只认 `XyModel` |
| read/write/bash/grep **实现** | infra/tools | ACI 实现；agent 只调度 `XyTool` |
| Session JSONL / 会话树存储 | infra/session | 「文件系统中枢」的落盘；agent 经 `XySessionStore` |
| Trust 持久化 / 项目闸 UI | infra/trust + app | 产品 Trust≠工具 popup；非循环内核 |
| MCP 客户端装配 | infra/mcp | 配置驱动；未配置零成本 |
| TUI / Print / Server 渲染 | app | 面；经 Driver |
| xml_escape、无调用 trust helper | 删或旁路 | 杂物 |
| RAG / 向量库主路径 | 非本仓主线 | 书 ch3；xylitol 用仓库文件+工具搜 |
| 模型后训练 / Eval 框架 | 仓外 / 另轨 | 书 ch6–7 |
| 语音 / Computer Use / 机器人 | 后置 | 书 ch9 |
| 多 Agent 社会 / A2A | 后置 | 书 ch10；子 agent 若做也走编排扩展，不先堆抽象 |
| 「强制设计文档瀑布」状态机 | **不要**写死进 agent | 书：真实产品是 ReAct 裁剪；流程靠提示+技能+用户，不靠硬编码阶段机 |
| 工具侧逐调用 popup | **产品禁止** | xylitol allow-all；约束靠 trust/沙盒/语义闸（infra） |

### 14.4 Harness 五功能 × xylitol 归属（防再塞错）

| 功能 | agent（编排） | infra / 外 | app |
|---|---|---|---|
| Context | 组装、投影、压缩触发、policy | 资源发现（AGENTS.md 加载） | 展示轨迹 |
| Tools | ToolSet、freeze、batch、validate 调度 | 七工具实现、MCP | slash 触发 |
| Constrain | 咨询 `XyPermission` 路由 | allow-all 实现、沙盒、trust store | `/trust` UX |
| Verify | stop hook、完成语义；安静化工具结果进历史 | 测试/lint **经 bash 工具**；CI 在仓外 | 展示失败 |
| Correct | provider retry、compact 后继续 | 工具错误结构化返回 | 中止/重试 UX |

### 14.5 对 Layout A 命名的启示

- 书把「Model 外一切」叫 **Harness**——若模块叫 `harness/`，易吞掉整个 infra，**过宽**，不推荐作 `<AGG>`。
- 能力袋本质是：**单次 Agent 实例上，循环所依赖的可变工作状态**（模型绑定、工具表、提示、session id、队列）→ 目录名 **`state/`** 与书/工程直觉最贴。
- 循环目录 **`turn/`** 对齐产品「一轮对话」与书 ReAct 迭代；保留 `runtime/` 也可以，但弱于产品词。
- **不要**用 `session/` 装能力袋：书里 Session/Sessionless 谈的是**交互与工作区存活**，持久化在文件系统/infra，不是 God facade。

### 14.6 Coding Agent 检查清单（重组后自检）

重组完成后，`agent/` 应能清晰回答：

1. 如何跑一轮 ReAct？（turn）
2. 发给模型的消息从哪来？（history + prompt + context）
3. 本轮有哪些工具、是否冻结？（tools + state）
4. 上下文爆了怎么办？（compaction）
5. 插话/中止如何进循环？（queue + runtime API）

下列问题若只能在 `agent/` 里答，说明塞错了：

- JSONL 如何写盘？→ infra
- bash 如何起进程？→ infra
- 终端怎么画 thinking 边框？→ app
- OpenAI SSE 如何解析？→ bridge

### 14.7 与 xylitol 已钉产品的对齐（勿被书带偏）

| 书中常见默认 | xylitol 钉 |
|---|---|
| 工具默认需用户授权（Claude Code 例） | **工具开箱 allow-all**；Trust 闸的是**项目本地资源是否加载** |
| 通用 Agent 可叠浏览器/搜索模块 | Pre-1.0 交付：OpenAI 兼容 + Anthropic；MCP **配置才装配** |
| Sessionless 常驻 Gateway | 主形态是本地 TUI/Print + 可恢复 session 文件，不是 IM Gateway |
| 记忆用 MEMORY.md 等 | 主记忆 = **session 轨迹 + 仓库文件**；不另造平行记忆产品（除非另开 change） |

---

## 13. 修订记录

| 日期 | 说明 |
|---|---|
| 2026-08-06 | 初稿：QA 基线 + God 对象量化；方案 A；`.tmp/` |
| 2026-08-06 | 增补：概念表；能力袋命名备选（淡化 Host）；Layout A–E；微优化分轨（stats 双遍实证） |
| 2026-08-06 | Layout A：废弃 `capsule/`；`<AGG>` 改推 `state/`/`kit/`/`hub/`/`capabilities/`/`inner/` |
| 2026-08-06 | 澄清：`bash`/`export` 不属 agent core；移出 `<AGG>`（bang / 会话搬运） |
| 2026-08-06 | 增补 §14：对照 `ai-agent-book`（Agent=LLM+上下文+工具 / Model+Harness；Coding Agent 七工具+文件系统）判定 `agent/` 内外边界 |
| 2026-08-06 | 增补 §15：采纳 `agent::core`（循环+上下文+工具表）vs 外层工程权衡；渐进迁移、与 `app::core` 撞名处理 |

---

## 15. 推荐迁移策略：`agent::core` vs 外层 `agent`（渐进）

> 回应：「Agent core = 循环 + 上下文 + 工具表」很深刻；全盘 Layout A 成本/风险高；希望 **`agent::core` 钉公式核**，外层 `agent` 承担工程权衡。

### 15.1 判断：这个切法好不好？

**好，而且比一次改完 Layout A 更适合本仓。**

| 维度 | 说明 |
|---|---|
| 概念 | 与书公式同构；外层 ≈ Harness/产品工程权衡（bang、导出、装配门面） |
| 风险 | 可先「立核再迁入」，旧路径暂留；每步可 `just qa` |
| 成本 | 不必一次改名 session→state、搬尽文件 |
| 纪律 | core 边界写死后，再往里塞 bang/export = 明显违规（比散装 Layout 更好守） |

注意：书里整个 Harness 很大；**这里的 `agent::core` 刻意更窄**——只钉「循环 + 上下文 + 工具表」，不是把 infra 也吞进来。

### 15.2 两层心智

```text
                    ┌─────────────────────────────────────┐
  高维 / 工程权衡    │  crate::agent（外层）                 │
                    │  · Builder / 对外 facade             │
                    │  · bang (!cmd)、export/import        │
                    │  · 过渡期仍胖的 AgentCapabilities    │
                    │  · 尚未迁入 core 的旁路              │
                    │         │ 依赖 / 委托                 │
                    │         ▼                             │
                    │  ┌─────────────────────────────┐     │
  公式核（窄）       │  │  agent::core                  │     │
                    │  │  循环 + 上下文 + 工具表         │     │
                    │  └─────────────────────────────┘     │
                    └─────────────────────────────────────┘
                              │ ports only
                              ▼
                     protocol / infra / bridge
```

| 层 | 路径 | 装什么 | 不装什么 |
|---|---|---|---|
| **核** | `crate::agent::core` | ReAct 循环；`project_for_llm` + ContextPolicy（+ 渐进纳入 prompt/compaction）；ToolSet/freeze/validate + 批调度；**循环必需**的窄状态（model bind、steer 队列、session_id） | bang、export、HTML 渲染、trust、HTTP、工具进程实现 |
| **外** | `crate::agent` 其余 | Builder；过渡 God facade；session_io（bang/export）；对外 `pub use`；工程便利 API | 不应再发明第二套折叠/第二套循环 |

口头：**「Agent（产品/工程）包含 Core（公式）；Core 不包含 bang/导出。」**

### 15.3 与 Layout A / `state/` 的关系

不必二选一：

- **终局**仍可是 Layout A（`turn` + `history` + `tools` + 窄 `state`…），但都挂在 **`agent::core::`** 下。
- **现在**不必先定 `state/` vs `kit/`：先立 `core/` 边界，内部目录可第二波再美化。
- 外层继续叫 `session/` 一阵子也可以——只要 **新代码** 往 core 长，旧旁路标明「外层/待剥离」。

### 15.4 与 `app::core` 撞名

| 路径 | 含义 |
|---|---|
| `crate::app::core` | 应用组合根 / Driver seam（面装配） |
| `crate::agent::core` | 对话公式核（循环+上下文+工具表） |

路径不同，**可共存**。文档/注释一律写全路径。若口头易混，备选模块名：`agent::kernel` / `agent::engine`（不如 `core` 贴公式）。**默认仍用 `agent::core`。**

### 15.5 渐进相位（低风险顺序）

**Phase C0 — 立核（几乎零行为）**

1. 新增 `src/agent/core/mod.rs`：文档钉死公式；`pub use` 暂指向现有 `runtime` / `llm_project` / `tools` / `context_policy`（**先不搬文件**）。
2. `agent/mod.rs` 写明：稳定心智入口 `agent::core`；旁路仍在外层。
3. 闸：`just qa`。

**Phase C1 — 调用方改口（机械）**

- 新代码 / 能改的 import：`crate::agent::runtime` → `crate::agent::core::…`（via re-export）。
- 不改行为。

**Phase C2 — 物理迁入（一次一类）**

建议顺序（每步单独 commit）：

1. `history`（`llm_project` + tool_quiet）→ `core/history/`
2. `context_policy` → `core/context/`
3. `tools`（ToolSet/freeze）→ `core/tools/`（或 `core` 再导出）
4. `runtime` → `core/turn/` 或 `core/runtime/`
5. 从 Capabilities **抽出**循环热路径窄状态 → `core/state/`（可仍由外层 facade 持有并 `Deref`/委托）

**Phase C3 — 剥离破坏核的东西（外层显式化）**

- `bash`（bang）、`export` → `agent::session_io` 或 Driver 直调；**禁止**进 `core`。
- 外层 facade 变瘦：只剩装配 + 旁路 + 委托 core。

**Phase C4 —（可选）美化**

- core 内再 Layout A 命名；外层删 `session` 谎言；specs 后置压缩。

**硬纪律**：结构 commit ≠ 微优化 commit（§5）；不碰折叠串 / thinking replay。

### 15.6 Core 边界清单（门禁用）

**允许进入 `agent::core`**

- [ ] 循环（ReAct / 事件流 / retry / tool_batch|exec）
- [ ] 上下文投影与策略（`project_for_llm`、ContextPolicy；prompt/compaction 可渐进）
- [ ] 工具表（集合、冻结、校验、调度）——不是工具实现
- [ ] 循环必需状态（当前模型绑定、队列、session_id）

**禁止进入 `agent::core`**

- [ ] bang / `!cmd`
- [ ] session export/import HTML/JSONL
- [ ] trust 决策 UI/持久化
- [ ] infra 工具实现、JSONL store、MCP 客户端、provider HTTP
- [ ] TUI/Print 渲染

PR 自检：若 diff 往 `core/` 加了「面专属旁路」，驳回。

### 15.7 过渡期类型怎么摆

| 策略 | 做法 | 适合 |
|---|---|---|
| **A. 外胖内瘦** | 保留 `AgentCapabilities` 在外层；内嵌/委托 `core::State`；逐步搬家 | **推荐默认** |
| **B. 改名并行** | 新 `AgentState` 在 core；旧名 type alias | 想早用新名 |
| **C. 一次剪裁** | 同时瘦 facade + 迁 core | 风险高，不作第一刀 |

Driver / embed 继续打外层 `Agent`/`Capabilities`；待 C3 后再收口 `pub use`。

### 15.8 小结（拍板用语）

> **采纳双层：`agent::core` = 循环 + 上下文 + 工具表；外层 `agent` = 工程权衡与产品旁路。**
> 用 re-export 立核 → 迁入 → 剥离 bang/export，避免大爆炸重构。
> Layout A 作为 core **内部**终局目录可选，不阻塞开工。

### 15.9 落地进度

| 相位 | 状态 | 说明 |
|---|---|---|
| **C0 立核** | **done（本会话）** | `src/agent/core/mod.rs` 钉公式边界 + re-export；`agent/mod.rs` 双层叙事；未搬文件。**满闸 `just qa` / `doc-check` 延后到迁移完成**（已知：rustdoc 对私有 `app::core` 的 intra-doc 链会红——迁完后用正当方式解，不改文案迁就闸） |
| C1 调用方改口 | pending | 新代码优先 `agent::core::` |
| C2 物理迁入 | pending | history → context → tools → turn → 窄 state |
| C3 剥离 bang/export | pending | 外层 `session_io` 或 Driver |
| C4 美化 | pending | core 内 Layout A 命名可选 |

**跨机器进度板 / 终局讨论稿**：[`_HANDOFF/final-refactor-agent-module.md`](../_HANDOFF/final-refactor-agent-module.md)（以 HANDOFF 勾选为准；本文件为长调研）。
