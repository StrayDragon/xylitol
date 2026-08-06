# HANDOFF：`agent/` 重构 — 自包含讨论草案 + 续作清单

> **地位**：跨机器续作的**完整草案**（讨论 + 可选落地步骤）。**不是** `src/AGENTS.md` 规范。
> **本会话约定（2026-08-06）**：**只讨论、梳理**；**不再在本机继续改代码**。真正搬迁在其他机器按本文执行。
> **长调研（附录）**：同目录 [`agent-module-reorg-research.md`](./agent-module-reorg-research.md)（已从 `.tmp` 挪入以便 share）；**无该文件时仍可只靠本文 + 代码**开工。
> **闸**：过程以 `cargo check` 为主；**满闸 `just qa` 放迁移收尾**；不为 doc-check 削弱文档。

---

## 0. 换机怎么用

```bash
cd /path/to/xylitol
eval "$(just cargo-wt-env)"
git status && git log -3 --oneline
# 1) 读本文 §1–§4（概念与分组讨论）
# 2) 把 §3 各组「决议」填完（或接受建议默认）
# 3) 若工作区已有未提交的 agent::core（C0 雏形）→ 见 §5.0：保留作起点或 git restore 丢掉再按决议重做
# 4) 按 §5 TODO 执行；每步 cargo check；满闸留 §5 收尾
```

---

## 1. 问题与目标（我们在治什么）

### 1.1 病征（代码事实）

- `session/` 名实不符：里面是 **God `AgentCapabilities`（~61 pub 方法）**，不是「持久化会话」。
- `runtime/react.rs` 体量大；与 `session` **生产环依赖**。
- 产品旁路（**bang `!cmd`**、**export/import**）焊进能力袋，污染「对话核心」阅读。
- 顶层孤儿：`text.rs`、`tool_result_quiet.rs`；`trust.rs` 自承无生产调用。

### 1.2 目标（产品/分层仍遵守）

- 单 crate；`agent` ↛ `infra`（生产）；Env 折叠只在 `project_for_llm`；业务只认 `XyModel`。
- 阅读时能回答：循环在哪、上下文怎么进模型、工具表谁管、旁路产品能力在哪。
- **渐进**：可一组一组迁；允许过渡期双路径 `pub use`。
- **Coding Agent 主形态**（与 ai-agent-book 同构）：循环 + 上下文 + 工具表 + 文件系统（落盘在 infra）；TUI/Print 是面。

### 1.3 公式（讨论用语）

```text
对话公式核 ≈  ReAct 循环  +  上下文（投影/策略/提示…）  +  工具表（集合/冻结/调度）
产品/工程壳 ≈  bang、导出、Builder 门面、过渡期胖 facade、尚未归位的旁路
```

是否用目录名 `agent::core` **本身已降级为可选甚至不推荐默认**——见 **§8 策略重审**（不强制拆 core / 并行新建替代）。

---

## 8. 策略重审：不强制 `core`、以及「新建 `agents/` 替代」

> 2026-08-06 用户直觉：硬拆 `agent::core` **很快会再出问题**；`agent/` 心智应是 **上下文 + 工具 + LLM 处理**；可考虑 **先不改面，并行重写一个新模块再替换**。

### 8.1 你的直觉映射（建议采纳为模块使命）

```text
agent/（或替代名）的主责
  ├── 上下文：普通（投影/policy）+ 扩展（prompt/compaction）——都在本层，用目录区分深浅即可
  ├── 工具表：capabilities/tools（调度）；实现仍在 infra
  └── LLM 处理：选模绑定 + 调 XyModel + 吃流式 chunk → 写回轨迹
                （不是 HTTP/SSE；那是 bridge/infra）
```

循环（ReAct）是「LLM 处理」的主循环，不是第四个并列上帝。
bang/export/trust UX → **app**（已决）——本来就不该进这个使命。

**不必再套一层叫 `core` 的目录**：使命本身就是「公式」；再套 core 容易变成「核里的核 / 壳里的壳」，半年后又 God。

### 8.2 若不强制拆 `core`，怎么设计才好？

**策略 S — 平铺领域目录（推荐默认改道）**

- **不做** `agent::core/` 子模（或仅文档提及公式，无强制 import 路径）。
- `agent/` 顶层按领域平铺，**每目录一句 MUST/MUST NOT**（可写在各 `mod.rs`）：

