# xylitol 规划脊柱

本文件是规划真值，进 git 便于 diff。不升格为 `AGENTS.md` / spec。
进度只改本文件。稳定规则仍写进 `AGENTS.md`。行为合约仍写进 `llmanspec/`。

交流风格: ../STC-Simplified-Technical-Chinese

## 现在

下一刀：**W2.0**（改 AGENTS crate 图）。

怎么读：先本节，再 §2 总图，再 §3 未完成表。附录 A/B 是盘点与完成记录。预览合约以已归档 c2930 为准，附录 C 只留指针。

顺序不得颠倒：工具链（WS0，已做）→ feature 真能关 → 契约叶 + 拆 ai-bridge HTTP → 新 crate 重写、旧 `src/` 双写。不为编译速度拆 crate。本轮不引入 rutis。

「不用就不付钱」分三层，不要混：编译（依赖写在会用它的 crate）→ 装配（没挂的插件不 start）→ 热路径（不在每 token / 每帧上）。单包 default-on 的 Cargo feature 不省默认二进制。

Host 侧 crate 边界 = 插件的替换 / 开关 / 生命周期。客户端是独立 `xylitol-app-*`，不是 Host 插件。
产品 TUI 默认 attach 本机监听（`xylitol-serve`）。MCP 有配置才装配，不得当死代码删。
禁止预埋 gpui / LSP / DAP / Sub-Agent 空壳。`incremental = false` 保持。default features 仍含 `cli,tui,otel,server,mcp`，只让关掉时真能关。

---

一事只在一节写全，他处用指针。

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

WS1 表已空。W1.8 / W1.9 / W1.10（`wire` 改名、双 `model`、host 三义）随 W2 搬 crate 时做，不单独全树 rename。

本轮默认不做：W1.1b（default 去掉 `server`）、W1.2b（默认二进制不含 MCP）、W1.3（`otel` 出 default）、W1.4（单包 `image` feature；解码器随 W2 工具集 crate 下沉）、W1.11（gherkin 改名）。

已完成：W1.0 带走范围、W1.1 `server` 可关、W1.2 `mcp` 可关、W1.3b otel 路径可关、W1.5 hooks、W1.6 TUI 内预览（`xylitol tui designing`，废止 `/debug`）、W1.7 session export、W1.12 删 CLI `resources` 与 `serve install`。说明见 [附录 B](#附)。

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
| 图像粘贴 / 读图 | `infra/image` + bang | 主线；依赖重 | 核（解码器随 W2 工具集 crate；不做单包 feature） |
| 预览（场景槽 + 假 Host） | 产品 TUI 预览表面 | `xylitol tui designing`（仅 debug 构建） | 核（开发协作；[c2930](llmanspec/changes/archive/2026-10-18-c2930-add-tui-preview/)） |
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
| 可选 OTLP（`[otel]` 默认 none） | feature `otel` | default 仍开；lane/采样未做 | 核（不扩；W1.3b 路径已可关） |
| samply `profile-suite` | `scripts/profile_tui_suite.py` | 维护 | 推迟（不进 crate 图） |
| 自研检视台 / Jaeger 必选 | 无 | 推迟 | 推迟 |

### 4.4 暂不带走

| 项 | 说明 | 推荐 |
|---|---|---|
| gpui / LSP / DAP / Sub-Agent / Loop / Computer-Use | 未实现；禁止空壳 crate | 推迟 |
| delayed-changes 草案 | 不进 S0 文件搬家 | 推迟 |
| `agent_demo*` / `host_loop` | 引擎演示，非产品 SSOT | 留在 `xylitol-tui` |
| maka research | S1 对照 | 留 `docs/research/` |
| 23 个集成测试二进制 | 不删测试 | 瘦（share-generics 已做） |
| 预览框选 / Inspect wasm | 另 change；本轮只做 TUI 内场景槽 | 推迟（c2930 范围外） |

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
3. `image` crate：不在单包加 default-on feature。W2 随 `plugin-tools` 下沉依赖。
4. `jsonschema` / `minijinja` / `schemars`：S0 下沉到使用方 crate，不预删。
5. `cli = []`：极瘦进程入口和 `xylitol-app-print` 给真实依赖，不单独发明空开关。
6. 23 个集成测试二进制：不删测试；靠 `-Zshare-generics` 降重复。

**DELETE**

1. 不新增 gpui / Web 控制台 / DAP / LSP / Sub-Agent 空壳。
2. 不把场景表当产品 slash；场景源归预览表面（已归档 c2930）。
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
13. 单包 `image` Cargo feature（W1.4）。

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
| `otel` | optional OTLP 三件套；W1.3b 后可关（default 仍开；配置默认 none） |
| `image` / `fory` / `jsonschema` / `minijinja` / `schemars` / `fastrace` | 非 optional，始终进主 crate（`image` 不做单包 feature） |
| `salvo` / `tokio-tungstenite` / `xylitol-tui` | 随 feature 可选 |

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

`just lint-all` 增加 default 减 `otel`（`cli,tui,server,mcp`）clippy。关 feature 时 resolve/diag 标 `dead_code`；`install` 两边都 `pub`，无 feature 返回 `None`。default 仍含 `otel`。

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

## 附录 C. 预览

TUI 内预览（场景槽 + 假 Host，废止 `/debug`）：[c2930-add-tui-preview](llmanspec/changes/archive/2026-10-18-c2930-add-tui-preview/)。入口是 `xylitol tui designing`（仅 debug 构建），不是顶层 `preview` / `designing`。

框选覆盖层 / `InspectFrame` / 薄 HTML overlay / Inspect wasm **不在** c2930。另开 change。不建 `designing/`、本轮不装 wasm target。

<a id="UI-WORKFLOW"></a>

## 附录 D. 预览工作流

WHAT / 转移表 / BDD / 场景 / 眼睛 五层不要压成一层。c2930 只落地 **场景 + 假 Host + TUI 画布**。眼睛（命名框）后置。细则写在已归档 c2930 的 `design.md`，此处不复述。
