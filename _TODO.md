# xylitol 规划脊柱

规划真值，进 git 便于 diff。不升格为 `AGENTS.md` / spec。
进度只改本文件。稳定规则写进 `AGENTS.md`。行为合约写进 `llmanspec/`。
文风：`/home/l8ng/Projects/__straydragon__/STC-Simplified-Technical-Chinese`。

一事只在一节写全，其余用指针。沿革查 git。

<a id="now"></a>

## 现在

下一刀是**结构扫障**，不是再拆 crate，也不是写 rutis。产品回归（TUI / Host / MCP 门闸）仍走主线修复。GitHub [#28](https://github.com/StrayDragon/xylitol/issues/28) 仍是未开放端的认领口。

**定位**（避免前后矛盾，细则 [§2.2](#POS)）：xylitol 是个人开箱即用的 coding agent。不是插件市场，不是 Extension Host。终端用户靠 YAML / MCP / hooks / skills 扩展。作者侧「跑起来后改接线、少重启」是工程性质，不是给第三方的热插拔承诺。

**crate 图**作为静态边界已经够用。根包是组合根，不抽 `xylitol-host` / `xylitol-config` / `xylitol-app-cli`。不为编译速度拆 crate。依据见 [§3](#SRC)。真源：`src/AGENTS.md`。

**rutis** 排在扫障之后，条件见 [§4](#MAIN)。现在不得 `use rutis`，不得再建进口隔离包，不得预埋空壳 `Plugin`。`Ctx` 不传回合消息。

扫障清单见 [§3.4](#PRE)。延后项见 [§6](#DEFER)。

禁边闸：`scripts/check_crate_graph.py`。成员名单以根 `Cargo.toml` 为准。

回退点 tag **`v0.0.0-pre.old`**（`004a2e4d`）。命令：`git switch -d v0.0.0-pre.old`。

“不用就不付钱”分三层，不要混。

| 层 | 规则 |
|---|---|
| 编译 | 依赖写在会用该库的 crate |
| 装配 | 没挂的插件不 start |
| 热路径 | 不在每 token / 每帧上计费 |

读法：现在 → [§2](#TARGET) 形态与定位 → [§3](#SRC) 根包与扫障 → [§4](#MAIN) 转向日程 → [§5](#SCOPE) 产品核 → [§6](#DEFER)。

---

## 1. 约束

1. 行为不变的组织改动可跳过 llman-sdd 生命周期。live `.feature` 主语 / 路径过期时，直接改 feature，再跑 `llman-sdd validate`。
2. 产品可观察行为变化必须走完整 SDD：explore → propose → apply → verify → archive。
3. 任何批次合并前必须 `just qa`。触及真终端时再加 `just qa-e2e`。
4. 试验脚本只许 `lab_` 前缀。
5. 提交用 Conventional Commits。未要求则不自动 commit。
6. `AGENTS.md` 只收硬约束 / 稳定原则。不得把本文件进度抄进去。移除即改写，不留墓碑。
7. 同 crate 的不同 worktree **不得**共用 `CARGO_TARGET_DIR`。
8. 改名一次性改调用点。不留兼容别名，不留 `#[deprecated]`。

| SDD | 何时 | 做什么 |
|---|---|---|
| skip-SDD | 纯组织 / 工具链 / 编译隔离，且产品行为不变 | 直接改代码 + `just qa` |
| validate-only | 组织演进导致 live `.feature` 过期 | 直接改 `.feature` + `llman-sdd validate` + `just qa` |
| must-SDD | 改了用户能看见或默认二进制含什么 | 完整 change 生命周期 |

发现“以为组织改动、实际改了行为”时立刻停，转 propose。

提交：工具链 `build:` / `chore:`；重写 `feat:` / `refactor:`；行为 `fix:`；SDD `chore(sdd):` / `archive(sdd):`。
不碰（除非该批次点名）：`docs/architecture/`、`_INTERVIEW/`、`delayed-changes/legacy/`。改行为才动 `llmanspec/`。

不得创建：`xylitol-host`、`xylitol-client`、`xylitol-kernel`、`xylitol-assemble`、`xylitol-types`、`xylitol-app-cli`、`xylitol-config`、`xylitol-obs`、`xylitol-official-plugin-react`、`xylitol-official-plugin-tui`、`xylitol-app-voice`、空 LSP/DAP crate、dylib 加载器、`crates/xylitol/`。

---

<a id="TARGET"></a>

## 2. 现行形态

crate 图、依赖方向、`Xy*`、开闭规则的真源是 `src/AGENTS.md`。本文件只留规划要用的定案。不得 `pub use` 根 `src/` 模块凑编译。不得留兼容 shim。

三层分名：**角色**（执行权威）/ **服务提供方**（`xylitol-runtime`）/ **载体**（`xylitol-serve`）。**组合根不是第四个角色名**，落在根包 `xylitol`。

`XyDriver` 是 runtime 侧应用协议。客户端只通过 attach 消费。产品 TUI 不是 rutis fiber。

<a id="SETTLED"></a>

### 2.0 定案

**P1 替换** 不同理由分开：厂商 HTTP ≠ 压缩策略 ≠ 会话落盘 ≠ 工具实现 ≠ ReAct 循环。OpenAI 兼容与 Anthropic 各一插件。DeepSeek overlay 留在 openai 插件。

**P2 开关** 没配置不 start。mcp / hooks 仍是官方插件 crate。`server` / `mcp` feature 可从二进制拿掉。未配置不得为“将来插件”堆抽象。

**P3 生命周期** 现行：组合根注入 `Arc`。用户插入点仍是配置 / 文件。S2 才把有配置的 MCP / hooks 挂到 fiber；未接入前不得预埋空壳 `Plugin` / `Ctx`。会话 instance 见 [S3](#S3)。dylib 见 [S4](#S4)，必须与否后定。不得做 Extension Host。不得 `use rutis`。

**P4 热路径** 现行：ReAct 循环在 `xylitol-runtime`，无第二实现。测端已有 `ScriptedDriver`。这是现行边界，不是永禁；转向见 [§4](#MAIN)。

**P5 稳定方向** `plugin-sdk` ∥ `protocol` ∥ `xylitol-tui`。runtime → sdk。runtime ↛ 插件、↛ 信封。插件之间不得 `use`。mapping 只在 serve。Print 只依赖 runtime。

**P6 依赖卫生** `async-openai` 只在 openai 插件。`rmcp` 只在 mcp 插件。`salvo` 只在 serve。fory 只在 protocol。Anthropic 构建不得编 `async-openai`。第三方与 `xylitol-*` 版本真源是根 `Cargo.toml` 的 `[workspace.dependencies]`。

官方插件七个：session、compaction、provider-openai、provider-anthropic、tools、mcp、hooks。没有 react。内置默认挂：session / compaction / provider-* / tools。配置启用：mcp / hooks。

测循环用假适配器。测端用 `ScriptedDriver`。插件依赖声明适配器，由组合根注入。

<a id="EXTEND"></a>

### 2.1 插入点

用户 / agent 扩展能力，今天**不写** rutis `Plugin`。产品心智：[扩展与开闭.md](docs/architecture/扩展与开闭.md)。

| 角色 | 怎么插 | 格式 |
|---|---|---|
| 使用 xylitol 的人 / agent | 改配置和项目文件 | YAML、MCP、hook JSON、SKILL.md |
| 嵌入库的 Rust 作者 | `embed` + `XyDriver` + 可选 `Xy*` | 词表 / Driver，不编 fory |
| Host 插件作者 | `impl` plugin-sdk 端口，改根包接线 | `XyTool` 等；不编 fory、不 `use rutis` |
| 多语言叶作者 | 无产品路径 | [S4](#S4) 才有 |
| 新端作者 | 信封客户端 | fory-v3 |

两条通道不互转：端 ↔ Host 走信封 fory-v3；Host 进程内装配走 `Arc` 注入。唯一产品映射是 serve 的 `XyEvent` ↔ `v3::Event`。不得做 fory ↔ rutis 转码器。

主动交互：唤醒源走准入，不换内环。schedule 见 [D7](#D7)。

<a id="POS"></a>

### 2.2 定位（三条受众，不要混）

产品心智真源：[产品分层总览.md](docs/architecture/产品分层总览.md)、[扩展与开闭.md](docs/architecture/扩展与开闭.md)。本文件只写规划时容易写拧的边界。

| 受众 | xylitol 是什么 | 不是什么 |
|---|---|---|
| 使用 xylitol 写代码的人 | 开箱就能聊、能改仓库的个人 coding agent。默认 TUI attach 本机 Host | 插件市场、扩展平台、每个工具弹窗审批 |
| 嵌 xylitol 的 Rust 作者 | `embed` + `XyDriver` + 精选 `Xy*` | 内部 `wiring` 当稳定 API |
| 改 xylitol 本仓的人 | 组合根换端口实现；`/reload` 重读配置 / MCP / skills，不新开会话 | 不重启换内核；第三方 dylib；把官方插件当成市场货架 |

三条不要合成一句「xylitol 要热插拔」。用户扩展 = 配置与文件。作者热换 = 进程内换接线。内核转向 = rutis / cordis，见 [§4](#MAIN)。

现行已经能承诺的「少丢状态」：

| 能 | 不能当现行必须 |
|---|---|
| `/reload` 换 MCP / skills / 提示，会话树还在 | 换 ReAct 实现而不停回合 |
| 会话 JSONL 可续、可分支；Host linger 占槽 | 杀进程后恢复**在飞** agent loop |
| Print / embed 同进程直调；TUI 默认跨进程 attach | 产品 TUI 当 Host 侧 fiber |

主线与后置仍以 [§5](#SCOPE) 与产品分层总览为准。未开放端（gpui、LSP、DAP、语音）走 [#28](https://github.com/StrayDragon/xylitol/issues/28)，不得写成现行必须。

---

<a id="SRC"></a>

## 3. 根 `src/`：组合根，不拆 crate

结论：组合根留在根包 `xylitol`。不为对称性抽 `xylitol-host` / `xylitol-config` / `xylitol-app-cli`。不为编译速度拆 crate。行数不是闸。

### 3.1 现行落点

五文件入口：`main.rs` / `lib.rs` / `cli.rs` / `wiring.rs` / `embed.rs`。

| 目录 | 职责 |
|---|---|
| `cli.rs` + `cli/` | argv。`run()` 在 `cli.rs`；clap 词表 `cli/args.rs`；表面意图 `cli/intent.rs` |
| `wiring/config/` | YAML 装载 + schema 闸 |
| `wiring/driver/` | 根包装器 `XyComposedDriver`（内层是 runtime `XyInProcessDriver`）+ `/reload` 配方 |
| `wiring/occupancy/` | serve 装配。占用政策在 `xylitol-serve` |
| `wiring/bootstrap.rs` | 各端共用装配路径（`build_agent` 之前） |
| `wiring/composition.rs` | 注入官方插件 `Arc`；`McpSession` |
| `cli/logging/` | 进程启动写 `xylitol.log` / 可选 OTLP |
| `wiring/settings/` | 用户设置落盘 |
| `wiring/{provider,event,export,timing}` | 组合根接线用的实现 |

TUI 画布、Print 渲染、HTTP 监听、ReAct、工具实现不在根 `src/`。分别在 `xylitol-app-*` / `xylitol-serve` / `xylitol-runtime` / `official-plugin-*`。

观测写出留根包：进程启动才知道写日志、是否装 OTLP。

测留同 crate `#[cfg(test)]`：occupancy 装配夹具、bootstrap、MCP/`/reload` 各跟所测模块。不得升成 `tests/*.rs` 新集成测试二进制。每个文件单独链接整树，是 m×n 爆炸。

`config/types.rs` 是 YAML 形状，不是协调者。没有第二套装载器之前，不拆。`bootstrap` 与 `composition` 的分工写在文件头，不为行数再套 wrapper。`cli = []` 保持空 feature。`jsonschema` / `schemars` 留根包装载闸。

### 3.2 为什么不拆 crate

Cargo 按依赖 DAG 并行调度。宽而浅能并行；链状拆 crate 收益趋近于零。见 `~/.config/llman/skills/rust-build-tune/references/ci-and-codegen-model.md`。

根包 `xylitol` 已经是 workspace **叶子**：它依赖官方插件、runtime、serve、两端。从叶子再抽出 `xylitol-config` / `xylitol-obs`：

1. 图变成 `叶子 → 新 crate` 的链，不增加并行度。
2. `AppConfig` 的调用方是根包自己和同 crate 的 BDD。抽出后改 YAML 类型仍会重编根包。
3. 多一个 crate 多一份 rustc 启动与 metadata，对装载代码得不偿失。

`src/AGENTS.md`：不为编译速度拆 crate。不得以文件物理行数作为质量约束。复杂度闸走 `just complexity` / TUI 硬门禁。

编译参数按 rust-build-tune 磁盘优先，见 [附录 A](#FACTS)。不要为了再拆 crate 去动 profile。Rustls / reqwest 在根包依赖图里。Cranelift 不用。

### 3.3 `/reload`：Host 跑资源重载，TUI 跑端侧，CLI 不处理

改需求：资源重载（trust / YAML / MCP 重连 / skills / 工具表 re-freeze）归 **Host**。键位和主题归 **TUI**。CLI argv 与 Print **不暴露** `/reload`。

现行落点已经按这个切：

| 面 | 谁跑 |
|---|---|
| 产品 TUI attach | slash → unary `reload` → `HostState::reload_resources` → 工厂注入的引擎 |
| TUI 本地 | `reload_keybindings` / `reload_themes`，不进信封 |
| 进程内 TUI | 同一进程里的 Host（adapter 转发）。不是 CLI 处理 slash |
| Print / CLI argv | 无 |

配方文件仍在 `wiring/driver/in_process/reload.rs`，因为 occupancy 工厂在根包装配槽位，reload 是「同一套装配再跑一遍」。这不是 CLI 累赘，删掉会让产品 `/reload` 掉进 runtime 默认 no-op（静默空报告）。

不把配方搬进 `xylitol-serve`：载体会吞 YAML 与官方 mcp 插件，变成第二组合根。换房间不是清死代码。

扫障可清：过期注释把 dispatch 写成 REST / CLI 命令。不可清：`enable_reload_state`（工厂给 Host 用）和 `reload.rs` 本体。

<a id="PRE"></a>

### 3.4 rutis 之前还做什么

结论：静态 crate 边界已经能撑主线。转向前只做**扫障**和**产品主线**。不靠再切包逼近「不重启改形态」。

**保持的主线**（与 [§5](#SCOPE) / 产品分层总览一致）：

- 用户：对话循环、内置工具、会话树 / 导出、压缩、OpenAI 兼容 + Anthropic、Trust、图像。默认入口 TUI attach 本机 Host。
- 后置：MCP（有配置才装配）、hooks。
- 跨端：Print / TUI / Host 同一套动作语义。未开放端不写进现行必须。
- 工程：`plugin-sdk` ∥ `protocol` ∥ `xylitol-tui`；runtime ↛ 插件；组合根唯一。

**扫障（skip-SDD，不改产品语义）**

| 做 | 不做 |
|---|---|
| 组合根职责可跳转：`XyComposedDriver` = 现行热换端口配方位 | 抽 `xylitol-app-cli` / `xylitol-host`，让 `src/`「只剩 wrapper」 |
| CLI 留根包：`cli/args.rs` 词表、`cli/intent.rs` 意图、`run()` 调接线 | 把 bootstrap / `/reload` 配方搬进所谓 app-cli |
| 文档与 crate 图用现行名；矛盾收口在 [§2.2](#POS) | 为 rutis 预埋 `Ctx` / `Plugin` / 进口隔离包 |
| 产品回归优先：MCP 门闸期间回车须立刻有 Assembling / 队列条 | 把 Host 持锁等 MCP 的进度刷新当成转向前置 |

**两条「少丢状态」不要写成同一句**

1. **对话还在**：会话 JSONL + Host linger。杀 TUI、甚至 linger 窗口内杀客户端，树还在。这是现行主线。
2. **在飞 loop 还在**：要 instance 回收语义（[S3](#S3)）之后才谈。杀 Host 进程等于停回合。不是 crate 切分能给的。

`/reload` 是现行产品面上最接近「不换会话换接线」的刀。扫障时把它当原型护住，不要拆散。

---

<a id="MAIN"></a>

## 4. 转向日程：rutis

排在 [§3.4](#PRE) 扫障之后。终端用户扩展仍然是 skills / MCP / 配置，不因转向变成插件市场。

接入条件（全满足才写代码）：自己用 rutis 跑通过；愿意改「不得 `use rutis`」；热路径延迟有基线可比。未满足前不得预埋空壳。

另开时先定：**根包直接 `use rutis`**（rutis 当基础库，用到的 crate 直接依赖）。不再做进口隔离包。届时改写 `src/AGENTS.md` 的「不得 `use rutis`」。

**`Ctx` 不传回合消息。** 用户一轮、工具起止、token 增量走 `XyDriver` 队列与信封（`XyEvent` / fory-v3）。`Ctx` 是进程底板：服务注册表（`TypeKey` → `Arc`）、插件生命周期总线、fiber 表、instance。两通道不互转，见 [§2.1](#EXTEND)。`rutis-host` 持有根 `Ctx`，本身不是 Plugin。

**现行 vs 转向。** 现行：官方库是端口实现；ReAct 在 runtime。这是当前预期，可以改。转向吸引点是作者侧进程内换形态（cordis 式插件生命周期 + 热重载）。那一步等于换内核，另开。它**不**把 xylitol 改成第三方扩展市场。dsh 创造模式 / loop 当插件 **不等于 S2**。S2 只是 `Ctx` 当桌子常驻。loop 当 Plugin 是换内核，S2 之后另开。

读 [`/home/l8ng/Projects/__straydragon__/rutis/xylitol-x-rutis-temp`](/home/l8ng/Projects/__straydragon__/rutis/xylitol-x-rutis-temp) 时以 2026-10-20 对照为准。工作树：上游 `3d0bc0d` / rutis 0.13.0。

热路径延迟不劣化仍是若接入时的 go/no-go。契约叶与 `xylitol-app-*` 留下。

<a id="S2"></a>

### 4.1 S2 运行时承载走 rutis

**未开。** 扫障完成且接入条件满足后再开。另开前先定根包直接 `use rutis`（见本节开篇）。

用户仍写配置 / MCP / hooks / skills。有配置时生命周期挂在 fiber 上。空配置行为不变（r2911 / r2912）。终端用户不写 `Plugin` trait。

没有常驻 `Ctx`，hooks / MCP 无法挂 fiber。

| 切片 | 状态 | 做什么 |
|---|---|---|
| S2a `Ctx` 常驻 | 未开 | 进程一次 `Ctx::root()`。空 MCP / hooks 仍不挂 fiber。Print 仍不占端口。must-SDD |
| S2b hooks 走总线 | | 有配置才挂；空 hooks 不拉进程。旧 hook BDD 必须绿。must-SDD |
| S2c MCP 走 loader | | 有配置才连；空列表无 `mcp` 前缀。`/reload` 语义不变。must-SDD |

过：`just qa` 全绿。`workspace-assembly` 五场景绿。`host.describe` 仍含 fory-v3。

<a id="S3"></a>

### 4.2 S3 会话 = instance

**未开。** 占用政策（linger、单写者）在 `xylitol-serve`。JSONL 树 / fork 仍在 session 插件。S2 之后才谈「杀进程仍保住在飞 loop」。

| 切片 | 状态 | 做什么 |
|---|---|---|
| S3a 实例 API | | 若接入：instance 隔离 API。单测：两 instance 互不可见；卸载零残留；在飞恰好失败一次。不改 Host |
| S3b 槽 = instance | | `materialize_writer_at` 在会话 instance 登记写者。删 `HostState.sessions` 作为物化权威。occupancy BDD 继续绿。双会话互不可见、结束零残留走 propose |
| S3c 删基线 | | 删 `RuntimeBaseline` / `build_baseline*` / `AgentBuilder::build_baseline`。调用点一次改完，不留 shim。Print 仍能 `AgentEnd` |
| S3d freeze → generation | | 工具表冻结挂 generation。超时仍冻结。`arm_tool_freeze` BDD 绿 |

Kill：实例回收与快照 / 恢复冲突 → 隔离退回手写 HashMap，保留 S3a API。

<a id="S4"></a>

### 4.3 S4 多语言叶 + dylib

**未开。** 动态加载。官方 Host 插件在此之前保持静态 crate。必须交付或降为实验特性，做到那一刀再定。不得在 S2/S3 预埋加载器空壳。

<a id="S5"></a>

### 4.4 S5 产品 TUI：客户端订阅

**未开。** 产品 TUI 仍走信封，不是 Host 侧 fiber。S5 只改端进程内合帧节奏。不改 `POST /rpc` + `WS /rpc` + fory-v3。不得做 `xylitol-official-plugin-tui`。端不得 `use rutis`。验收含 `just qa-e2e`。

---

<a id="SCOPE"></a>

## 5. 产品范围

核 = 必须在现行 crate 图里。瘦 = 代码留、feature-gate 或下沉。删用户可见面须改 `.feature`。未列入本节的方向见 [§6](#DEFER) 或 GitHub [#28](https://github.com/StrayDragon/xylitol/issues/28)。

**核 — 用户侧**：TUI 默认 attach 本机 Host；Print；embed `XyDriver`；slash；bang；`$skill`；会话树；滚动区 / 固定区；换模 / thinking；图像粘贴 / 读图；designing（`xylitol tui designing`，仅 debug 构建；合约 [c2930](llmanspec/changes/archive/2026-10-18-c2930-add-tui-preview/)）。

**核 — Host**：监听 + 四象限信封（HTTP 不得删）；`XyDriver` 会话槽 / 单写者 ReAct；内置工具；Trust；会话持久化 / 导出；压缩；OpenAI 兼容 + Anthropic；MCP（有配置才装配）；hooks；`edit` 内部 patch。

**核 — 观测**：`provider-trace.jsonl` + fastrace；`xylitol.log`（永不 stdout）；`just obs-*`；inspect 脚本；`otel` feature（default 开，配置默认 none，关 feature 真能关）。不得再引入 `tracing` 双栈。

**瘦**：serve `stop` 只印 SIGTERM；`server` / `mcp` feature 可关（default 仍开）；`jsonschema` / `schemars` 留根包装载闸；`minijinja` 根包装载 + runtime prompt；`cli = []` 是空 feature，不发明新开关。`incremental = false`。

---

<a id="DEFER"></a>

## 6. 延后

现在不做。认领走 GitHub [#28](https://github.com/StrayDragon/xylitol/issues/28)。不得为这些方向预埋空壳 crate。

| 项 | 内容 | 若认领 |
|---|---|---|
| D2 | `AgentMessage` / `EnvMessage` / `AgentPart` 不加 `Xy`。`Xy*` 留给适配器与叶 | 改名前 must-SDD。`AgentPart` 将来前缀叫 `XyPart` |
| D6 | gpui、LSP、DAP、Sub-Agent、语音端 / 全双工 realtime、键位可视化、Inspect wasm / 框选 | 各一个 FR。不得 `xylitol-app-voice` / 空 LSP crate |
| D7 | schedule / cron 唤醒源。唤醒源走准入，不换内环 | must-SDD |
| D8 | samply suite、Jaeger。`agent_demo*` / `host_loop` 留在 `xylitol-tui`。maka 对照留 `docs/research/` | 观测后置 |

信封标识：`xy.wire.v3` / `application/x-fory-v3`。实现见 `crates/xylitol-protocol/src/v3/`。

---

## 7. 来源

| 主题 | 路径 |
|---|---|
| 团队怎么做 | `.claude/skills/xylitol-dev/SKILL.md` |
| STC | `/home/l8ng/Projects/__straydragon__/STC-Simplified-Technical-Chinese` |
| 代码架构 | `src/AGENTS.md` |
| 产品主线 | `docs/architecture/产品分层总览.md` |
| 编译调优 | `~/.config/llman/skills/rust-build-tune/` |
| rutis 方向 | `/home/l8ng/Projects/__straydragon__/rutis/xylitol-x-rutis-temp/` |

---

<a id="FACTS"></a>

## 附录 A. 现行 workspace 与编译

成员：根包 + plugin-sdk / protocol / runtime / serve / app-tui / app-print / xylitol-tui / 七个官方插件。`vendor/gherkin` 是 `[patch.crates-io]`。根 default = `cli, tui, otel, server, mcp`。

| 项 | 事实 |
|---|---|
| `cli = []` | 空 feature，不裁依赖 |
| `server` | optional；default 开；关 feature 真能关 |
| `mcp` | optional `rmcp`；default 开；运行时有配置才装配 |
| `otel` | optional OTLP；default 开；配置默认 none；关 feature 真能关 |
| `[workspace.dependencies]` | 版本真源。成员 `workspace = true` |

dev：`opt-level = 0`，依赖 `opt-level = 3`，`debug = "line-tables-only"`，`incremental = false`，`codegen-units = 16`。
release：`opt-level = "z"`，`lto = "thin"`，`strip = "symbols"`，`panic = "abort"`，`codegen-units = 1`。
`.cargo/config.toml`：Linux mold + `-Zshare-generics`（同一份 rustflags）。不用 Cranelift。
主 worktree 用仓库内 `target/`；额外 worktree 须 `eval "$(just cargo-wt-env)"`。

产品主线：对话循环、内置工具、会话持久化 / 导出、压缩、两家模型 API、Trust、图像附件。默认入口：产品 TUI attach 本机 Host（未在听失败）。后置：MCP。

---

## 附录 B. designing

入口：`xylitol tui designing`（仅 debug 构建）。场景槽 + 假 Host。designing preview = 把某场景注入同一 TUI 画布。合约：[c2930-add-tui-preview](llmanspec/changes/archive/2026-10-18-c2930-add-tui-preview/)。

框选 / `InspectFrame` / Inspect wasm 见 [D6](#D6)。不建 `designing/` 目录，本轮不装 wasm target。