```text
src/agent/                         # 使命：上下文 + 工具表 + LLM 循环
├── turn/                          # ReAct + queue（G1+G5）
├── context/
│   ├── messages/                  # project_for_llm + quiet（G2）
│   └── policy/                    # ContextPolicy（G3）
├── capabilities/
│   └── tools/                     # ToolSet/freeze（G4）；预留其它子能力
├── prompt/                        # 扩展上下文（G7）
├── compaction/                    # 扩展上下文（G8）
├── model/                         # 选谁 / thinking（G6，待决细节）
├── builder.rs                     # 装配
└── （无 session/ 谎言；无 bang/export）
```

- 外层 **瘦 facade**（现 Capabilities）只做「持有 turn 所需状态」，或最终删掉拆成几个协作对象。
- **纪律**：靠目录名 + `//!` 门禁，不靠第二命名空间。
- **迁移**：仍可一组一组搬；旧 `runtime/`/`session/` 逐步改名；**可丢掉已有 C0 `core/` 雏形**。

| 优点 | 缺点 |
|---|---|
| 少一层抽象；与「agent=公式」同构 | 顶层目录稍多 |
| 不会「core 又变胖」 | 要靠 review 守门禁 |
| 与已决 G\* 命名自然相接 | God Capabilities 仍要另案瘦 |

### 8.3 策略 G — 并行新建「干净 agent」再替换（你提的 rewrite）

**先不改 TUI/Print 界面**；Driver 仍吃同一套 `XyEvent` / 队列 API；背后换实现。

```text
src/agent/          # 旧：继续跑产品（只修 blocker）
src/agent_/         # 新：按 §8.1 使命重写（临时名，已决）
app/core            # 组合根开关：旧|新；或测里双跑
```

**命名注意**：

| 名字 | 评价 |
|---|---|
| **`agent_/`**（已决） | 临时、一眼可分；完成后批量 rename → `agent/` |
| `agents/`（复数） | 易与「多 agent」/产品 AGENTS.md 混淆，**不推荐** |
| `agent_next/` / `agent2/` | 也可，不如 `agent_` 短；未选 |
| `dialogue/` / `react/` | 表意循环，但丢「工具+上下文」 |
| 仍叫 `agent/`，新树在分支上重写后一次性替换 | 也行，但并行期难 |

**范围（新模块应有）**：turn + context/{messages,policy} + capabilities/tools + prompt + compaction + model bind + 窄状态；经 ports 调 `XyModel`/`XySessionStore`/`XyTool`。

**范围（新模块不应有）**：bang、export、trust UI、infra 实现、TUI。

**接法**：

1. 新模块实现「能跑一轮 ReAct + 投影 + 工具批」的最小闭环（Fake provider）。
2. 挂 `AgentBuilder`-兼容或新 builder → Driver 测切换。
3. 黄金测：c1925/c1930 / `project_for_llm` / prefix idempotency **必须**先在新树复现。
4. 面零改或只改组合根一行开关。
5. 旧 `agent/` 删或缩成 re-export。

| 优点 | 缺点 |
|---|---|
| 不受 God Capabilities / 环依赖绑架 | **双轨维护**一段时间 |
| 边界一次做对 | 易漏边角（hooks、MCP freeze、steer） |
| 面可不动 | 工作量大；需强黄金测护栏 |
| 可丢弃 C0 core 实验 | 需要明确「完成定义」才切流量 |

### 8.4 策略对比（怎么选）

| | S 平铺重构 | G 并行重写 | 旧案：强制 `agent::core` |
|---|---|---|---|
| 抽象层数 | 少 | 少（新树内） | **多一层，易再 God** |
| 与「上下文+工具+LLM」直觉 | **贴** | **贴** | 核/壳分裂，壳仍乱 |
| 短期风险 | 中（搬迁） | 中高（双轨） | 低起步、长期概念债 |
| 样板代码 | 低 | 新树完整写一遍 | re-export 低，心智贵 |
| 推荐 | **默认改道** | 若旧树搬迁痛到不想碰 | **降级 / 可废弃 C0** |

### 8.5 若选 G：最小「完成定义」

- [ ] Fake 下跑通：用户轮 → 工具批 → 助手轮 → 事件序列与旧语义对齐（关键子集）
- [ ] `project_for_llm` 折叠串 + assemble 前缀黄金测绿
- [ ] thinking full-replay / encrypted 相关测绿（经 bridge，不改行为）
- [ ] Driver `run`/`abort`/steer/follow-up 经新实现
- [ ] bang/export **不在**新模；仍走 app
- [ ] 面无视觉回归要求（本波）
- [ ] 切流后旧 `agent` 删除或只剩薄 re-export

### 8.6 决议栏

