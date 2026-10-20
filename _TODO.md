# xylitol 规划脊柱

规划真值，进 git 便于 diff。不升格为 `AGENTS.md` / spec。
进度只改本文件。稳定规则写进 `AGENTS.md`。行为合约写进 `llmanspec/`。
文风：`/home/l8ng/Projects/__straydragon__/STC-Simplified-Technical-Chinese`。

一事只在一节写全，其余用指针。沿革查 git。

<a id="now"></a>

## 现在

crate 图见 [§2](#TARGET)。`just qa` 与 `just qa-e2e` 已通过。

下一刀是 rutis 接入，另开讨论。[S2](#S2)–[S5](#S5) **停**。插入点见 [§2.0c](#EXTEND)。

延后项见 [§5](#DEFER)：D2 词表不加 `Xy`；D6 未开放端；D7 schedule；D8 观测后置。

装配适配层 `xylitol-rutis` 在 workspace。组合根经它 `apply` 官方 Host 插件；`apply` 后立刻 `shutdown`。S2 起才改这条语义。

禁边闸：`scripts/check_crate_graph.py`。成员名单以根 `Cargo.toml` 为准。架构规则以 `src/AGENTS.md` 为准。

回退点 tag **`v0.0.0-pre.old`**（`004a2e4d`）。命令：`git switch -d v0.0.0-pre.old`。

“不用就不付钱”分三层，不要混。

| 层 | 规则 |
|---|---|
| 编译 | 依赖写在会用该库的 crate |
| 装配 | 没挂的插件不 start |
| 热路径 | 不在每 token / 每帧上计费 |

不为编译速度拆 crate。单包 default-on 的 Cargo feature 不省默认二进制。

读法：现在 → [§2](#TARGET) → [§3](#MAIN) → [§5](#DEFER)。

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

不得创建：`xylitol-host`、`xylitol-client`、`xylitol-kernel`、`xylitol-assemble`、`xylitol-types`、`xylitol-official-plugin-react`、`xylitol-official-plugin-tui`、`xylitol-app-voice`、空 LSP/DAP crate、dylib 加载器、`crates/xylitol/`。

---

<a id="TARGET"></a>

## 2. 现行形态

crate 按契约划分。不得 `pub use` 根 `src/` 模块凑编译。不得留兼容 shim。

三层分名：**角色**（执行权威）/ **服务提供方**（`xylitol-runtime`）/ **载体**（`xylitol-serve`）。crate 名只表达其中一层。不得叫 `xylitol-host`、`xylitol-client`、`xylitol-server-host`。**组合根不是第四个角色名**，落在根包 `xylitol`。

`XyDriver` 是 runtime 侧应用协议。客户端只通过 attach 消费。`xylitol-app-tui` 不是 rutis fiber。S2 起 rutis 仍只装配 runtime 进程内的 Host 侧插件。

<a id="TERMS"></a>

### 2.0 用语

本节名称在后文只按这里的意思用。对照总表：`docs/architecture/术语表.md`。

**词表**（vocabulary，共享类型）是 Host 侧（插件 + runtime）都要 `use` 的那一组类型和适配器 trait，几乎不含业务实现。例子：`AgentMessage`、`XyEvent`、`XyError`、`XyModel`。词表 crate 是 `xylitol-plugin-sdk`。产品 TUI 可依赖 sdk 做 `XyEvent` 投影，不得依赖官方插件，不得依赖 runtime。不得再拆 `xylitol-types`。

**词表薄留**：只把类型形状放进 `xylitol-plugin-sdk`。HTTP、假实现、观测实现不放进去。

**产品信封**（envelope）是 client/server 四象限 RPC。crate 是 `xylitol-protocol`。只描述跨端形状。不认识适配器，不认识 `XyEvent`。与 `xylitol-plugin-sdk` 互不依赖。用户可设开关名 `XYLITOL_ENVELOPE_V3`；未设默认开。content-type `application/x-fory-v3`；fbs `namespace xy.wire.v3`。改 namespace 等于删旧加新。

**官方 Host 插件**是本仓库 Host 侧静态 crate，包名 `xylitol-official-plugin-*`。不是插件市场，不是 dylib。动态加载见 [S4](#S4)。

**适配器**（adapter）：`XyModel`、`XyTool`、`XySessionStore` 这类可替换实现。调用点 `xylitol_plugin_sdk::{XyModel, …}`。不要写 seam / port / 可替换口。sdk 内私有模块名仍是 `ports`，不改名为 `adapters`。

**执行权威**是 runtime 侧角色：会话循环、trust、准入、ReAct 循环。crate 是 `xylitol-runtime`。`xylitol-runtime` 不是组合根。不认信封，不认官方插件。

**载体**（carrier）是监听 + 编解码。本仓库实现是 `xylitol-serve`。占用政策（会话槽、linger、单写者、mux）在 serve。装配与 MCP 物化由组合根注入。

**组合根**是唯一把循环 + 工具 + 模型 + 会话接在一起的地方。落在仓库根包 `xylitol`（argv + 默认接线）。经装配适配层 `apply` 官方 Host 插件。`xylitol-runtime` 不接线。

**装配适配层**：`xylitol-rutis`。唯一允许 `use rutis`。

**客户端 / 端**（client / surface）是 `xylitol-app-*`。端认信封或 `XyDriver`。端不是 Host 插件。TUI 引擎 `xylitol-tui` 零引用产品。产品 TUI attach 走 `xylitol-app-tui` 的远端 Driver。

**操作器**相对端而言：模型、会话、MCP、trust 在 Host 侧。端做键、画、TTY。

不得用这些 crate 名：`xylitol-types`；`xylitol-assemble`；`xylitol-kernel`；`xylitol-host`；`xylitol-official-plugin-react`；`xylitol-app-voice`。不得用 `xylitol-protocol` 承接 sdk。不得把信封 / 词表 / ReAct 循环做成可卸官方插件。

<a id="SETTLED"></a>

### 2.0b 定案

**P1 替换** 不同理由分开：厂商 HTTP ≠ 压缩策略 ≠ 会话落盘 ≠ 工具实现 ≠ ReAct 循环。OpenAI 兼容与 Anthropic 各一插件。DeepSeek overlay 留在 openai 插件。

**P2 开关** 没配置不 start。mcp / hooks 仍是官方插件 crate。`server` / `mcp` feature 可从二进制拿掉。未配置不得为“将来插件”堆抽象。

**P3 生命周期** 组合根经 `xylitol-rutis` `apply`。用户插入点仍是配置 / 文件，运行时承载见 [S2](#S2)。会话 instance 见 [S3](#S3)。dylib 见 [S4](#S4)，必须与否后定。不得预埋空壳。不得做 Extension Host。

**P4 热路径** ReAct 循环在 `xylitol-runtime`。不做成官方插件。无第二实现。测端已有 `ScriptedDriver`。

**P5 稳定方向** `plugin-sdk` ∥ `protocol` ∥ `xylitol-tui`。runtime → sdk。runtime ↛ 插件、↛ 信封。插件之间不得 `use`。mapping 只在 serve。Print 只依赖 runtime。

**P6 依赖卫生** `async-openai` 只在 openai 插件。`rmcp` 只在 mcp 插件。`salvo` 只在 serve。fory 只在 protocol。Anthropic 构建不得编 `async-openai`。第三方与 `xylitol-*` 版本真源是根 `Cargo.toml` 的 `[workspace.dependencies]`；成员 `workspace = true`。`vendor/gherkin` 走 `[patch.crates-io]`。

用户数据单元：书签 → 事实日志 + 回合/步骤。书签 ≠ 主路径 ≠ 树干前缀。KV 只追加不改前缀。切主路径 = 追加高注意力事实。压缩追加切点，不重写旧行。组稿三槽：稳定系统提示 / 高注意力（最新主路径；压缩后仍要看见）/ 易变状态栏。改注入策略走 must-SDD。

```text
我 → 会话*（书签 + 主路径）
      ├── 事实日志（只追加）
      └── 回合* → 步骤*
```

所有权：session 插件写事实；compaction 插件写摘要条；runtime 跑回合、步骤、投影组稿；端只消费事件。

官方插件七个：session、compaction、provider-openai、provider-anthropic、tools、mcp、hooks。没有 react。内置默认挂：session / compaction / provider-* / tools。配置启用：mcp / hooks。

测循环用假适配器。测端用 `ScriptedDriver`。插件依赖声明适配器，由组合根注入。

<a id="EXTEND"></a>

### 2.0c 插入点

用户/agent 扩展能力，今天**不写** rutis `Plugin`。rutis 只给组合根 `apply` 官方插件。产品心智：[扩展与开闭.md](docs/architecture/扩展与开闭.md)。

| 角色 | 怎么插 | 格式 | 经 rutis |
|---|---|---|---|
| 使用 xylitol 的人 / agent | 改配置和项目文件 | YAML、MCP、hook JSON、SKILL.md | 否 |
| 嵌入库的 Rust 作者 | `embed` + `XyDriver` + 可选 `Xy*` | 词表 / Driver，不编 fory | 内部装配会 `apply` |
| Host 插件作者 | `impl` plugin-sdk 端口，改根包接线 | `XyTool` 等；不编 fory、不 `use rutis` | 组合根替他 `apply` |
| 多语言叶作者 | 无产品路径 | — | [S4](#S4) 才有 |
| 新端作者 | 信封客户端 | fory-v3 | 否 |

现行插入点：模型配置；MCP；脚本 hooks；skills / `$skill`；slash / bang；Trust；`/reload`；会话内换模 / thinking。TUI 靠已有 `ToolStart` / `ToolEnd` 与文本增量呈现。不必为每个新工具改 fory schema。

Host 端口真源：`xylitol-plugin-sdk`。接线优先序：配置开关 → 既有端口 → 新端口（须有第二实现）。新增第八个官方插件必须改根包 `Cargo.toml` 与组合根接线。

两条通道不互转：端 ↔ Host 走信封 fory-v3；Host 进程内装配走 `TypeKey` + `Arc`。唯一产品映射是 serve 的 `XyEvent` ↔ `v3::Event`。S4 叶子才用 rutis-bridge serde，载荷是服务调用，不是信封帧。不得做 fory ↔ rutis 转码器。

主动交互：唤醒源走准入，不换内环。schedule 见 [D7](#D7)。

<a id="LEVELS"></a>

### 2.1 组件级别

crate 边界按替换、开关、生命周期来划，不按 `app / agent / infra` 分层来划。分层纪律留在每个 crate 内部。按契约写实现，不把今日模块原样倒进目标 crate。

内核判据：别的插件都要依赖，并且不能自己被门控掉。

| 级别 | 放什么 | 例 |
|---|---|---|
| 内核 | 词表、适配器、错误、信封、trust、会话身份、ReAct 循环。不得知道压缩策略、具体厂商、TUI。不得接线 | `xylitol-plugin-sdk`、`xylitol-protocol`、`xylitol-runtime` |
| 内置插件 | 官方静态 `xylitol-official-plugin-*`。默认挂。进程内 `apply` 取一次 `Arc`，之后直连 | session、compaction、provider-*、tools |
| 配置启用插件 | 仍是官方插件 crate；没配置不 start | mcp、hooks |
| 装配 | 同一批插件，rutis 装配器 | `xylitol-rutis` |
| 客户端 | 独立 `xylitol-app-*` + 引擎 `xylitol-tui`。不是 Host 插件 | app-tui / app-print / xylitol-tui |

不得为 LSP / DAP / gpui / 语音端预埋空壳。不得把客户端放进 `xylitol-official-plugin-*`。

<a id="GRAPH"></a>

### 2.2 目录总图

```text
仓库根 xylitol/                 组合根 package（BusyBox bin + embed）。不要 crates/xylitol/
  src/                          CLI + wiring + embed，见 [§2.2.1](#ROOTSRC)
  crates/
    xylitol-plugin-sdk/         词表 + 适配器 + 错误
    xylitol-protocol/           四象限信封 + fory-v3 + PROTOCOL_VERSION
    xylitol-runtime/            执行权威：XyDriver + trust + 会话槽 + ReAct 循环 + 唤醒准入
    xylitol-serve/              载体：监听 + 占用 + XyEvent ↔ v3 Event mapping
    xylitol-tui/                TUI 引擎。零引用产品。不改名
    xylitol-app-tui/            产品 TUI 客户端
    xylitol-app-print/          Print 客户端
    xylitol-rutis/              装配适配层；唯一 use rutis
  plugins/                      仅 Host 侧。Cargo 依赖只出现在根包
    xylitol-official-plugin-*/
```

箭头 `A → B` = A 的 `Cargo.toml` 依赖 B。

```mermaid
flowchart TB
  subgraph contracts["① 契约叶 · 框内三包互不依赖"]
    direction LR
    xylitol_plugin_sdk["xylitol-plugin-sdk<br/>词表 + 适配器"]
    xylitol_protocol["xylitol-protocol<br/>信封"]
    xylitol_tui["xylitol-tui<br/>引擎"]
  end

  subgraph plugins["② 官方 Host 插件 · 只依赖 xylitol-plugin-sdk"]
    xylitol_official_plugin_session["session"]
    xylitol_official_plugin_compaction["compaction"]
    xylitol_official_plugin_provider_openai["provider-openai"]
    xylitol_official_plugin_provider_anthropic["provider-anthropic"]
    xylitol_official_plugin_tools["tools"]
    xylitol_official_plugin_mcp["mcp"]
    xylitol_official_plugin_hooks["hooks"]
  end

  subgraph kernel["③ 执行权威 · 不接线"]
    xylitol_runtime["xylitol-runtime<br/>XyDriver + 准入 + ReAct 循环"]
  end

  subgraph surfaces["④ 载体与端"]
    direction LR
    xylitol_serve["xylitol-serve<br/>监听 + 占用 + mapping"]
    xylitol_app_print["xylitol-app-print"]
    xylitol_app_tui["xylitol-app-tui"]
  end

  subgraph root["⑤ 组合根"]
    xylitol["xylitol<br/>argv + 默认接线"]
    xylitol_rutis["xylitol-rutis<br/>唯一 use rutis"]
  end

  xylitol_official_plugin_session --> xylitol_plugin_sdk
  xylitol_official_plugin_compaction --> xylitol_plugin_sdk
  xylitol_official_plugin_provider_openai --> xylitol_plugin_sdk
  xylitol_official_plugin_provider_anthropic --> xylitol_plugin_sdk
  xylitol_official_plugin_tools --> xylitol_plugin_sdk
  xylitol_official_plugin_mcp --> xylitol_plugin_sdk
  xylitol_official_plugin_hooks --> xylitol_plugin_sdk
  xylitol_runtime --> xylitol_plugin_sdk
  xylitol_serve --> xylitol_runtime
  xylitol_serve --> xylitol_protocol
  xylitol_serve --> xylitol_plugin_sdk
  xylitol_app_print --> xylitol_runtime
  xylitol_app_tui --> xylitol_protocol
  xylitol_app_tui --> xylitol_tui
  xylitol_app_tui --> xylitol_plugin_sdk
  xylitol --> plugins
  xylitol --> xylitol_runtime
  xylitol --> xylitol_serve
  xylitol --> xylitol_app_print
  xylitol --> xylitol_app_tui
  xylitol --> xylitol_rutis
```

规则：

1. 插件只依赖 `xylitol-plugin-sdk`。插件之间不得 `use`。依赖用适配器声明，由组合根注入。
2. ① 三包互不依赖。`protocol` 不认识适配器，也不认识 `XyEvent`。`plugin-sdk` 不认识信封。
3. `xylitol-runtime` 只依赖 `xylitol-plugin-sdk`。不得依赖信封，不得依赖任何 `xylitol-official-plugin-*`。
4. `xylitol-serve` 依赖 runtime + protocol。mapping 只在 serve。Print 只依赖 runtime。
5. `xylitol-app-tui` 依赖信封、`xylitol-tui`、plugin-sdk。不依赖 Host 插件，不依赖 runtime。
6. 根包是组合根：唯一列出默认官方插件的 `Cargo.toml`。新增官方插件只改根包接线，不改 runtime。根包依赖 `xylitol-rutis`，不得直接 `use rutis`。
7. 每个 crate 内部仍守 `app → agent|infra → protocol`；`agent` 不得依赖 `infra`。不得按这三层各拆一 crate。

<a id="ROOTSRC"></a>

### 2.2.1 根 `src/`

组合根留在仓库根 package。`cargo run` 仍在仓库根。不得再做一个 `crates/xylitol/`。

```text
src/
  main.rs      tokio + run()
  lib.rs       embed + 精选 Xy* 再导出
  cli.rs       clap：无子命令 / tui / print / serve / --list-models
  wiring.rs    apply 七个官方插件，注入 runtime；没配置不 start mcp/hooks
  embed.rs     库用户走同一套 wiring，不经 argv
```

`cli/`、`wiring/` 是对应 `.rs` 的子模块。根包只接线：装配夹具可留在 `wiring::occupancy`；占用政策在 `xylitol-serve`；`XyInProcessDriver` 在 `xylitol-runtime`。

里面没有 TUI 画布、Print 渲染、HTTP 监听、ReAct、工具实现。那些在 `xylitol-app-*` / `xylitol-serve` / `xylitol-runtime` / `official-plugin-*`。不得把端实现嵌回 `src/app/tui`、`src/app/print`。

<a id="LLM"></a>

### 2.3 LLM 方言

两族方言各一官方插件：OpenAI 兼容、Anthropic Messages。DeepSeek overlay 留在 openai 插件。不要抽 `xylitol-llm` / `xylitol-types`。不要把 DTO 搬回主仓 `xylitol`。不要把 HTTP / 假实现 / 观测实现放进 `xylitol-plugin-sdk`。

准入：P1 两个方言；P6 Anthropic 构建不得编 `async-openai`。live-provider 路径解析在 `lab_responses_prompt_cache.rs`。

**词表薄留**（只把类型形状放进 `xylitol-plugin-sdk`，零 HTTP）：`AgentMessage::Llm` / `XyChunk`；产品 usage 形状；`HEURISTIC_BYTES_PER_TOKEN` + `heuristic_token_count`（不得用 tiktoken / RemoteCount）；`XyGenerateOptions` 字段与 `ObsSessionContext` 纯结构；`HeaderBag` 或直接 `serde_json::Map`；`tool_wire`；`Compat` 名字（`generic` / `deepseek` 配置词）。

**插件各自实现**：Responses / Completions / assembler / OpenCode attribution / DeepSeek overlay → openai 插件。Anthropic Messages → anthropic 插件。假模型直接 `impl XyModel`。工厂由根包接线时调用。

---

<a id="MAIN"></a>

## 3. 主线：rutis 接入

编译痛点在工具链，不在拆 crate。不为编译速度再拆 crate。

接入前读 [`/home/l8ng/Projects/__straydragon__/rutis/xylitol-x-rutis-temp`](/home/l8ng/Projects/__straydragon__/rutis/xylitol-x-rutis-temp)（阅读序：assessment → host-server → plugin-policy → rewrite → overhead → dependency）。工作树：上游 `3d0bc0d` / rutis 0.13.0。

0.13 事件是五分发（`emit` / `parallel` / `serial` / `waterfall` / `bail`）。不要按 0.11 写「跨进程只通知」。temp 里与本文件冲突的句子不要照抄：ReAct 不是官方插件；包名是 `xylitol-official-plugin-*`；组合根是仓库根包。

热路径延迟不劣化是整次接入的 go/no-go。契约叶与 `xylitol-app-*` 留下。

<a id="S1"></a>

### 3.1 S1 装配运行时

| 切片 | 状态 |
|---|---|
| S1 装配运行时 | 完成。现行形态见 [§2](#TARGET) |

<a id="S2"></a>

### 3.2 S2 运行时承载走 rutis

**停。** 另开再谈。

用户仍写配置 / MCP / hooks / skills。有配置时生命周期挂在 fiber 上。空配置行为不变（r2911 / r2912）。终端用户不写 `Plugin` trait。

没有常驻 `Ctx`，hooks / MCP 无法挂 fiber。assemble 末尾不得再 `shutdown`。

| 切片 | 状态 | 做什么 |
|---|---|---|
| S2a `Ctx` 常驻 | 停 | Host / Print 启动一次 `process_root`；去掉 assemble 返回前的 `ctx.shutdown()`；每进程只建一个根 `Ctx`。Print 仍不占端口 |
| S2b hooks 走总线 | | 有配置时脚本 hooks 挂 rutis 事件。空 hooks 仍不 `apply`、不拉进程。旧 hook BDD 必须绿 |
| S2c MCP 走 loader | | 有配置时连接生命周期走 loader。空列表仍无 mcp 前缀工具。`/reload` 语义不变 |

过：`just qa` 全绿。`workspace-assembly` 五场景绿。`host.describe` 仍含 fory-v3。

<a id="S3"></a>

### 3.3 S3 会话 = instance

**停。** 占用政策（linger、单写者）在 `xylitol-serve`。JSONL 树 / fork 仍在 session 插件。

| 切片 | 状态 | 做什么 |
|---|---|---|
| S3a 实例 API | | `xylitol-rutis` 包 `register_instance` / `host_key_in` / `in_instance`。单测：两 instance 互不可见；卸载零残留；在飞恰好失败一次。不改 Host |
| S3b 槽 = instance | | `materialize_writer_at` 在会话 instance 登记写者。删 `HostState.sessions` 作为物化权威。occupancy BDD 继续绿。双会话互不可见、结束零残留走 propose |
| S3c 删基线 | | 删 `RuntimeBaseline` / `build_baseline*` / `AgentBuilder::build_baseline`。调用点一次改完，不留 shim。Print 仍能 `AgentEnd` |
| S3d freeze → generation | | 工具表冻结挂 generation。超时仍冻结。`arm_tool_freeze` BDD 绿 |

Kill：实例回收与快照/恢复冲突 → 隔离退回手写 HashMap，保留 S3a API。

<a id="S4"></a>

### 3.4 S4 多语言叶 + dylib

**停。** 动态加载。官方 Host 插件在此之前保持静态 crate。必须交付或降为实验特性，做到那一刀再定。不得在 S2/S3 预埋加载器空壳。

<a id="S5"></a>

### 3.5 S5 产品 TUI：客户端订阅

**停。** 产品 TUI 仍走信封，不是 Host 侧 fiber。S5 只改端进程内合帧节奏。不改 `POST /rpc` + `WS /rpc` + fory-v3。不得做 `xylitol-official-plugin-tui`。端若依赖 `xylitol-rutis`（仍不得 `use rutis`）须先 propose 修订 r2910。验收含 `just qa-e2e`。

---

<a id="SCOPE"></a>

## 4. 产品范围

核 = 必须在现行 crate 图里。瘦 = 代码留、feature-gate 或下沉。删用户可见面须改 `.feature`。未列入本节的方向见 [§5](#DEFER) 或 GitHub [#28](https://github.com/StrayDragon/xylitol/issues/28)。

**核 — 用户侧**：TUI 默认 attach 本机 Host；Print；embed `XyDriver`；slash；bang；`$skill`；会话树；滚动区 / 固定区；换模 / thinking；图像粘贴 / 读图；designing（`xylitol tui designing`，仅 debug 构建；合约 [c2930](llmanspec/changes/archive/2026-10-18-c2930-add-tui-preview/)）。

**核 — Host**：监听 + 四象限信封（HTTP 不得删）；`XyDriver` 会话槽 / 单写者 ReAct；内置工具；Trust；会话持久化 / 导出；压缩；OpenAI 兼容 + Anthropic；MCP（有配置才装配）；hooks；`edit` 内部 patch。

**核 — 观测**：`provider-trace.jsonl` + fastrace；`xylitol.log`（永不 stdout）；`just obs-*`；inspect 脚本；`otel` feature（default 开，配置默认 none，关 feature 真能关）。不得再引入 `tracing` 双栈。

**瘦**：serve `stop` 只印 SIGTERM；`server` / `mcp` feature 可关（default 仍开）；`jsonschema` / `minijinja` / `schemars` 下沉到使用方；`cli = []` 不单独发明空开关。`incremental = false`。

---

<a id="DEFER"></a>

## 5. 延后

现在不做。认领走 GitHub [#28](https://github.com/StrayDragon/xylitol/issues/28)。不得为这些方向预埋空壳 crate。

| 项 | 内容 |
|---|---|
| D2 | `AgentMessage` / `EnvMessage` / `AgentPart` 不加 `Xy`。`Xy*` 留给适配器与叶。`AgentPart` 若将来加前缀应叫 `XyPart` |
| D6 | gpui、LSP、DAP、Sub-Agent、语音端 / 全双工 realtime、键位可视化、Inspect wasm / 框选 |
| D7 | schedule / cron 唤醒源。唤醒源走准入，不换内环 |
| D8 | samply suite、Jaeger。`agent_demo*` / `host_loop` 留在 `xylitol-tui`。maka 对照留 `docs/research/` |

信封标识：`crates/xylitol-protocol/src/v3/`；fbs `namespace xy.wire.v3`；content-type `application/x-fory-v3`；`WIRE_FORMAT_FORY_V3`。

---

## 6. 来源

| 主题 | 路径 |
|---|---|
| 团队怎么做 | `.claude/skills/xylitol-dev/SKILL.md` |
| STC | `/home/l8ng/Projects/__straydragon__/STC-Simplified-Technical-Chinese` |
| 代码架构 | `src/AGENTS.md` |
| 产品主线 | `docs/architecture/产品分层总览.md` |
| 编译调优 | `~/.config/llman/skills/rust-build-tune/` |
| rutis 方向 | `/home/l8ng/Projects/__straydragon__/rutis/xylitol-x-rutis-temp/` |
| crate 图细则 | 同上 `skills/xylitol-rutis-rewrite/references/` |

---

<a id="FACTS"></a>

## 附录 A. 现行 workspace

成员：根包 + plugin-sdk / protocol / runtime / serve / app-tui / app-print / xylitol-tui / xylitol-rutis / 七个官方插件。`vendor/gherkin` 是 `[patch.crates-io]`。根 default = `cli, tui, otel, server, mcp`。

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
