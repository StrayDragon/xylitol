# src/ 分层架构（代码架构 SSOT）

本文件是 `src/` **代码架构**的单一真值源：依赖方向、seam、`Xy*`、消息/工具边界、扩展开闭、复杂度与体量策略。全局工作方式见根 `AGENTS.md`；产品心智见 `docs/architecture/`。子目录 `AGENTS.md` 只写本面例外，不重复本文件。

**真值顺序**：代码与测试 → 本文件 → archive design。本文件写**稳定规则**，不写文件清单、change id 进度、超标表。模块落点以目录树为准。

## 分层（normative）

单 crate 逻辑分层（**不**为分层拆 crate；**禁止**再抽 `xylitol-domain` / 把 LLM 叶从 bridge 挪进主仓）：

```text
app → agent → protocol/{wire, ports, model, session, …}
  ↓     ↑         ↑
  └──── infra ────┘
         ↘
        utils   (叶：仅 std；各层可依赖；非 Xy* 稳定面)
```

| 层 | 做什么 | 硬约束 |
|---|---|---|
| `protocol` | 线协议 `wire` + 可替换口 `ports` + 跨层共享类型（根模块及 `model/`/`session/` 等聚类） | ↛ `agent`/`infra`；MAY 依赖 bridge **DTO only**；`wire` ↛ `ports`；`ports` ↛ `wire`；禁止再建 `domain`/`vocab`/`types` 第三顶栏；根下可按领域聚子树（`model/`、`session/`），**不是**新顶栏 |
| `agent` | ReAct / `capabilities`（能力聚合）/ 编排 / `project_for_llm` | ↛ `infra`；运行时能力在 `agent::capabilities`；持久化词表在 `protocol::session`；**不得**再扩 bang / export / 产品 slash 目录（属 app） |
| `infra` | ports 实现（provider、tools、session、config…）；vendor SDK 关在此层 | ↛ `agent` |
| `app` | 应用面 + `core` 跨面 seam | 走 seam，不 reach `agent`/`infra` 内部；**产品 slash / bang / session export** 在 `app/core` + `XyDriver`（面经 Driver，不进 capabilities） |
| `utils` | 纯叶工具（如 `xml_escape`、poison-tolerant mutex） | ↛ `agent`/`infra`/`app`/`protocol`；各层 MAY 依赖 |

### `AgentCapabilities` 目标面

引擎侧聚合体，**只**收敛这些职责（新能力先问是否属此列）：

| 留 | 不留（归 app / Driver） |
|---|---|
| model 选择与 thinking | 产品 slash 词表（`app::product_commands`） |
| tools + freeze / hooks / permission | bang `!` / `!!`（`app::bang_exec`） |
| session id + store + fork/stats | HTML/JSONL export-import（`app::session_export`） |
| context_policy + prompt 装配 | — |
| compaction 闸 | — |
| steer / follow-up queues | — |

Skill/extension slash 尚未交付：需要时在 **app / `XyDriver`** 侧注册并并入 `get_commands`，**不要**再塞回 `AgentCapabilities`。产品 builtin 列表只在 `XyDriver::get_commands` 组装。禁止 `agent` → `app`。

- **组合根**才同时 import `agent` + `infra` 做装配（`app/core` 与各面入口）。靠 review + 行为测守住；**禁止**源码 grep 元测试卡 import。
- **应用面**：只经 `crate::agent`（mod 级）与 `crate::app::core`；共享流水线 = 装配 → `XyDriver::run` → `XyEvent` 流 → 面渲染。不够就扩 seam（`l8ng-write-surface`），不绕过。
- **产品角色（client / host）**：TUI 等面是 client（键、画、TTY、编辑器、剪贴板）。模型 / 会话 / MCP / 工作区 / trust 是 host 操作器角色。默认 embed 两角色同进程、走同一条 Command/Event 语义；**禁止**把 host 等同于 HTTP 监听器。监听器是 `serve` 的后置职责。
- **进程内 Driver** 可调 `infra` 做 trust/clipboard/config 等表面能力；默认工具集 / provider / session 仍归组合根。面仍禁止 reach。剪贴板等面本地能力留在 client，不交给远程 host 写本机盘。
- **steer / follow-up / abort**：只经 `XyDriver` 队列 API；面不得改 ReAct 内部队列。产品语义：`docs/architecture/插话续跑与中止.md`。
- **`AgentRuntime` 会话 actor（硬约束）**：一个 runtime **显式绑定一个 session** 后才可根提交；任意时刻至多一个 ReAct worker 可写该 session（单飞）。根提交默认拒绝忙碌并发；`steer`/`follow_up` 是轮内改道，**不是**第二次根提交。`RunId` 仅运行时内部，不进 `XyEvent`/wire。未来子 agent = **另建隔离 runtime**，不得在同一 runtime 按 session id 多路复用，也不得共享 history / cancel / active turn / 插话队列。构造基线用可克隆的 `RuntimePorts`（`AgentBuilder::build_ports` → `materialize_runtime`），每次物化得到独立 ModelManager / queues / session / coordinator / compaction。