| # | 问题 | 决议 |
|---|---|---|
| S0 | 是否 **放弃强制 `agent::core` 目录** 作为默认？ | **是**（2026-08-06） |
| S1 | 主策略 | **G 并行重写**（2026-08-06）：新树按 §8.1 使命；面尽量不动；接好调用与测后再切流 |
| S2 | 新模临时目录名 | **`src/agent_/`**（2026-08-06）：刻意临时；**全部重写 + 接线 + 测试通过后，再批量 rename → `agent/`**（或当时终名） |
| S3 | 本机已有 `src/agent/core/` 雏形 | **废案**：他机可 `git restore` / 删；不并进 `agent_/` |
| S4 | 共享纯函数落点 | **T2 `src/util/`**（2026-08-06）：`util::xml::…`；见 §8.9 / G13 |
| S5 | 终层名：`agent` vs 顶栏 `core`？ | **待讨论**（§8.10）；你倾向「`agent`→`core`，因整仓即 xylitol agent」 |

### 8.7 并行重写操作草图（选 G + `agent_/`）

```text
src/
  common/           # 新建：xml_escape 等纯共享（无向上依赖）
  agent/            # 旧：产品仍跑；只修 blocker；逐步停写新功能
  agent_/           # 新：turn + context + capabilities/tools + prompt + compaction + model…
  app/              # 面不动；组合根晚些增加「走 agent_」开关或测专用装配
  infra/ · protocol/
```

**完成后再做的批量重命名（单独窗口）**：

1. 确认 `agent_/` 已接 Driver/测、黄金测绿、旧路径无唯一依赖
2. 旧 `agent/` → `agent_legacy/` 或直接删
3. `agent_/` → `agent/`（`git mv` + 全仓改 path）
4. 满闸 `just qa`

**为何要临时名**：并行期两套并存时，`agent` vs `agent_` 在路径上一眼可分；避免半迁移时 `agent` 名实不清。

**Rust 注意**：目录名 `agent_/` ↔ `mod agent_;`（或 `#[path]`）；合法，略丑，正是「临时」信号。

### 8.8 `agent_/` 目录 + 公开 API 草图（讨论稿）

> 目标：面（TUI/Print）仍只认 **mod 级** `crate::agent_*::{…}`（或切流后的 `crate::agent`）；组合根可 deep-import。
> **不进新树**：bang、export、trust UX（→ app）；工具进程/HTTP/JSONL（→ infra）。

```text
src/agent_/
  mod.rs                 # 唯一「库用户入口」：精选 pub use
  builder.rs             # 装配；最终产出 Runtime（+ 窄状态）

  turn/                  # 循环 + 队列（G1+G5）
    mod.rs               # AgentRuntime, AgentHooks, XyEventStream
    react.rs
    tool_batch.rs · tool_exec.rs · hooks.rs · retry.rs · obs.rs
    queue.rs             # PendingMessageQueue, QueueMode, QueueStats
    # script_hook_ctx 等可留 turn/ 或旁路 app 若只服务面

  context/               # 普通上下文（G2+G3）
    mod.rs
    messages.rs          # project_for_llm + tool_result_quiet（今日 llm_project）
    policy.rs            # ContextPolicy, DatePlacement, …

  capabilities/
    tools/               # 工具表（G4）— 不是工具实现
      mod.rs             # ToolSet, freeze, fingerprint, MCP gate const

  prompt/                # 扩展上下文 · 系统提示 / skills / slash 词表
  compaction/            # 扩展上下文 · 压缩编排（经 XyModel，无 HTTP）
  model/                 # registry / manager / resolver / bind（G6 待钉）

  # 过渡期可有窄 state facade（G14 待钉），禁止再叫 session/ 装旁路
```

#### 建议 mod 级公开面（面 / embed / 多数测）

| 符号 | 来自 | 说明 |
|---|---|---|
| `AgentRuntime` | `turn` | ReAct 入口 |
| `AgentBuilder` | `builder` | 组合根装配 |
| `AgentHooks` / stop hooks / `BeforeToolHook` | `turn` | 钩子 |
| `XyEventStream` | `turn` | 事件流类型别名若仍要 |
| `PendingMessageQueue` / `QueueMode` / `QueueStats` | `turn::queue` | steer/follow-up |
| `project_for_llm` | `context::messages` | 黄金钉 |
| `ContextPolicy` + 枚举 | `context::policy` | 请求布局 |
| `ToolSet` / freeze / fingerprint / `MCP_FIRST_TURN_GATE_TIMEOUT` | `capabilities::tools` | 工具表 |
| `XyEvent` / `AgentMessage` | re-export protocol | 词汇表便利 |
| `build_system_prompt` / product slash | `prompt` | 组合根 / product_commands 今日路径 |
| Compaction settings / estimate | `compaction` | bootstrap / Driver types |
| ModelRegistry 等 | `model` | bootstrap |

