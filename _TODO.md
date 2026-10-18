# xylitol 规划脊柱

本文件是规划真值，进 git 便于 diff。不升格为 `AGENTS.md` / spec。
进度只改本文件。稳定规则仍写进 `AGENTS.md`。行为合约仍写进 `llmanspec/`。

交流风格: ../STC-Simplified-Technical-Chinese

## 现在

下一刀：**W1.3b**（otel 代码路径，default 仍开）→ **W1.4**（`image` feature-gate，default 仍开）→ **W2.0**（改 AGENTS crate 图）。

| 要做的事 | 去哪 |
|---|---|
| 目标 crate 图、双写、ai-bridge | [§2](#TARGET) |
| 未完成批次 | [§3](#BATCH) |
| S0 必须带走什么 | [§4](#CORE) |
| 预览（不进本轮） | [附录 C](#DESIGN-WASM) / [附录 D](#UI-WORKFLOW) |

怎么读：先本节，再 §2 总图，再 §3 未完成表。附录 A/B 是盘点与完成记录，主线不依赖它们。

顺序不得颠倒：工具链（WS0，已做）→ feature 真能关 → 契约叶 + 拆 ai-bridge HTTP → 新 crate 重写、旧 `src/` 双写。不为编译速度拆 crate。本轮不引入 rutis。

Host 侧 crate 边界 = 插件的替换 / 开关 / 生命周期。客户端是独立 `xylitol-app-*`，不是 Host 插件。
产品 TUI 默认 attach 本机监听（`xylitol-serve`）。MCP 有配置才装配，不得当死代码删。
禁止预埋 gpui / LSP / DAP / Sub-Agent 空壳。`incremental = false` 保持。default features 仍含 `cli,tui,otel,server,mcp`，只让关掉时真能关。

---

## 目录

| 节 | 内容 |
|---|---|
| [§1](#constraints) | 固定约束与 SDD 路由 |
| [§2](#TARGET) | 目标 crate 图、双写、ai-bridge |
| [§3](#BATCH) | W* 未完成批次 |
| [§4](#CORE) | 核心三面（带走范围） |
| [§5](#KEEP) | KEEP / SLIM / DELETE |
| [§6](#NAMES) | 改名对照 |
| [§7](#NO) | 本轮不做 |
| [§8](#AGENTS) | 抽出契约叶前必须改的 AGENTS / spec |
| [§9](#SRC) | 来源 |
| [附录 A](#FACTS) | 现状盘点 |
| [附录 B](#附) | 已完成批次说明 |
| [附录 C](#DESIGN-WASM) | 预览：框选覆盖层 |
| [附录 D](#UI-WORKFLOW) | 预览：工作流原语 |

一事只在一节写全，他处用指针。

---

<a id="constraints"></a>

## 1. 固定约束

1. 行为不变的组织改动可跳过 llman-sdd 生命周期。live `.feature` 主语 / 路径过期时，直接改 feature，再跑 `llman-sdd validate`。
2. 产品可观察行为变化必须走完整 SDD：explore → propose → apply → verify → archive。
3. 任何批次合并前必须 `just qa`。触及真终端时再加 `just qa-e2e`。
4. 试验脚本只许 `lab_` 前缀。不得再引入 `evidence_` / `experiment_`。
5. 提交用 Conventional Commits。未要求则不自动 commit。
6. `AGENTS.md` 只收硬约束 / 稳定原则。不得把本文件进度抄进去。移除即改写，不留墓碑。
7. 同 crate 的不同 worktree **禁止**共用 `CARGO_TARGET_DIR`。
8. 改名一次性改调用点。不留兼容别名，不留 `#[deprecated]`。

### 1.1 SDD 路由

| 类别 | 何时 | 做什么 |
|---|---|---|
| skip-SDD | 纯组织 / 工具链 / 编译隔离，且产品行为不变 | 直接改代码 + `just qa` |
| validate-only | 组织演进导致 live `.feature` 过期 | 直接改 `.feature` + `llman-sdd validate` + `just qa` |
| must-SDD | 改了用户能看见或默认二进制含什么 | 完整 change 生命周期 |

发现「以为组织改动、实际改了行为」时立刻停，转 propose。

提交：工具链 `build:` / `chore:`；重写 `feat:` / `refactor:`；行为 `fix:`；SDD `chore(sdd):` / `archive(sdd):`。
不碰（除非该批次点名）：`docs/architecture/`、`_INTERVIEW/`、`delayed-changes/legacy/`。改行为才动 `llmanspec/`。

---

<a id="TARGET"></a>

## 2. 目标形态（重写 + `src/` 双写）

新 crate 按契约重写，禁止 `pub use` 旧 `src/` 模块凑编译。旧 `src/` 继续出产品二进制，对照 [§4](#CORE) 后再删对应旧模块。不留兼容 shim。

三层分名：**角色**（执行权威）/ **服务提供方**（服务目录，`xylitol-runtime`）/ **载体**（直调或 WS，`xylitol-serve`）。crate 名只表达其中一层。不要 `xylitol-host`、`xylitol-client`、`xylitol-server-host`。

`XyDriver` 是 runtime 侧应用协议。客户端只通过 attach（信封或直调）消费。S1 rutis 只装配 runtime 进程内的 Host 侧插件。`xylitol-app-tui` 不是 rutis fiber。

### 2.1 总图

```text
crates/
  xylitol-core/                 词表 + ports + errors
  xylitol-protocol/             四象限信封 + fory-v3 + PROTOCOL_VERSION
                                可含很薄的 attach client 半；过厚再拆 xylitol-attach
  xylitol-runtime/              执行权威 + 服务目录 + 组合根 / trust
                                进程内直调合法。S1 在此换 rutis 装配
  xylitol-serve/                只把 runtime 挂到 HTTP/WS。不是角色定义
  xylitol-tui/                  TUI 引擎 lib。零引用产品。不改名
  xylitol-app-tui/              产品 TUI 客户端。从 src/app/tui 重写，独立分发
  xylitol-app-print/            Print 客户端。从 src/app/cli print 重写
  xylitol-ai-bridge/            LLM 叶，无 HTTP。拆分见 2.2
  xylitol-rutis/                S1 才建；唯一 use rutis
  xylitol/                      极瘦 argv：选 app-tui / app-print / serve
  plugins/                      仅 Host 侧能力，由 runtime 装配
    xylitol-core-plugin-react/
    xylitol-core-plugin-session/
    xylitol-core-plugin-compaction/
    xylitol-core-plugin-provider-openai/
    xylitol-core-plugin-provider-anthropic/
    xylitol-core-plugin-tools/
    xylitol-core-plugin-mcp/
    xylitol-core-plugin-hooks/
```

| 角色 | crate | 说明 |
|---|---|---|
| TUI 引擎 | `xylitol-tui` | lib。通用组件与合帧。demo ≠ 产品 |
| 产品 TUI | `xylitol-app-tui` | 独立分发的客户端。依赖引擎 + protocol/attach。`cargo build -p xylitol-app-tui` |
| Print | `xylitol-app-print` | 独立客户端。同进程直调 runtime 是载体，不是 Host 插件 |
| 操作器 | `xylitol-runtime` | 服务目录；Print / 嵌入只链这里 |
| 监听 | `xylitol-serve` | 产品 TUI 默认 attach 本机这一层 |
| 进程入口 | 根包 `xylitol` | 只解析 argv |

新客户端 = 新 `xylitol-app-*`，只依赖协议 + attach，不进 `plugins/`。
Host 侧插件只依赖 `xylitol-core`（必要时只读 protocol DTO），插件不横向依赖。
方言插件额外依赖 `xylitol-ai-bridge` 叶。
`xylitol-app-tui` 额外依赖 `xylitol-tui`。
`runtime` → core + protocol。`serve` → runtime + protocol。极瘦 bin → 所选 app 或 serve。

每个 crate 内部仍守分层：`app → agent|infra → protocol`；`agent` ↛ `infra`；信封 ↛ ports。
禁止按层各拆一 crate。禁止 `xylitol-domain`。禁止预埋 gpui / LSP / DAP 空壳。
`vendor/gherkin` 保持上游包名 + path patch。

双写：

| 轨 | 做什么 | 编译 |
|---|---|---|
| 旧 | `src/` 出产品 `xylitol` | 照旧 |
| 新 | `xylitol-runtime` / `xylitol-serve` / `xylitol-app-*` 按信封与 `.feature` 重写 | `cargo build -p …` |

共享只允许：跨端协议形状、`.feature` 句子、固定区词汇、DESIGN 色板。
每个 worktree 必须独立 `CARGO_TARGET_DIR`。
Harness：引擎按键→状态在 `xylitol-tui`；产品场景 / 假 Host 预览在 `xylitol-app-tui`。

S0 先做：契约叶、ai-bridge、工具链。不要把 `src/app` 原样搬成 `xylitol-host` / `xylitol-client`。

<a id="AI-BRIDGE"></a>
<a id="W2.2b"></a>

### 2.2 ai-bridge 拆分

现在的 `xylitol-ai-bridge` 把 LLM 叶和两套 HTTP 捆在一起。S0 必须拆。不要抽 `xylitol-llm-types`，不要把 DTO 搬回主仓。

```text
xylitol-ai-bridge                 叶：DTO / accounting / WirePolicy / FakeProvider / HeaderBag
        ▲                                    ▲
plugin-provider-openai            plugin-provider-anthropic
（async-openai；Responses/Completions / deepseek）   （reqwest Messages）
```

`AiBridgeLlmAdapter` 留在叶上。主仓 `src/infra/provider` 的 `AdapterXyModel` / `map.rs` 搬进对应方言插件，不再包第三层。

| 块 | 约行 | 热度 | 去向 |
|---|---|---|---|
| DTO（message / chunk / json_parse） | 594 | 流式块每 token | 叶 |
| accounting | 189 | compaction `heuristic_token_count` | 叶。禁止 tiktoken / RemoteCount |
| usage 类型 | 359 | 响应结束 | 类型留叶；`from_openai` / `from_anthropic` 跟方言 |
| WirePolicy + defaults | 206 | 请求装配 | 叶 |
| thinking 解析 | 480 | 请求前一次 | 解析留叶；`apply_thinking_*` 跟方言 |
| FakeProvider | 571 | 测 / 预览 | 叶 |
| error / hooks HeaderBag | 78 | 契约 | 叶 |
| tool_wire | 115 | 工具名过网关 | 叶 |
| OpenAI Responses + Completions + client + assembler + hooks mw | ~3510 | 热点；`async-openai` | `plugin-provider-openai` |
| Anthropic Messages | 1194 | 热点；reqwest | `plugin-provider-anthropic` |
| dialect/deepseek | 71 | 冷 | openai 插件 |
| trace + obs_session + attribution | 1344 | 观测 | 方言插件；共享 attr 可留叶 |
| config live-provider 路径 | 115 | lab | 叶 config 或 lab，不进内核 |
| `src/infra/provider` map 壳 | 另计 | DTO→`XyChunk` | 各方言插件内 |

行数来自 `crates/xylitol-ai-bridge/src` 的 `wc -l`，合计 9172。准入：P1 两个方言；P6 Anthropic 构建不得编 `async-openai`。

不要：按文件再拆 assembler；DeepSeek 单独 crate；等 S1 再拆 HTTP；把 provider-trace 放进内核。

批次：W2.2b，在 W2.2 之后、W2.3 之前。`package-ai-bridge*.feature` 若把单包 HTTP 写成主语，validate-only 改句子。

---

<a id="BATCH"></a>

## 3. 批次

编译痛点在工具链，不在拆 crate。WS0 已完成，记录见 [附录 B](#附)。

### 3.1 现在做（WS1 剩余）

| ID | 项 | SDD | 说明 | 完成 |
|---|---|---|---|---|
| W1.3b | 只修 otel 代码路径，default 不动 | skip-SDD | leftover [#27](https://github.com/StrayDragon/xylitol/issues/27) | [否](#W1.3b) |
| W1.4 | `image` crate feature-gate，default 仍开 | skip-SDD | 模块小、依赖重 | 否 |
| W1.6 | 审计 `debug_fixtures`（~653 行）：测试夹具下沉 `tests/` | skip-SDD | 若砍用户可达调试面 → must-SDD | 否 |

W1.8 / W1.9 / W1.10（`wire` 改名、双 `model`、host 三义）随 W2 搬 crate 时做，不单独全树 rename。

本轮默认不做：W1.1b（default 去掉 `server`）、W1.2b（默认二进制不含 MCP）、W1.3（`otel` 出 default）、W1.11（gherkin 改名）。

已完成：W1.0 带走范围、W1.1 `server` 可关、W1.2 `mcp` 可关、W1.5 hooks、W1.7 session export、W1.12 删 CLI `resources` 与 `serve install`。说明见 [附录 B](#附)。

### 3.2 WS2 契约叶 + 重写双写

验收：边界就位 + `just qa` 全绿。编译变快只是副产品。回退靠 git revert 该批。

| 批 | 抽什么 | 准入 | SDD | 完成 |
|---|---|---|---|---|
| W2.0 | 改写 `src/AGENTS.md` 与根 `AGENTS.md`：crate 图见 [§2](#TARGET) | 否则代理按旧规则回滚 | skip-SDD | 否 |
| W2.0b | `layer-architecture.feature` 组织句：多 crate；分层纪律留在 crate 内 | spec 钉了组织 | validate-only | 否 |
| W2.1 | `xylitol-core`（词表 + ports + errors） | P5 | skip-SDD + validate-only | 否 |
| W2.2 | `xylitol-protocol`（信封 + fory 帧）；顺手去掉 `wire` 撞车名 | P5 | 同上 | 否 |
| W2.2b | 拆 `xylitol-ai-bridge`：叶留下、HTTP 进方言插件。见 [§2.2](#AI-BRIDGE) | P1 / P6 | skip-SDD + validate-only | 否 |
| W2.3 | Host 侧 `plugin-mcp` / `-hooks` / `-provider-*` | P1 / P2 / P6 | skip-SDD | 否 |
| W2.4 | 重写 `xylitol-runtime` + `xylitol-serve`；旧 `src/app/server` 与 `app/core` 仍出产品 | P2 / P4 / P5 | skip-SDD + 行为差 must-SDD | 否 |
| W2.5 | 重写 `xylitol-app-tui` + `xylitol-app-print`；旧 `src/app/tui` / print 仍出产品 | P1 / P2 | 同上；TUI 加 `just qa-e2e` | 否 |
| W2.6 | 每批后 `just qa` | 门禁 | 强制 | 否 |

不创建：`xylitol-rutis`、`xylitol-host`、`xylitol-client`、`xylitol-core-plugin-tui`、空 LSP/DAP crate、dylib 加载器。

### 3.3 WS3 S1+ rutis（只记靶心）

| 阶段 | 动作 | 本轮 |
|---|---|---|
| S1 | 引 rutis 只做组装运行时；热路径延迟不劣化 | 不做。这是整次重写的 go/no-go |
| S2 | 会话 = instance；删 `RuntimePorts` 手工物化 | 不做 |
| S3 | hooks→bus、MCP boot→loader、freeze→generation | 不做 |
| S4 | 多语言叶 + dylib | 不做 |
| S5 | TUI 作客户端事件订阅者 | 不做 |

S1 失败则整次 rutis 接入可中止。契约叶与 `xylitol-app-*` 仍留下。

---

<a id="CORE"></a>

## 4. 核心三面（S0 带走范围）

**三面 = 用户侧 + Host/服务端 + 可观测**。其余默认不进第一批 crate 图。

推荐列：`核`（S0 必须搬走）/ `瘦`（代码留、先 feature-gate 或下沉测试）/ `推迟` / `删`。删用户可见面须改 `.feature`。

### 4.1 用户侧

| 能力 | 落点 | 现状 | 推荐 |
|---|---|---|---|
| TUI 默认入口（attach 本机 Host） | `app/tui` + `xylitol-tui` | 开箱 | 核 |
| Print 一轮 | `app/cli` print | 已开放 | 核 |
| 嵌入 `XyDriver` | `embed` / `app/core` | 已开放 | 核 |
| slash 词表 | `app/product_commands` | session-* / model / reload / trust / theme / mcp / exit | 核 |
| bang `!` / `!!` | `app/bang_exec` | 产品 | 核 |
| `$skill` 注入 | TUI + resource loader | 产品 | 核 |
| 会话树 / fork / clone / resume / new / name | Driver + TUI | 产品 | 核 |
| 滚动区 / 固定区 / activity 折叠 | TUI + DESIGN.md | 产品 | 核 |
| 换模 / thinking run 绑定 | `/model` + footer | 已落地 | 核 |
| 图像粘贴 / 读图 | `infra/image` + bang | 主线；依赖重 | 核（compile 见 W1.4） |
| `/debug` 场景 | `app/debug_fixtures` | 仅 debug_assertions | 瘦（下沉 tests，见 W1.6） |
| 键位可视化 / `/hotkeys` | 未做 | GH #23 | 推迟 |
| gpui 桌面 | 未做 | GH #12 | 推迟 |

### 4.2 Host / 服务端

| 能力 | 落点 | 现状 | 推荐 |
|---|---|---|---|
| Host 监听 + 四象限信封 | `app/server` + `protocol/wire` | 产品 TUI 默认依赖；HTTP **不得删** | 核 |
| `XyDriver` 会话槽 / 单写者 ReAct | `app/core` + `agent/runtime` | 主线 | 核 |
| 内置工具 read/write/edit/bash/grep/find/ls/todo/ask | `infra/tools` | 开箱 | 核 |
| Trust 门禁 | `infra/trust` + CLI | 开箱 | 核 |
| 会话持久化 / 导出 HTML+JSONL | `infra/session` + `session_export` | 主线 | 核 |
| 自动压缩 + `/session-compact` | `agent/compaction` | 主线 | 核 |
| OpenAI 兼容 + Anthropic | `xylitol-ai-bridge` + `infra/provider` | 主线 | 核 |
| MCP（配置才装配） | `infra/mcp` | default feature 仍开 | 核（目标 `plugin-mcp`；未配置不连） |
| hooks | `infra/hooks` + embed | 空配置零成本 | 核（目标 `plugin-hooks`） |
| `infra/tools/patch` | `edit` 内部：模糊匹配 + diff | 无独立工具名 | 核（随 `edit` 进 `plugin-tools`） |
| serve 生命周期 `stop` | 只印 SIGTERM 说明 | 无 lock-file | 瘦 |

### 4.3 可观测侧

| 能力 | 落点 | 现状 | 推荐 |
|---|---|---|---|
| `provider-trace.jsonl` + fastrace 过程树 | `infra/observability` | 对照底座 | 核 |
| `xylitol.log`（`log`，永不 stdout） | `app/cli/logging` | 排障 | 核 |
| 窄读 `just obs-summary` / `obs-recent` / `obs-tui-lag` | `scripts/inspect_provider_trace.py` | 开发核 | 核 |
| inspect：requests / turns / lag / lifecycle / channel | 同脚本 | 人跑 | 核 |
| 可选 OTLP（`[otel]` 默认 none） | feature `otel` | default 仍开；lane/采样未做 | 核（不扩；W1.3b 只修路径） |
| samply `profile-suite` | `scripts/profile_tui_suite.py` | 维护 | 推迟（不进 crate 图） |
| 自研检视台 / Jaeger 必选 | 无 | GH #27 leftover | 推迟 |

### 4.4 暂不带走

| 项 | 说明 | 推荐 |
|---|---|---|
| gpui / LSP / DAP / Sub-Agent / Loop / Computer-Use | 未实现；禁止空壳 crate | 推迟 |
| delayed-changes 草案 | 不进 S0 文件搬家 | 推迟 |
| `agent_demo*` / `host_loop` | 引擎演示，非产品 SSOT | 留在 `xylitol-tui` |
| maka research | S1 对照 | 留 `docs/research/` |
| 23 个集成测试二进制 | 不删测试 | 瘦（share-generics 已做） |
| 预览（假 Host + 预览页） | [附录 D](#UI-WORKFLOW)；机制 [附录 C](#DESIGN-WASM) | 推迟 |

---

<a id="KEEP"></a>

## 5. KEEP / SLIM / DELETE

编译隔离与产品删除必须分开。default features 不动，只让关 feature 真能关。

**KEEP（主线，S0 后仍默认装配）**

1. Print + 产品 TUI。
2. `xylitol-runtime` 操作器 + `xylitol-serve` 本机监听（default 含 `server`）。
3. 跨端信封 + `XyDriver` / `XyEvent`。
4. ReAct、session store、compaction、OpenAI 兼容 / Anthropic、内置工具、trust。
5. MCP（运行时：有配置才装配；目标 `xylitol-core-plugin-mcp`）。
6. hooks（embed / `hooks_config` 已是产品口）。
7. session export。
8. `xylitol-ai-bridge`。
9. fastrace + `log`（禁止再引入 `tracing` 双栈）。

**SLIM（compile 隔离；default 建议仍开）**

1. HTTP Host 代码（~4.3k）+ 始终编译的 wire：修好 `server` feature。
2. `rmcp` → `mcp` feature / 未来 plugin crate。
3. `image` crate → feature，default 仍开。
4. `jsonschema` / `minijinja` / `schemars`：S0 下沉到使用方 crate，不预删。
5. `cli = []`：极瘦进程入口和 `xylitol-app-print` 给真实依赖，不单独发明空开关。
6. 23 个集成测试二进制：不删测试；靠 `-Zshare-generics` 降重复。

**DELETE**

1. 不新增 gpui / Web 控制台 / DAP / LSP / Sub-Agent 空壳。
2. `debug_fixtures`：确认全是测试夹具后再下沉或删生产模块。
3. 不删 `vendor/gherkin`。
4. 不删 salvo / tokio-tungstenite 的产品用途。
5. 不提供 CLI `xylitol resources`。
6. 不提供 `xylitol serve install`。

---

<a id="NAMES"></a>

## 6. 今日非标准名字

只列现状不标准者。S0 搬 crate 时顺手改。

| 现状 | 问题 | 目标 |
|---|---|---|
| `src/protocol/wire/`、`app/server/wire_v3`、`host_client/wire_v3_client` | `wire`（线协议）与 `wiring`（组合根接线）撞车 | crate `xylitol-protocol`；中文「跨端协议」 |
| `HostState` / 角色 host / `HostClient` | host 一词三义 | 角色保持「执行权威」；提供方称 HostServer；进程内 / WS 称 carrier。`XyDriver`、`HostClient` 保留 |
| `protocol/model*` 与 agent 侧 `model` | 双 `model` 顶栏 | 契约侧对齐 `XyModel`；agent 侧用选择 / 装配语义 |
| 新公开类型第三套品牌 | `Xy*` 与 `AgentMessage` / `Command` / `Event` 混用本身合法 | `Xy*` = 库入口契约；`AgentMessage` = protocol 根词表；Command/Event = 信封载荷。新类型不得再发明第四套 |
| 根包仍叫 `xylitol` 且含全部 lib | 布局不标准 | 根缩成极瘦 argv；Print → `xylitol-app-print`；TUI 产品 → `xylitol-app-tui`；Host 侧插件在 `crates/plugins/` |
| feature 名 `server` | 和 `xylitol-serve` 载体对应 | Cargo feature 可仍叫 `server`；不得改 attach 行为 |

不改：`xylitol-tui` 引擎包名、`XyDriver`、`XyEvent`、`XyModel`、`vendor/gherkin`。`xylitol-ai-bridge` 叶包名保留；HTTP 见 [§2.2](#AI-BRIDGE)。

---

<a id="NO"></a>

## 7. 本轮明确不做

1. gpui / Web 控制台 / DAP / LSP / Sub-Agent 空壳 crate。
2. S1 引入 rutis，以及 S2–S5。
3. 按 layer 拆 crate。
4. 抽 `xylitol-domain`；把 LLM 叶从 bridge 搬进主仓。
5. 仅为编译速度拆 crate。
6. 全局 Cranelift。
7. 跨 worktree 共享 `CARGO_TARGET_DIR`。
8. 把 `AGENTS.md` 写成进度板。
9. 跳过 `just qa`。
10. 为对称预拆 `xylitol-core-plugin-lsp`、`xylitol-app-gpui` 空壳。
11. 新建 `xylitol-host`、`xylitol-client`、`xylitol-core-plugin-tui`。
12. 新 crate `pub use` 旧 `src/` 模块凑编译。

---

<a id="AGENTS"></a>

## 8. 抽出契约叶前必须改的 AGENTS / spec

改写只换稳定规则，不写进度，不留墓碑。

1. `src/AGENTS.md`「单 crate 逻辑分层（不为分层拆 crate）」→ 目标图见 [§2](#TARGET)。Host 侧 crate 边界 = 插件替换 / 开关 / 生命周期。客户端是 `xylitol-app-*`。禁止按层各一 crate。继续禁止 `xylitol-domain`。
2. `protocol/{wire, ports, model, session}` 同一顶栏 → 信封进 `xylitol-protocol`，ports / 词表进 `xylitol-core`，session 实现进 plugin。信封 ↛ ports 升为跨 crate 规则。
3. 「组合根才同时 import agent + infra」落点从 `app/core` → `xylitol-runtime`。S1 前仍是手工装配。
4. 根 `AGENTS.md` 与 `src/AGENTS.md` 同步。`crates/` 角色表换成 [§2](#TARGET) 总图。
5. `layer-architecture.feature` purpose「单 crate」→ validate-only 直接编辑。
6. 「开箱 vs 配置后置」拆成两句：监听主线且 feature 真可关；MCP 配置后置且 compile 也可关。
7. `RunId` 不得进 `XyEvent` / 信封。规则本身保留。
8. `xylitol-core-plugin-*` 是官方静态 Host 侧 crate，不是插件市场，不是 dylib。客户端不进该前缀。

不改的硬规则：Trust 门禁项目本地资源；工具开箱 allow-all；MCP 有配置才装配；产品 TUI 默认 attach 本机监听；监听 ≠ 操作器定义；改名不留别名；`just qa`；worktree 独立 `CARGO_TARGET_DIR`。

---

<a id="SRC"></a>

## 9. 来源

| 主题 | 路径 |
|---|---|
| 团队怎么做 | `.claude/skills/xylitol-dev/SKILL.md` |
| 代码架构 SSOT | `src/AGENTS.md` |
| 产品主线 | `docs/architecture/产品分层总览.md` |
| spec 组织句 | `llmanspec/specs/layer-architecture/layer-architecture.feature` |
| 编译调优 | `~/.config/llman/skills/rust-build-tune/` |
| rutis 重写方向 | `/home/l8ng/Projects/__straydragon__/rutis/xylitol-x-rutis-temp/skills/xylitol-rutis-rewrite/` |
| 拆分闸门 | 同上 `references/split-principles.md`（P1–P6） |
| 目标 crate 图 | 同上 `references/crate-graph.md` / `crate-naming.md` / `crate-split-detail.md` |
| 迁移阶段 | 同上 `references/migration-stages.md` |

---

<a id="FACTS"></a>

## 附录 A. 现状盘点

盘点日期：2026-10-17。Workspace 成员：根包 `xylitol` + `crates/xylitol-tui` + `crates/xylitol-ai-bridge`。`vendor/gherkin` 是 `[patch.crates-io]`。`cargo metadata` resolve 节点约 594 个。

| 区域 | 约行数（tokei Code / 文件总行） | 来源 |
|---|---|---|
| `src/` 合计 | Code ~10.0 万 / 总行 ~11.8 万 | 356 个文件 |
| `src/app` | Code ~5.4 万（其中 `app/tui` Code ~3.6 万） | 最大单块 |
| `src/agent` | 总行 ~2.1 万 | ReAct / capabilities / prompt |
| `src/infra` | 总行 ~2.1 万 | ports 实现 |
| `src/protocol` | 总行 ~1.3 万 | 词表 + ports + wire |
| `crates/xylitol-tui` | Code ~2.4 万 | 零引用主 crate |
| `crates/xylitol-ai-bridge` | Code ~0.8 万 | 主 crate 必选 path 依赖 |
| `tests/` | Code ~3.8 万 | 含 BDD |

根 `Cargo.toml` default = `cli, tui, otel, server, mcp`。

| 项 | 事实 |
|---|---|
| `cli = []` | 空 feature，不裁依赖 |
| `server` | optional；W1.1 后可关（default 仍开） |
| `mcp` | optional `rmcp`；W1.2 后可关（default 仍开；运行时仍需配置才装配） |
| `image` / `fory` / `jsonschema` / `minijinja` / `schemars` / `fastrace` | 非 optional，始终进主 crate |
| `salvo` / `tokio-tungstenite` / `xylitol-tui` / OTel 三件套 | 随 feature 可选 |

dev：`opt-level = 0`，依赖 `opt-level = 3`，`debug = "line-tables-only"`，`incremental = false`，`codegen-units = 16`。
release：`opt-level = "z"`，`lto = "thin"`，`strip = "symbols"`，`panic = "abort"`，`codegen-units = 1`。
`.cargo/config.toml`：Linux mold + `-Zshare-generics`（同一份 rustflags）。Cranelift 不用（`aws-lc-rs` / `ring`）。
主 worktree 用仓库内 `target/`；额外 worktree 须 `eval "$(just cargo-wt-env)"`。

产品主线：对话循环、内置工具、会话持久化 / 导出、压缩、两家模型 API、Trust、图像附件。默认入口：产品 TUI attach 本机 Host（未在听失败）。后置：MCP。
spec `@req:r1532` 要求 server 应用面。未实现：gpui、Web 控制台、DAP、LSP、Sub-Agent（索引 [#28](https://github.com/StrayDragon/xylitol/issues/28)）。

抽出契约叶之前必须改 AGENTS（现行规则仍写「单 crate」）。见 [§8](#AGENTS)。

---

<a id="附"></a>

## 附录 B. 已完成批次说明

无附加则完成列只写 是 / 否 / —。有提交或落点才在此按 ID 开节（标题即锚点）。

#### W0.1

脚本：`python3 scripts/lab_compile_baseline.py`（`--cold` = `cargo clean -p` 三成员后重编）。本机 `CARGO_TARGET_DIR=~/.cache/cargo-targets/xylitol/3a096cc8d643`。

| 条件 | 墙钟 | du target | bin |
|---|---|---|---|
| 指纹无改 | 0.2s | 8.1G→（后 `--cold` 落到 4.6G） | 260.3M |
| `--cold` + sccache 全命中 | 1.7s | 4.6G | 260.3M |
| `--cold` 且 `env -u RUSTC_WRAPPER` | **16.0s** | 4.6G | 260.3M |

对照用最后一行。timings HTML：`.tmp/lab_compile_baseline/20261017T141652Z/`。`scripts/lab_compile_baseline.py` 不进 qa。全部 W* 完成后删除，或把 summary 进 freeze `research/` 再删脚本。禁止升格 just recipe。

#### W0.2

`073eccfc`。`.cargo/config.toml` Linux mold；rustflags 与 W0.4 同份。CI `apt install clang mold`。

#### W0.3

`scripts/cargo_worktree_env.sh`：有 sccache 则导出 `RUSTC_WRAPPER=sccache`。

#### W0.4

同 W0.2（`073eccfc`，`-Zshare-generics`）。

#### W0.5

`incremental = false`。

#### W0.6

`cargo clean`；`debug/build` 另补删。

#### W0.7

`Cargo.toml` 注释改指 `.cargo/config.toml`。

#### W1.1

同 W0.2（`073eccfc`）。

#### W1.2

`b3acd3f8`。default 仍含 `mcp`。

#### W1.3b

产品 leftover [#27](https://github.com/StrayDragon/xylitol/issues/27)；本项只修 compile 路径，default 仍开 `otel`。

#### W1.12

删顶层 `xylitol resources` 与 `serve install`。发现仍走 `DefaultResourceLoader`。合约：`cli-entry` / `runtime-resource-discovery` / `runtime-config`。

#### CLEAN

`a77a96c5`：删 `designing/`、一次性 lab、crate `PI_DELTAS`；cccc 内联；Palette 门。

#### CONFIGS

`e11f1e76`：`configs/secret.env.example`。

#### DOC

`c6dfaec8`。FR [#11](https://github.com/StrayDragon/xylitol/issues/11)–[#27](https://github.com/StrayDragon/xylitol/issues/27)；roadmap [#28](https://github.com/StrayDragon/xylitol/issues/28)。research 留 `maka-runtime-kernel-lessons-2026.md`。政策已写入 `docs/AGENTS.md`。

---

<a id="DESIGN-WASM"></a>

## 附录 C. 预览：框选覆盖层（机制）

产品和引擎组件出现在共享画面上，画面上有框选覆盖层。人和 LLM 对着同一套 UI、交互、状态讨论。TUI 只有一层单元格，框不能画进 ANSI。未落地；不建 `designing/`、不装 wasm target、不进本轮主线。

### C.1 问题拆开

| 需要 | HTML 世界已有的 | TUI 为什么难 |
|---|---|---|
| **真组件** 同一套 render | React 就是 DOM | 渲染真值是 ANSI 行，不是 DOM；第二套 TS 格子会漂 |
| **眼睛** 看见同一帧 | 人看浏览器，agent 看 a11y 树 / 截图 | agent 现在看 insta 文本或真 PTY 截图，没有「点这个框」 |
| **框选覆盖层** 标组件 | DevTools outline / Storybook inspect | 覆盖层若画进格子会改用户所见；终端仿真器也不给应用 HTML overlay |
| **状态目录** 切 UI/UX | Storybook stories | 有 `/debug` scene（`src/app/debug_fixtures`），但是手测，不是 inspect 目录 |
| **规范化协作** | locator / ref / selected node | 没有共享的「框 id → 固定区词汇 / 槽 / FoldTarget」契约 |

wasm 是**运输**（把真组件送到「能叠 HTML overlay 的表面」），不是目标。目标是 **Inspect 表面 + 框目录 + 人/agent 同一份帧契约**。

术语：框 = overlay box（网页层，不进 transcript）；格子 = 产品 `Component::render` 的单元格。固定区词汇仍走 `docs/architecture/TUI信息呈现与固定区词汇.md`（覆盖弹出框 / 侧栏 / 槽 / 通知条…）。

### C.2 仓库里已经像「眼睛」的东西

这些是 **inspect 的半成品**，不是预览壳。

| 能力 | 落点 | 对「框选」够不够 |
|---|---|---|
| 无 TTY 出格子 | `VirtualTerminal`（仅 `crates/xylitol-tui/tests/support/`） | 能看见像素级单元格；**无名** |
| 矩形命中 | `Hit` / `HitRegion` / `HitRegionList`（`pointer.rs`，c2858 指针命中管线）：`on_line` / `at_cell` + 列宽 + `tall`；后注册覆盖先注册 | **几何已有**；`HitId = u64` 引擎不解释产品名 |
| hover 染色 | `apply_hover_tint`、`pointer_hover_id` | 产品交互，不是 inspect 标注；染色改格子 |
| 产品意义 | host 把 id 映到 `FoldTarget` / tab / mermaid | 覆盖不全：footer / 状态条 / 待办栏 / 页脚很多**没有** hit |
| 固定区预算 | `layout/fixed_zone_footprint.rs` 行数常量 | 有「这块占几行」，无「这一帧的像素框」 |
| 槽互斥脸 | `EditorSlot`（Editor / Tree / Models / …） | 状态枚举，可当 story 轴 |
| scene 表 | `DEBUG_SCENES`：Resume / LiveXy / LiveTape / `FixedZoneOp` | 最接近 Storybook；只服务 `/debug` 手测 |
| 视觉 SSOT | `DESIGN.md` Palette + insta | 回归用，不是讨论用 |
| 不要 TS 再绘格子 | `shell-frame.json` + `cell-grid.ts` 形态 | 有格子无框、无 agent 契约 |

结论：不必从零发明 hit-test。缺的是 **命名框目录（inspect tree）** 和 **不污染格子的 overlay 宿主**。

### C.3 HTML overlay 对 TUI 的正确类比

DevTools / Cursor 浏览器工具的分工：

| 层 | 人 | agent |
|---|---|---|
| 像素 | 看见页面 | 可选截图（贵、无名） |
| 结构 | inspect 高亮 + 标签 | a11y 树 / `ref`（Cursor `browser_snapshot`） |
| 动作 | 点框 | 对 ref 点/填 |

TUI 应对齐这套，而不是对齐「把 TUI 再实现成网页」：

```
Scene（/debug 或以后的 story）
  → 真 RootLayout / Component（产品代码）
  → TUI<VirtualTerminal>.render_frame()
  → InspectFrame {
        story, cols, rows,
        cells | ansi_lines,          // 真渲染
        boxes: [{ id, vocab, role, rect, text }],  // 眼睛
        selected?
     }
```

- **人**：HTML 画格子（只是 VT 的投影，不实现组件）+ **绝对定位 outline**（框选覆盖层）。点框 = `selected`。
- **agent**：读同一份 `InspectFrame` JSON（像 a11y YAML），用 `id` / 固定区词说话；需要观感时再加 PNG。
- **禁止**：把框画进 ANSI（会改布局、和 ApplicationOwned 选区 / hover tint 打架）。

这就是「TUI 上很难、所以想去浏览器」的技术理由：浏览器提供 **格子层 + overlay 层分离**；真终端只有一层单元格。

### C.4 wasm 在这条链里扮演什么（patch 不是拦路牌）

若宿主是静态网页、不想每次 `cargo test` 出帧，才需要 wasm。编的不是「完整 xylitol TUI app」，是 **Inspect 切片**：scene → render_frame → InspectFrame。系统相关 **按切片 patch**，不是把 Host/MCP/fd 搬进浏览器。

| 模块 | 进 Inspect wasm？ | patch |
|---|---|---|
| `Component::render` / layout / widgets | 要 | 原样 |
| `Terminal` 实现 | 要一个 | `VirtualTerminal` 升库 API；或写格子到 JS。**不要** crossterm 的 TTY backend |
| `crossterm::event` 类型 | 要（键鼠形状） | 可继续当纯数据；mio/TTY 用 `cfg(not(target_arch = "wasm32"))` 挡掉 `crossterm` 的 terminal 特性。上游 [crossterm#995](https://github.com/crossterm-rs/crossterm/issues/995) 仍开 → **自管 Terminal，不求 crossterm wasm** |
| `PointerPipeline` / `HitRegionList` | 要 | 原样；inspect 框从这里 + 固定区 footprint **命名后**导出 |
| `autocomplete_fd` / `std::process` | 不要 | `cfg` 掉 |
| tokio rt | 尽量不要 | inspect 同步 `render_frame` 即可 |
| syntect / two-face | 可选 | wasm 体积大；story 可关 highlight |
| 产品 Host / MCP / Trust / 图像协议 | **不要** | scene 注入 `UiEntry` / `EditorSlot` / `FixedZoneOp`（现成 `debug_fixtures`） |
| 产品 slash / keybindings | 后 | 键注入走 `dispatch_event`，先只看态 |

wasm32-unknown-unknown **可以**走通的前提是：Inspect 切片从第一天就不链 crossterm-tty / fd / Host。这是产品分层已经部分满足的（host 驱动 render），不是幻想。工作量仍大（独立 SDD），但是 **为 overlay 宿主服务**，不是为「能在浏览器里聊天」。

对照业界：Termless（xterm.js/Ghostty 测终端 UI、可 PNG）解决的是「仿真器一致性」，**不**给组件框；Storybook 给 stories + inspect，但是 DOM。要的是二者的缝：**真 ANSI 组件 + Storybook 式框。**

### C.5 三条宿主（同一契约，wasm 只是 C）

契约先于宿主。`InspectFrame` 稳定后，宿主可换。

| 宿主 | 人怎么看 | agent 眼睛 | 真组件？ | 何时 |
|---|---|---|---|---|
| **A 原生 dump** | 终端看格子；框在旁边 JSON / 另开 HTML 文件 | JSON | 是（VT） | **先做**：把 VT 升出 test-only；scene 渲一帧 dump |
| **B 薄 HTML overlay** | 格子 DOM + `position:absolute` 框（DevTools 样） | 同一 JSON；可选截图 | 是（帧来自 A，或 wasm） | A 能 dump 之后；**不要**再写组件 |
| **C Inspect wasm** | B 的页里 `wasm` 切 story / 键入 | 同一 JSON | 是（浏览器内 render） | 仅当「改代码立刻在页里切态」值得独立 SDD |

旧结论「优先 B 不要 wasm」在 **预览帧** 意义上仍对；在 **眼睛** 意义上：A/B 已够人-agent 讨论；C 是交互密度升级，不是正确性升级。

### C.6 框目录（规范化的核心）

没有命名框，overlay 只是好看的矩形。框必须能被人和 agent **用同一套词**指认。

建议 `InspectBox`：

| 字段 | 来源 | 例 |
|---|---|---|
| `id` | 稳定字符串（不要只暴露 u64） | `fixed.footer` / `slot.tree` / `fold.tool:t1` |
| `vocab` | 固定区词汇表 | 页脚 / 槽 / 覆盖弹出框 / 对话条目 |
| `role` | 机器枚举 | `footer` / `editor_slot` / `hit` / `transcript_entry` |
| `rect` | 单元格 `{col, row, cols, rows}` | 来自 `HitRegion` 或 footprint 本帧实测 |
| `text` | 框内纯文本预览 | agent 不靠视觉也能读 |
| `hit_id?` | 可选 u64 | 能回放到指针管线 |

覆盖规则（须写进以后的 spec，此处只记）：

1. **命中矩形**（已有 `HitRegionList`，后写入上）→ 有交互的控件。
2. **固定区实测框**（status / 待办栏 / 通知条 / 页脚 / 槽 / 通知栈 / 侧栏）→ 现在很多没 hit，inspect 仍要框，否则人和 agent 没法指「页脚」。
3. **对话条目**按 `UiEntry` 种类切块（tool / thinking / user…），可与 `FoldTarget` 对齐。
4. z 与产品 overlay 一致：覆盖弹出框 / 槽在对话条目之上。
5. 框 **不** 进入 `Component::render`。产品 hover tint 保持产品行为；inspect 高亮只在 HTML 层（或 dump 旁路）。

`debug_fixtures` 在 [§4](#CORE) 标「瘦」。inspect 立项时它是 story 源，改推荐列后再动。

### C.7 人-agent 协作手续（规范化，不是工具清单）

最小闭环（对标 Cursor 浏览器：snapshot → 点 ref → 再 snapshot）：

1. **开 story**：`scene_id` 来自 `DEBUG_SCENES`（或以后独立 catalog）。
2. **出帧**：得到 `InspectFrame`（格子 + 框）。
3. **指认**：人点 overlay **或** agent 报 `id` / `vocab`；双方只引用框 id，不说「上面那块绿的」。
4. **操作（可选）**：向真组件 `dispatch_event`（键/合成指针），再出帧。wasm 让这一步留在同一页；原生 A 则是「跑一次 harness」。
5. **记结论**：框 id + 期望（对比 DESIGN 词 / Palette），可落 insta 或 BDD；**不要**用 HTML DOM 当回归 SSOT。

agent 侧 skill 以后应读 JSON 框树，而不是先截图。截图只解决「色/间距好不好看」。

### C.8 明确不做

| 不做 | 原因 |
|---|---|
| 产品 TUI 全量进 wasm（Host/MCP/Trust/图像） | 眼睛不需要真模型循环；scene 注入即可 |
| 在 ANSI 里画 inspect 框 | 破坏用户所见，也破坏 ApplicationOwned 选区 |
| 用 bun+Vite 再绘一套格子 | 第二套 UI；不复活 `designing/` |
| fork crossterm-wasm | 预览用 VT，不用 crossterm TTY |
| 本轮建 `designing/`、装 wasm target | 预览不进本轮主线 |

### C.9 若立项，建议切片（仍不本轮）

| 步 | 产出 | skip-SDD？ |
|---|---|---|
| 0 | `InspectFrame` / `InspectBox` 形状 + 与固定区词汇对齐（spec） | **must-SDD**（新用户/agent 可见面） |
| 1 | VT 升 crate 可选 API；scene→dump JSON；一个静态 HTML overlay 读文件 | dump 可 lab_；HTML 壳若仅开发者不 must |
| 2 | 补固定区无名框（footer 等） | 行为若只服务 inspect 宿主，随 spec |
| 3 | agent 读框树的 skill / 约定 | 团队 how-to → xylitol-dev |
| 4 | 可选 Inspect wasm 切片 | 独立 change，不进 S0 |

关键产物是 `InspectFrame`（真渲染格子 + 命名框目录）。宿主可以是原生 dump、薄 HTML overlay、或以后的 wasm 切片。wasm 只编预览切片并把 TTY / `fd` / 真 Host patch 掉。现有 `HitRegion` + `debug_fixtures` + 固定区词汇覆盖半只眼睛；缺的是命名、overlay 层分离、以及 agent 与人共用的帧契约。预览不进本轮主线。

工作流原语见 [附录 D](#UI-WORKFLOW)。

---

<a id="UI-WORKFLOW"></a>

## 附录 D. 预览：工作流原语

接附录 C。web 是 GUI 式人-agent 协作面（框选、指认、切态），不是第二套 UI。Story + 代码态机 + XState/BDD 收成从设计到迭代的原语。产品 TUI 入口仍是 TTY attach。未落地。

### D.1 五层真值，不要压成一层

| 层 | 写什么 | 今日落点 | 管什么 |
|---|---|---|---|
| **WHAT** | 产品可观察行为 | `llmanspec/specs/app-tui-*.feature` 规则句 | 「Esc 须先关槽」 |
| **图** | 闭集转移 `from / event / guard / to` | 场景大纲 + llman-sdd XState **E1–E4**（表结构 / 确定性 / 可达 / 死锁） | 「有没有漏态、双 dest、死岛」 |
| **可执行抽样** | 假如/当/那么 + harness | `tests/bdd` + `TuiTestHarness` 按键→状态 | 这一跳在真组件上发生 |
| **Story** | 命名可达态（给眼睛看） | `DEBUG_SCENES` / `/debug` | 「Models 槽打开、busy、短终端」这一帧 |
| **眼睛** | 格子 + 命名框 | 附录 C `InspectFrame`；宿主宜 web | 人点框、agent 报 id |

XState 在本仓的角色是 **validate 时的图论门禁**，不是产品运行时。已用于 `RuntimePhase`（`agent-runtime` r2905）和 server 就绪态（`server-core` r1786）。**不要**在 TUI 里跑 xstate.js。运行时仍是 Rust 穷举枚举（`EditorSlot` 已是互斥脸）。

漂移记一笔：c2922 曾把 `specs.check_transitions: true` 写入 config；现行 `llmanspec/config.yaml` **没有**该键。图门禁是否还在跑，立项前先核对，不在本轮修。

### D.2 用代码描述 UI/UX/状态——对，但分清三种「代码」

| 种类 | 该不该当 SSOT | 例 |
|---|---|---|
| **面状态机**（槽、折叠、选区模式） | 要。枚举 + 转移表 | `EditorSlot`；忙时 Esc abort 且不得开树 |
| **像素 / 色 / 词** | 要。DESIGN + 固定区词汇 + insta | 不是状态机能表达的 |
| **Host 循环**（模型/工具/MCP） | 不要塞进 UI 机 | story 用 fixture 注入投影，不在浏览器跑 ReAct |

今日缺口：`EditorSlot` 在代码里是互斥枚举，spec 里是散落 `场景:`（`app-tui-input` 大量槽行为），**没有** `from/event/to` 大纲，所以 E1–E4 护不住「槽图」。`/debug` scene 也不声明自己对应转移表的哪一格。BDD 是抽样，不是闭集。

补齐原语（设计 → 维护 → 迭代）：

```
转移表（WHAT 的图）
  → story = 表上一个可达节点的注入
  → 真 Component 渲一帧 InspectFrame
  → 人/agent 在 web overlay 上指认框、改表或改期望
  → 场景大纲 + harness 执行该边
  → insta 钉这一帧的格子
```

web 在这条链上的位置：**协作宿主**（GUI 交互模式：点、框、切 story），不是产品语义源。语义源仍是 feature 表 + Rust 枚举。以后可以把同一张 `from/event/to` 丢给 xstate 可视化，仍然禁止用 JS 机当运行时。

这套对 **gpui / 以后任意端** 同样成立：图与 story 跨端同源；每端只换「格子怎么画」。端是 client（`app-tui` r1162）。设计态 wasm 绑的是 **Host 端口的 mock/磁带**，不是用户机器上的真 bash。

### D.3 rutis「插件化」到底卸掉什么

对照 `xylitol-x-rutis-temp`（S1 靶心，本轮不引入 rutis）：

| 已钉 | 含义 |
|---|---|
| 产品 TUI = 独立客户端包 | `xylitol-app-tui` 自己合帧，不进 Host `plugins/` |
| **TUI 合帧内部是红线** | 不把每帧拆成运行时插件 |
| 产品信封 ≠ rutis-bridge | 新载体仍走四象限 RPC / `PROTOCOL_VERSION` |
| 热数据不进跨进程插件 | 每帧/每 token 不经 rutis 叶子 |

「插件」是大概念（可替换实现，按平台换绑）。rutis 只是其中一种带生命周期开销的运行时库。现行代码词是端口 + 组合根。Host 侧官方能力用 `xylitol-core-plugin-*`。客户端用 `xylitol-app-*`。

### D.4 叫「预览」

代码里已经有这个词：`PreviewInject`、产品 Preview、`/debug` 场景。前端也用同一个词：打开 Storybook 就是打开预览。

三件分开说：

| 层 | 叫法 | 常见对应 | 已有落点 |
|---|---|---|---|
| 看哪一帧 | **场景** | Storybook 的一条 story | `/debug` · `DEBUG_SCENES` · `PreviewInject` |
| 系统怎么应 | **假 Host** | 假数据、假接口 | `MockBash`、假 `XyModel`、LiveTape |
| 人在哪看、怎么点 | **预览页** | 浏览器里的预览画布，可框选 | 还缺；宜用 web |

整包叫 **预览**：同一套 TUI client，挂假 Host，画面开在预览页上。

产品：client 挂真 Host，画面在 TTY。
预览：client 挂假 Host，画面在预览页。

wasm 只是预览页的一种载体，方便叠 HTML 框选。wasm 不是形态名，也不是第二种产品。

不要叫：profile、检视工坊、替身 Host、playground、sandbox、模拟器。harness 是按键到状态的测试机。预览会用到 harness，预览本身不是 harness。

### D.5 产品真 Host 不进浏览器；预览用假 Host

浏览器页 **没有** 用户那台机器的仓库、shell、MCP stdio 子进程、项目 `.xylitol/`。WASI/memfs 是页内沙箱，不是工作站。编码 agent 的产品承诺是改**真实仓库**；把真 `BashTool`/`MCP` 编进 wasm 仍摸不到那份盘，只是多一个假电脑。Trust 的产品语义是「是否加载**本机项目**资源」——页里没有「本机项目」，除非另有原生 Host 把仓库映射进来。MCP 同理：要 spawn 配置里的 server。

这不是「接口不能 mock」，是 **保真度**：真实现放进 wasm，得到的不是真环境。

你要的保真度是 UI、交互和状态机能在真组件上看到。系统副作用用磁带。这和测试里换假实现是同一条缝，仓库里已经有半截：

| 口 | 真 Host | 假 Host |
|---|---|---|
| `XyTool` / `BashOperations` | Host 里真 bash | `bash.rs` 已有 `MockBash` |
| `XyModel` | 真网关 | embed / Driver 测已有 TextMock |
| `XySessionStore` | JSONL | 测用内存 store |
| `XyPermission` / Trust | 本机项目门禁 | 可固定 trusted / 播「未信任」那一帧 |
| MCP | 有配置才装配 | 不装配，或回放「已连接」头卡 |
| 固定区 / 对话条目 | 真 `XyEvent` | `debug_fixtures` 场景已注入 |

组合根换的是 **挂哪一个 Host**，不是在组件里 `if wasm`：

| | 端（怎么画） | Host（系统怎么应） |
|---|---|---|
| 产品 TUI | TTY | 真 Host |
| **预览** | 预览页（可 wasm） | 假 Host：磁带 / 空操作 / 模拟 |
| （以后若要）薄 web 用户端 | 浏览器 client | 仍是**真** Host——另一条产品面 |

预览甚至可以不跑 ReAct：磁带直接喂 `XyEvent` / `UiEntry`（现成 LiveTape / LiveXy）。需要看「工具折叠随输出长高」时，回放一段 bash 结果比在 wasm 里真 exec 更稳、可重复。

预期形状：同一 client；换 Host 实现；换预览页载体。wasm 只服务预览页，不是把工作站搬进标签页。

### D.6 和 rutis 的关系（收窄）

rutis 不决定预览能不能做。预览依赖的是端口已存在、组合根能换绑、以及格子导出。rutis 以后若来，换绑可以变成卸一个 fiber、挂另一个。现在用 Rust 内置实现加开关就够。Core 算不算插件、插件层级怎么切，以后再谈。这里只记：大概念插件不等于必须是 rutis。

### D.7 明确不做

| 不做 | 原因 |
|---|---|
| 等 rutis 才开始 VT / 假 Host 绑定 | 端口和 scene 已在 |
| 把预览写成产品默认入口 | 预览用于设计和开发协作 |
| 预览 wasm 里真 exec 用户仓库 | 保真度错了；用磁带 |
| 用 xstate 解释器驱动产品 TUI | E1–E4 只看表 |
| 现在就钉「core 必须是 rutis 插件」 | 架构另开；先端口 |

这形态叫 **预览**。同一套 TUI client，挂 **假 Host**，开在 **预览页** 上，用 **场景** 切换状态。代码里已有 `PreviewInject` 和 `/debug`。缺的是把假 Host 收成与真 Host 对称的组合根，以及带框选的预览页。预览不进本轮主线。