开箱主线 vs 配置后置（Server / MCP / 更多 adapter）见根 `AGENTS.md` 与 `docs/architecture/`。库嵌入：`xylitol::embed`。

## 三圈契约与 `Xy*`

```text
① 线协议     wire Command / Event
② 应用协议   XyDriver + XyEvent 流 + XyDriverError
③ 可替换口   XyModel / XyTool / XySessionStore / …
```

- **`Xy*`** = 库入口级契约（精选 `pub use` / embed），不是全局品牌。端口、共享应用协议、跨面事件、库级错误用 `Xy*`；内部协作者、单处 DTO、bootstrap 细节、包 `xylitol-tui` **不用**。
- **外部库**：会出现在库入口或多方言统一处 → 包一层；纯内部 → 直接用上游类型。禁止为包而包。
- **取消**：公开工具上下文直接用 `tokio_util::sync::CancellationToken`（接受为契约依赖）。
- **schemars**：放 infra/config 面；protocol 根类型默认只 serde。
- **`XyEventSink`**：侧路生命周期（如 compaction），**不是**多 client turn 总线；turn 走 `XyDriver::run` 的事件流。

端口清单与签名以 `protocol/ports` 代码为准。升格新 port 前先证明不能用配置开关或既有 `XyTool`/`XyModel`/`XyDriver`（见下「扩展」）。异步 port 以 `dyn` 注入 → **维持 `async_trait`**（朴素 RPITIT 与 dyn 冲突）。

## 错误与观测

- 热路径：`XyError` / `XyToolError` + 稳定 `kind()`；会话域用 `XySessionError`（含 store 持久化 + 无绑定会话 / busy / 树旅行），`XySessionStoreError` 只表示 `XySessionStore` 持久化失败（`fork` 例外：树定位失败走 `XySessionError`）。Driver flatten：store `NotFound` → `kind=NotFound`；`NoActiveSession` → `kind=Message`、`detail_kind=Session`。
- 整机缝：`XyDriverError`（含 `Agent(…)`）+ `kind` / `detail_kind` / `log_failure`。session/export/trust 失败 flatten 后 `kind` 仍是 Driver 分类（`NotFound`/`Io`/…），`detail_kind` 与 `log_failure` 的 `source.kind` 保留来源域（`Session`/`Export`/`Trust`）。opaque 字符串经 `from_opaque`（具体短语，避免裸 token 误伤）。
- 栈：**仅** fastrace + `log`；禁止 `tracing` 双栈。失败日志宜带 `error.kind`。
- 读 trace：skill `xylitol-inspect-runtime-logs` / `just obs-*`；禁止整文件灌 JSONL。

## 工具与钩子

```text
跨进程 / MCP / 脚本 hook  = JSON Value
crate 内置工具             = *Args + serde；优先 TypedTool → dyn XyTool
```

- 不改 MCP / hook 的 JSON 口换「全类型」。
- schema 与 Args 手写并置；钩子总线与 `AgentHooks` **刻意保留 Value**。
- 内置工具经 `TypedTool` 装配；MCP / hook 仍走 `Value` + `XyTool`。

## 扩展开闭

产品心智：`docs/architecture/扩展与开闭.md`。实现优先序：

1. **配置 / 会话开关 + 组合根接线**（未启用 zero-cost）
2. **既有 port**（禁止旁路第二套语义）
3. **新 port**（仅真实第二实现或嵌入/测试必须替换时）

禁止：插件市场、Extension Host、为未交付能力预挖空 `Xy*`、「已是统一口再包一层」。未发布卫生（无兼容别名 / 无无理由 `allow(dead_code)`）见根 `AGENTS.md`「Pre-0.0.1 卫生」。

Trust / Permission / MCP 产品语义分别见 `docs/architecture/` 对应文；实现要点：Trust 闸**项目本地资源**；工具开箱 allow-all；MCP **有配置才装配**并支持重载。

## Provider 与消息

业务只认 `XyModel`。HTTP/SSE 优先官方 SDK，落在 `packages/xylitol-ai-bridge`；主仓：

1. **agent**：`project_for_llm`（`AgentMessage` → LLM 叶；Env 折叠）
2. **infra**：装配 `XyModel`、边界 map；**MUST NOT** 再对 `AgentMessage` 做 Env 折叠主路径

| 概念 | 归属 |
|---|---|
| LLM 叶（Message/Part/Usage…） | bridge DTO；protocol 可 `pub use` 别名 |
| `AgentMessage` / `EnvMessage` / `XyEvent` | protocol 根；agent 可再导出 |
| `project_for_llm` | agent |
| 真边界差（chunk 等） | infra map |