#### 刻意 **不** 在 mod 级长期暴露（或迁出后消失）

| 今日 | 去向 |
|---|---|
| `session::export` / bang | **app** |
| `session::trust::save_trust_decision` | **删**；infra + Driver |
| `text::xml_escape` | **叶模**（见 §8.9），不挂在 agent_ |
| God `AgentCapabilities` 全方法袋 | G14：瘦成窄状态或改名；热路径字段进 turn/context |

#### 组合根（`app::core`）允许 deep-import 的例子

`agent_::model::registry`、`agent_::prompt::…`、`agent_::turn::script_hook_ctx`——与今日纪律一致：**面禁止 reach 子模**。

#### 与旧树对照（迁移动作心智）

| 旧 | 新（`agent_/`） |
|---|---|
| `runtime/*` | `turn/*` |
| `session/queue.rs` | `turn/queue.rs` |
| `llm_project` + `tool_result_quiet` | `context/messages` |
| `context_policy` | `context/policy` |
| `tools/*` | `capabilities/tools` |
| `prompt` / `compaction` / `model` | 同名 |
| `session/{bash,export,trust,stats,mod God}` | 拆：bash 执行口仍经 port；export/bang→app；trust 封装删；stats→G11；God→G14 |

### 8.9 共享纯函数落点：Rust 习惯 vs 本仓

**社区共识（API Guidelines / Pragmatic Rust Guidelines 精神）**：

1. **按用例 / 领域命名**，不要按技术垃圾桶命名（反例：`traits/`、`errors/`、`utils/` 大杂烩）。
2. 只有一两个函数时：**就近**或 **窄域名模** 优于开抽屉。
3. 真要抽屉：生态里多见 **单数 `util`** + **子域文件**（`util::xml`），少见 Java 风 `common`；复数 `utils` 最易变垃圾场。
4. **不要**为几个纯函数再拆 workspace crate（本仓 AGENTS 也禁止为分层乱拆 crate）。

| 候选 | 例 | 评价 |
|---|---|---|
| **域名模** `text` / `xml` | `crate::text::xml_escape` 或 `crate::xml::escape` | **最 Rust**；今日只有 escape 时最优。`text` 与旧 `agent/text.rs` 同名心智 |
| **`util`**（单数）+ 子模 | `crate::util::xml::escape` | 抽屉可扩展；比 `common`/`utils` 更常见；**须**按域分子文件 |
| **`common`** | `crate::common::xml_escape` | 可用；偏 Go/Java；**必须硬门禁**否则必脏 |
| **`shared` / `helpers`** | — | 同 common，无额外信息 |
| **`utils`（复数）** | — | **不推荐** |
| **塞进 `protocol`** | — | **否**：protocol 是 wire/ports/类型，不是字符串帮手 |
| **infra** | — | **否**：实现层，不是纯帮手 |

**门禁（无论叫什么）**：

- 叶依赖：仅 `std`（或极少已是叶的 crate）；**禁止** `agent`/`app`/`infra`/业务 `protocol` 类型
- 只放 **纯函数 / 无状态小类型**；有 I/O、配置、端口 → 滚出去
- 新增条目要想域名：优先 `util::<domain>`，禁止往根上平铺 50 个 `fn`

**修订建议（待你拍）**：

| 选项 | 含义 |
|---|---|
| **T1** | 先只建 **`src/text.rs`**（或 `text/mod.rs`），放 `xml_escape`；没有第二域就不开抽屉 |
| **T2** | 建 **`src/util/`**，`util/xml.rs` | **已决**（2026-08-06） |
| **T3** | 坚持 **`src/common/`** | 未选 |

> 先前 G13 曾写 `common`；**已改口 T2**。

### 8.10 终层名：还叫 `agent`，还是顶栏改成 `core`？

**你的直觉**：`src/` 整体就是 xylitol 这个 coding agent 产品；中间层再叫 `agent` 有点套娃；干脆 `src/agent` → `src/core`，表示「业务核 / 公式层」。

**分层不会因为改名消失**——无论叫什么，依赖仍是：

```text
app → ??? → protocol
 ↓     ↑
 └──── infra
```

`???` 今日叫 `agent`；提案叫 `core`。

| | 继续叫 `agent` | 改成顶栏 `core` |
|---|---|---|
| 表意 | 「ReAct/编排层」业界常见 | 「业务核」；产品名与层名解耦 |
| 与「整仓是 agent」 | 层名=产品隐喻，略冗余但清晰 | 层名=架构角色，更贴 clean/hex |
| **硬伤** | 无新伤 | **已有 `app::core`（Driver/组合 seam）** → 双 core：`crate::core` vs `crate::app::core` |
| 文档/测/import 成本 | 低（维持） | 高（全仓 rename + AGENTS + specs 前缀） |
| 并行临时名 | `agent_/` → `agent/` | 可 `core_/`→`core/`，或新树直接 `core/`、旧 `agent/` 过渡 |

**若坚持顶栏 `core`，必须同时处理 `app::core`**（否则比现在更糊）：

| 子选项 | 做法 |
|---|---|
| **C-a** | 顶栏 `core` + **`app::core` → `app::seam`**（或 `app::driver` / 并进 `app` 根） |
| **C-b** | 顶栏 `core` + 保留 `app::core`，文档永远写全路径——**不推荐**（口头必撞） |
| **C-c** | 不叫 `core`：顶栏改 **`engine` / `runtime` / `harness`**——避开双 core，仍表达「核」 |

**建议（讨论用，未决）**：

1. 产品心智「整仓是 agent」**对**；不必因此强迫中间层也叫 agent。
2. 但 **裸改 `agent`→`core` 而不动 `app::core`** 会更糟。
3. 若要「核」字样：优先 **C-a**（顶栏 `core` + seam 改名）或 **C-c**（`engine`）——二选一比半吊子双 core 干净。
4. 并行期临时名跟着终名走：终名 `core` → 临时 `core_/`（或直接 `core` 双轨）；终名仍 `agent` → 维持 `agent_/`。

**与策略 G 的关系**：rename 终名是 **切流后的批量 rename 目标**；现在拍板的是「新树最终叫什么」，不是立刻 `git mv agent core`。

## 2. 方案族与「会不会被迫多写很多代码」

### 2.1 三种力度（成本预估）

| 方案 | 做法 | 新增样板 | 改 import 面 | 行为风险 | 推荐场景 |
|---|---|---|---|---|---|
| **L0 叙事-only** | 只改 `//!` / AGENTS 短述；不搬目录 | ≈0 | 0 | 极低 | 仅文档对齐 |
| **L1 门面核（低成本）** | 可选 `agent::core` **只 re-export**；旧路径全留；新代码走核路径 | **~50–80 行**（一个 mod.rs） | 可选、可慢改 | 极低 | **默认推荐起步** |
| **L2 物理迁入** | 文件搬进 `core/history|turn|…`；顶层 `pub use` 兼容一波 | 中：路径机械改 | 中高（可分 commit） | 低（纯搬） | L1 稳定后 |
| **L3 抽窄 State + 剥旁路** | Capabilities 委托；bang/export 外置 | 中：委托胶水 **估计 +100–300 行短时**，删 God 后净减少 | 中 | 中（API 形状） | 名实真正清理 |

**结论（预估，非承诺行数）**：

- **不会**因为「有个 core」就被迫把全仓改成新写法——L1 用 re-export，旧 `crate::agent::runtime::` 可长期共存。
- **会**多写的主要是：① 一个边界 mod 的文档；② 搬文件时的路径替换；③（若做 L3）facade 委托方法。
- **真正贵**的是一次改名+剥旁路+改 Driver 调用，不是「多一层 core 模块」本身。
- 若担心样板：可采用 **「逻辑 core、物理仍平铺」**——用文档 + 目录约定标记「公式核文件」，**暂不建 `core/` 目录**（见 §4 选项 B）。

### 2.2 `agent::core` 目录要不要？

| 选项 | 含义 | 样板 | 纪律 |
|---|---|---|---|
| **A. 要 `agent::core/`** | 公式核有硬路径；塞 bang 一眼违规 | L1 极低 | 强 |
| **B. 不要子模，只划「核文件清单」** | `runtime/`+`llm_project`+`tools`+… 标为核 | ≈0 | 靠约定，易再 slop |
| **C. 外层改名 `harness`/`shell`，核仍叫 `agent`** | 颠倒命名 | 大 | 易与书 Harness/infra 混淆，**不推荐** |

**草案默认（已随 §8 改道）**：选 **B / 策略 S（平铺、无 core 伞）**；强制 A（`agent::core/`）**不再作为默认**。

### 2.3 与 `app::core` 撞名

路径不同（`agent::core` vs `app::core`）。文档写全路径。rustdoc 私有链问题 **收尾正当解**，过程不为过闸改文案。

---

## 3. 分组讨论：每项「留 agent / 迁出 / 进公式核？」

> **用法**：逐组填「决议」。建议默认仅作起点。
> **迁出** = 离开 `src/agent/`（去 `app` / `infra` / 新薄模），不是塞进 `core`。

### G1 — ReAct 循环（`runtime/`）