protocol **MAY** 依赖 bridge **DTO only**，**MUST NOT** 依赖 bridge HTTP/SDK。新兼容端 = adapter/配置；**不改** ReAct / `AgentMessage`。Pre-1.0 交付范围见根 `AGENTS.md`。

Hook 三缝只认可移植 JSON（headers map + body Value）；不把 reqwest/某一 SDK 类型泄漏进 hook。原始 SSE 诊断用进程内 provider trace，不进 hook。

## 复杂度与体量

生产模块避免无结构 God 文件。**不以文件物理行数作为质量约束**（行数与可维护性无直接映射，行数硬测会逼出按 impl 块散落的假拆分）；质量信号只走函数级复杂度指标（cccc-rs 的 Sonar cognitive / McCabe cyclomatic）。

**TUI 面复杂度闸（HARD）**：产品 TUI 入口协调者（host 入口、layout 根、effects 入口、bridge 入口）的函数级复杂度经 `just qa` 的 `scripts/check_complexity.py`（cccc-rs）强制：Sonar cognitive ≤32 且 McCabe cyclomatic ≤27（按当前协调者 max 收紧，禁止回涨到旧 35/30）。**不以文件物理行数作为硬闸**（行数硬测会逼出按 impl 块散落的假拆分）。测试专用模块不参与本闸。

**全树复杂度雷达（review 信号）**：`just complexity`（`--radar`）对全生产树（排除测试文件）报告 cccc-rs top-cognitive 排名，作为 review 参照；软信号，非硬闸。复杂度高的函数优先拆分/重构，不用行数 KPI。

**默认不为行数大拆**：ReAct（剧本可与行为测同居；state machine 未开闸）、session manager、已拆开的 driver 子树。功能逼出或编辑痛点明确时再拆；复杂度超闸须拆分或显式豁免。**不**在本文件维护超标清单。

## 测试

跨层：BDD + 回归 + 面/Driver 行为测。细则与命令见根 `AGENTS.md`。设计史：`llmanspec/changes/archive/`。

### 进程级全局状态与并行纪律

进程内共享、跨测会互相踩的状态（非穷尽）：obs parent / session 槽、`provider_trace_active` 与 fastrace 全局 reporter、`std::env` / cwd、包级 `GLOBAL_KEYBINDINGS`、固定磁盘路径与端口。

- 新测试禁止无必要引入新的进程级可变全局；能注入 / 局部化则注入。
- 必须串行时用**按域命名**的 `#[serial(<domain>_global)]`（`obs_global` / `env_global` / `kb_global` 等），禁止默认空组把无关域捆在一起；注释写清为何串行与消除路径。
- 同域内「只写等价默认值、可与破坏者互斥」的测可用 `#[parallel(<domain>_global)]`（与同名 `serial` 互斥、彼此可并行）。
- 主闸 `just qa` 走 nextest（一测一进程）：跨测互斥优先考虑 nextest `test-groups`（共享 FS/端口等外部资源）；勿假设 in-process `#[serial]` 在 nextest 下跨测生效。当前无固定端口 / 共享非 temp 路径 → **不**配置 `[test-groups]`（见 `.config/nextest.toml`）；出现新外部共享资源再开。
- `cargo test` 回退与 `just test-tui` 仍依赖 in-process 命名组。
- 产品 `HostSession` 经 `KeybindingsScope`（thread-local）持有键位；勿在 `new_product_ui` 路径无必要写 `GLOBAL_KEYBINDINGS`。配置发现优先 `ConfigPaths::discover_with` / `load_app_config_*_with` 注入；trust 持久化走 reload `agent_dir`（勿为测改 `HOME`）。测试勿把 `SessionManager` 指到开发者真实 `~/.xylitol/sessions`（用 tempfile）。Obs 会话身份测用 `ObsSessionScope`（勿为测独占改写进程槽）。
- **`env_global` 仅剩**：`secret.env` 真注入进程环境（`secret_env` 单测 + `loader` 相关测）；`otel` live Langfuse smoke（兼 `obs_global`，装 fastrace reporter）。其它路径能注入则注入。
- **`obs_global` 必须 `serial`**：仅 live OTLP smoke（及仍直接 `fastrace::set_reporter` 的非 demux 路径）。单元测用 `ObsGateScope`（TLS 闸）+ `SpanCollectScope`（进程全局 demux sink + 互斥 permit；`flush` 在 helper 线程 report，故不能 TLS）；会话槽用 `ObsSessionScope`；parent 已走 `obs_parent`。
- **`kb_global`**：产品 / 组件测优先 `KeybindingsScope`；勿为测写 `GLOBAL_KEYBINDINGS`（域已可空）。
- **Kitty 键盘协议标志**：测走 `with_kitty_protocol_active`（API 内串行锁）；勿裸 `set_kitty_protocol_active` 跨断言窗口。