| | |
|---|---|
| 今日 | `AgentRuntime`、`react.rs`、hooks/retry/tool_batch/tool_exec/obs |
| 公式 | **核 · 循环** |
| 建议 | **留 agent**；属公式核。物理上 → `core/turn` 或保留 `runtime` 名 |
| 迁出？ | **否**（面只经 Driver 调） |
| 决议 | **OK — 留 agent · 属公式核**（2026-08-06） |

### G2 — 历史投影（`llm_project.rs` + `tool_result_quiet.rs`）

| | |
|---|---|
| 今日 | Env→LLM 折叠；write/edit 安静化进历史 |
| 公式 | **核 · 上下文** |
| 建议 | **留 agent**；归 `history/`（在核内）。**禁止**第二折叠路径 |
| 迁出？ | **否**（c1930 钉在此） |
| 决议 | **OK — 留 agent · 属上下文**（2026-08-06） |
| 命名意向 | 可落到 **`context/messages`**（投影/安静化）；与 G3 并列 `context/policy`。 |

### G3 — ContextPolicy（`context_policy/`）

| | |
|---|---|
| 今日 | tools/status_bar/date 钩子默认 |
| 公式 | **核 · 普通上下文 · 策略** |
| 建议 | **留 agent**；与 G2 同树 |
| 迁出？ | **否** |
| 决议 | **OK — `context/policy`，与 G2 组成 `context/{messages,policy}`**（2026-08-06） |

### G4 — 工具表（`tools/` ToolSet/freeze/validate）

| | |
|---|---|
| 今日 | 表 + 冻结闸；**不是** read/bash 实现 |
| 公式 | **核 · 工具表** |
| 建议 | **留 agent**；实现仍在 `infra/tools` |
| 迁出？ | **否**（调度在 agent，执行在 infra） |
| 决议 | **OK — 留 agent · 属工具表**（2026-08-06） |
| 命名 | **`capabilities/tools`**（复数 capabilities，可扩展其它 capability 子树；接受路径稍长）。避免单数 `capability/` 与类型 `AgentCapabilities` 口语混淆。 |

### G5 — 插话/续跑队列（`session/queue.rs`）

| | |
|---|---|
| 今日 | steer / follow-up，挂在 Capabilities |
| 公式 | **核 · 循环改道** |
| 建议 | **留 agent**；物理靠近循环（`turn`/`runtime` 旁），勿再挂在 `session/` 谎言目录 |
| 迁出？ | **否** |
| 决议 | **OK — 属循环**（2026-08-06）；目录随 G1 的 turn/runtime 安置 |

### G6 — 模型选择/thinking（`model/` + Capabilities 上方法）

| | |
|---|---|
| 今日 | registry/manager/resolver；HTTP 在 infra/bridge |
| 公式 | **核旁 · 绑定「选谁」**；发请求不是 agent |
| 建议 | **留 agent**（编排）；或核内窄 `model_bind` + `model/` 子树 |
| 迁出？ | **否**（manifest 加载若纯 IO 可再议，今日可留） |
| 决议 | _待定_ |

### G7 — 系统提示 / skills / slash 名（`prompt/`）

| | |
|---|---|
| 今日 | `build_system_prompt`、skill expand、commands |
| 公式 | **扩展上下文**（算进「上下文工程」，但**不进窄核**） |
| 建议 | **留 agent 外层**；资源发现仍 infra |
| 迁出？ | **否**（组装留 agent；发现留 infra） |
| 决议 | **OK — 算上下文家族，但不进 core**（2026-08-06）。窄核只要「普通上下文」= messages 投影 + policy |

### G8 — 压缩编排（`compaction/`）

| | |
|---|---|
| 今日 | cut/摘要/orchestrator；写盘经 store port |
| 公式 | **扩展上下文**（同上） |
| 建议 | **留 agent 外层**；JSONL 细节不进 agent |
| 迁出？ | **否** |
| 决议 | **OK — 算上下文家族，但不进 core**（2026-08-06） |

### G9 — Bang `!cmd`（`session/bash.rs`）

| | |
|---|---|
| 今日 | 用户 bang → Driver → Capabilities.execute_bash；**不经 ReAct** |
| 公式 | **非核** |
| 建议 | **迁出公式核**；候选：`agent::session_io` 或 **下沉 `app`/Driver** |
| 决议 | **下沉 app**（2026-08-06）：视为 **TUI/面专用旁路**；Driver（及 Print 若仍暴露 bang）直调 `XyBashExecutor` + 写 store。agent 不再持 `BashExecHandler` / `execute_bash` 门面。 |
| 注意 | 写 Env bash 条目的小函数可：① 留 `protocol`/`infra` 旁纯函数；② 或短暂 `pub(crate)` 在 app——**不要**为 bang 再留 agent 子系统 |

### G10 — Export/import（`session/export.rs`）

| | |
|---|---|
| 今日 | HTML/JSONL；`XyExportIo` |
| 公式 | **非核** |
| 决议 | **下沉 app**（2026-08-06）：slash 产品能力，Driver/面侧接线。`render_*`/`parse_*` 纯变换可随迁到 `app` 或贴近 `infra` IO；**不进公式核** |

### G11 — SessionStats / 双遍 count（`session/stats.rs`）

| | |
|---|---|
| 今日 | 读 store 数 user/assistant（**两遍 filter**） |
| 公式 | 弱相关诊断 |
| 建议 | **不必进核**；可随 G9/G10 旁路或 app 诊断；**perf 单遍单独 commit** |
| 决议 | _待定_ |

### G12 — Trust helper（`session/trust.rs`）

| | |
|---|---|
| 今日 | `save_trust_decision`；**无生产调用**（注释自承）。真路径：TUI `/trust` → Driver → **infra trust store** |
| 代码事实 | `rg`：agent 仅定义/re-export；`app/tui` + `app/core/driver` 走 Driver API |
| 建议 | **删 agent 薄封装**；逻辑留 **infra::trust** + **app**（slash/Driver）。不是「搬到 agent 别处」 |
| 决议 | **倾向删 agent 侧 / 留 infra+app**（2026-08-06，待实施时确认无遗漏调用） |

### G13 — `text::xml_escape`

| | |
|---|---|
| 今日调用方 | ① `agent/prompt/system.rs` ② `session/export.rs`（将随 G10 → app） |
| 放 `infra`？ | **不太合适**：infra 是 ports **实现**（HTTP、JSONL、工具进程、trust store），不是通用字符串帮手 |
| 放 `utils/`？ | **否**：泛 utils 易变杂物箱；根 AGENTS 也倾向避免 |
| 决议 | **T2 `src/util/`**（2026-08-06）：`util/xml.rs` → `crate::util::xml::escape`（或 `xml_escape`）；删 `agent/text.rs` |
| 门禁 | `util` **禁止**依赖 `agent`/`app`/`infra`/业务类型；只允 std / 极少叶依赖；**按域分子模**，禁止根上平铺杂 fn |

### G14 — God `AgentCapabilities`（`session/mod.rs`）

| | |
|---|---|
| 今日 | 能力袋 + 旁路方法集合 |
| 建议 | **留 agent 工程壳**作过渡 facade；热路径委托公式核状态；**改名/瘦身最后做** |
| 目录 | 不要再用 `session/` 装它（`state/` 等，待定） |
| 决议 | _待定_ |

### G15 — Builder（`builder.rs`）

| | |
|---|---|
| 建议 | **留 agent 外层**（装配）；组合根仍在 `app::core` |
| 决议 | _待定_ |

### G16 — 明确永不进 agent 的（对照用）

| 机制 | 应在 |
|---|---|
| LLM HTTP/SSE | bridge + infra |
| read/write/bash/grep **实现** | infra/tools |
| Session JSONL / 树存储 | infra/session |
| MCP 客户端 | infra/mcp |
| Trust store | infra/trust |
| TUI/Print/Server UI | app |

---

## 4. 草案默认（随决议更新）

| 项 | 当前默认 / 已决 |
|---|---|
| **主策略（§8）** | **G 并行重写**；临时模 **`src/agent_/`**；完成后批量 rename；**放弃强制 `core/`** |
| **agent 使命** | **上下文 + 工具表 + LLM 编排（经 XyModel）** |
| **共享纯函数** | **T2 `src/util/`**（`util::xml`…） |
| **终层名（§8.10）** | **待拍**：维持 `agent` / 顶栏 `core`(+改 `app::core`) / 或 `engine`… |
| C0 `agent/core` | **已回滚**（mod 接线去掉；孤儿目录已清） |
| 满闸 | 切流/rename 后 `just qa` |

### 4.1 目标布局（并行期）

```text
src/
  util/                        # T2：xml 等纯共享（按域子模）
  agent/                       # 旧（过渡）
  agent_/ 或 core_/            # 新（临时名随 §8.10 终名）
    turn/
    context/{messages,policy}
    capabilities/tools/
    prompt/
    compaction/
    model/
    builder.rs
    mod.rs
  app/                         # bang · export · trust UX · 面；内含 app::core seam（若顶栏改 core 则 seam 宜改名）
  infra/ · protocol/
```

「普通 vs 扩展上下文」= **`agent_/` 内目录**（`context/messages` vs `prompt/`），**不再**套 `core/`。

---

## 5. 落地 TODO（他机执行；本会话不推进）

### 5.0 关于本机 C0 雏形

- [x] **已回滚**：`agent/mod.rs` 无 `pub mod core`；未跟踪的 `src/agent/core/` 已删除（2026-08-06）

### 5.1 讨论完成门闩（动刀前）

- [x] 主策略：**G + `agent_/`**；放弃强制 `core/`（§8.6）
- [x] G13 → `src/common`
- [ ] §3 剩余 G6/G11/G14/G15 决议列填完（或声明「接受 §4 默认」）
- [ ] 确认本波 **不做** 的项（避免范围膨胀）

### 5.2 实施清单（拍板后 · 并行重写）

**P0 — 脚手架**

- [ ] 新建 `src/util/`（T2）：`util/xml.rs`；`xml_escape`；删 `agent/text.rs` 调用改指
- [ ] 新建新树骨架（临时名随 §8.10：默认仍 `agent_/`，若终名 `core` 则改 `core_/` 或直接 `core/`）
- [ ] `lib.rs`：`pub mod common;` + `pub mod agent_;`（旧 `agent` 仍保留）
- [ ] `cargo check --all-features`

**P1 — 新树最小闭环**

- [ ] Fake provider 下一轮 ReAct + `project_for_llm` + 工具批
- [ ] 黄金：c1925 / c1930 / prefix idempotency **先在新树复现**
- [ ] G9 bang · G10 export · trust UX **进 app**（不进 `agent_/`）
- [ ] G12：删旧 agent trust 薄封装（时机可与切流一并）

**P2 — 接线切流**

- [ ] Driver / builder 开关或替换装配 → `agent_`
- [ ] 面（TUI/Print）尽量零改
- [ ] 收口 app 对旧 `agent::session::*` 的依赖

**P3 — 批量 rename + 收尾**

- [ ] 旧 `agent/` 删或 `agent_legacy` 过渡后删
- [ ] `agent_/` → `agent/`（`git mv` + 全仓 path）
- [ ] G14 瘦 facade / 改名（可与 rename 同窗或稍后）
- [ ] `just qa`
- [ ] c1925/c1930 相关测确认
- [ ] （可选）stats 单遍 `perf` commit
- [ ] specs 修正/压缩
- [ ] `src/AGENTS.md` 短摘要（若边界已稳）

### 5.3 微优化轨（随时，独立）

- [ ] stats 单遍 role count

---

## 6. 换机日志

| 何时 | 谁 | 笔记 |
|---|---|---|
| 2026-08-06 | 讨论会话 | 转为「只讨论」；HANDOFF 自包含草案 |
| 2026-08-06 | 讨论会话 | 回填 G1/G2/G4/G9/G10/G12/G13；调研 md 入 `_HANDOFF/` share |
| 2026-08-06 | 讨论会话 | 主策略改 **G + `agent_/`**；`common` 收 xml_escape；废强制 core |

---

## 7. 讨论记录（往下追加）

> 每组结论一句话即可；并回填 §3「决议」。

- 2026-08-06：调研稿挪至 `_HANDOFF/agent-module-reorg-research.md` 便于 share。
- 2026-08-06：G1/G2/G4 OK；G2 意向 `context/messages`；G4 意向 `…/tools`（讨论 `capability/tools` 撞名）。G9/G10 **下沉 app**。G12 删 agent 封装、真源 infra+Driver；G13 `xml_escape` 按调用方拆（prompt≠export）。
- 2026-08-06：G3 OK → `context/policy`。G5 属循环。G7/G8 **扩展上下文、不进窄核**（窄核=普通上下文）。G4 钉 **`capabilities/tools`**。G13：不进 infra；慎 utils；就近或窄名模。`agent/mod.rs` 注释改为「窄核三件套」白话。
- 2026-08-06：策略重审——**可不强拆 core**；使命=上下文+工具+LLM；对比 **S 平铺 vs G 并行重写**（慎名 `agents/`）。默认建议改道 **S**；C0 core 可废。待填 §8.6。
- 2026-08-06：拍板 **G 并行重写**；临时模名 **`agent_/`**（完成后批量 rename）；**`xml_escape` → `src/common`**（纯共享、无向上依赖）；放弃强制 `core/`；C0 废案。
- 2026-08-06：补 §8.8 `agent_/` 目录+公开 API 草图；§8.9 共享纯函数命名对照（T1 `text` / T2 `util` / T3 `common`）；S4/G13 改「待拍」，允许从 common 改口。
- 2026-08-06：**T2 `util` 拍板**；C0 `agent/core` 已回滚并清孤儿目录；新开 §8.10——是否 `agent`→顶栏 `core`（须处理与 `app::core` 双名；备选 `engine` / C-a 改 seam）。
