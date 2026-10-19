# xylitol 规划脊柱

本文件是规划真值，进 git 便于 diff。不升格为 `AGENTS.md` / spec。
进度只改本文件。稳定规则仍写进 `AGENTS.md`。行为合约仍写进 `llmanspec/`。

交流风格: ../STC-Simplified-Technical-Chinese

## 现在

下一刀：**W1.13**（架构组织描述 + BDD 按 STC 改写）。然后才是 W2.0（AGENTS crate 图）。

怎么读：先本节，再 [§2.0](#LEVELS) 组件级别，再 [§2.2](#LLM) 剪枝表，再 [§3.1](#PRE) 重写前，再 [§2.1](#TARGET) 总图。附录 A/B 是盘点与完成记录。designing 合约以已归档 c2930 为准，附录 C 只留指针。

顺序不得颠倒：工具链（WS0，已做）→ feature 真能关 → 契约叶 + 按剪枝表重写方言插件 → 新 crate 重写、旧 `src/` 双写。不为编译速度拆 crate。本轮不引入 rutis。

「不用就不付钱」分三层，不要混：编译（依赖写在会用它的 crate）→ 装配（没挂的插件不 start）→ 热路径（不在每 token / 每帧上）。单包 default-on 的 Cargo feature 不省默认二进制。

Host 侧 crate 边界 = 插件的替换 / 开关 / 生命周期。客户端是独立 `xylitol-app-*`，不是 Host 插件。
产品 TUI 默认 attach 本机监听（`xylitol-serve`）。MCP 有配置才装配，不得当死代码删。
禁止预埋 gpui / LSP / DAP / Sub-Agent 空壳。`incremental = false` 保持。default features 仍含 `cli,tui,otel,server,mcp`，只让关掉时真能关。

---

一事只在一节写全，他处用指针。



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



## 2. 目标形态（重写 + `src/` 双写）

新 crate 按契约重写，禁止 `pub use` 旧 `src/` 模块凑编译。旧 `src/` 继续出产品二进制，对照 [§4](#CORE) 后再删对应旧模块。不留兼容 shim。

三层分名：**角色**（执行权威）/ **服务提供方**（服务目录，`xylitol-runtime`）/ **载体**（直调或 WS，`xylitol-serve`）。crate 名只表达其中一层。不要 `xylitol-host`、`xylitol-client`、`xylitol-server-host`。

`XyDriver` 是 runtime 侧应用协议。客户端只通过 attach（信封或直调）消费。S1 rutis 只装配 runtime 进程内的 Host 侧插件。`xylitol-app-tui` 不是 rutis fiber。



### 2.0 组件级别（先定级别，再拆包）

重写态度：按契约重写实现，不把今日模块原样倒进目标 crate。crate 边界 = 替换 / 开关 / 生命周期，不是分层。

判据（内核）：它是不是「别的插件都要依赖、且不能自己被门控掉」的东西？是 → 内核。否 → 插件或客户端。

| 级别 | 是什么 | 例 | 本轮 |
|---|---|---|---|
| 内核 | 词表 + ports + errors + 信封 + trust + 装配 + 会话身份。必须不知道 ReAct / 压缩 / 具体厂商 / TUI | `xylitol-core`、`xylitol-protocol`、runtime 的信任与组合 | S0 建 |
| 内置插件 | 官方静态 `xylitol-core-plugin-*`。默认挂，可替换。进程内 `apply` 取一次 Arc 之后直连 | react、session、compaction、provider-*、tools | S0 建 crate 并重写 |
| 配置启用插件 | 仍是内置插件 crate；没配置不 start（装配层 zero-cost） | mcp、hooks | S0 建；未配置不连 |
| S1 装配 | 同一批插件，换 rutis 装配器 | `xylitol-rutis` | 本轮不引入 |
| 客户端 | 独立进程或同进程直调载体；不是 Host 插件 | `xylitol-app-tui` / `xylitol-app-print` / 引擎 `xylitol-tui` | S0 重写 app；引擎不改名 |

产品「内置工具」= 内置插件 `plugin-tools`，不是内核。禁止为 rutis 预埋空壳；禁止客户端进 `xylitol-core-plugin-*`。

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
方言插件只依赖 `xylitol-core` 里 [§2.2](#LLM) 标成「词表薄留」的类型，不把今日 `xylitol-ai-bridge` 整包倒进去。
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
Harness：引擎按键→状态在 `xylitol-tui`；产品场景 / 假 Host designing 在 `xylitol-app-tui`。

S0 先做：契约叶、按 [§2.2](#LLM) 剪枝后重写两个方言插件、工具链。不要把 `src/app` 原样搬成 `xylitol-host` / `xylitol-client`。不要把 `xylitol-ai-bridge` 非 HTTP 整包倒进 `xylitol-core`。





### 2.2 `xylitol-ai-bridge` 剪枝（重写去留）

现在的包把词表、HTTP、观测、lab 路径、第三层 `AiBridgeLlmAdapter` 捆在一起。重写时按 [§2.0](#LEVELS) 定级别，不搬家。不要抽 `xylitol-llm` / `xylitol-llm-types`。不要把 DTO 搬回今日主仓（§7.4）。不要把非 HTTP 整包倒进 `xylitol-core`。

去留四字：

| 标记 | 意思 |
|---|---|
| 现在剪 | WS1 就能从库表面拿掉，产品行为不变 |
| 词表薄留 | 重写进 `xylitol-core`：纯类型 / 端口形状，零 HTTP |
| 插件重写 | 进对应内置插件；按契约重写，不 `pub use` 旧模块 |
| 重写不迁 | 今日活着，新 crate **不要**这块形状（第三层、TLS、lab 配置） |

```text
xylitol-core          只收表内「词表薄留」
        ▲                    ▲
plugin-provider-openai   plugin-provider-anthropic
```

| 模块 | 约行 | 今日 | 级别 | 去留 | 现在能剪 |
|---|---|---|---|---|---|
| `dto/message.rs` + `dto/chunk.rs` | 544 | `AgentMessage::Llm` / `XyChunk` 别名 | 内核词表 | 词表薄留 | 否 |
| `dto/json_parse.rs` | 50 | 流式工具参数补全 JSON；`partial-json-fixer` | 内置插件 | 插件重写（两方言可各留副本或私有 util） | 否 |
| `usage/mod.rs` 类型 | ⊂359 | `AiBridgeUsage` 已是产品 usage | 内核词表 | 词表薄留 | 否 |
| `usage/mod.rs` `from_openai` / `from_anthropic` | ⊂359 | 厂商 JSON → usage | 内置插件 | 插件重写 | 否 |
| `accounting/mod.rs` | 189 | compaction Heuristic；禁止 tiktoken / RemoteCount | 内核可留常量+函数，或 compaction 插件内 10 行 | 词表薄留（`HEURISTIC_BYTES_PER_TOKEN` + `heuristic_token_count`）；`estimate_context` 跟压缩插件 | 否 |
| `thinking.rs` 选项形状 | ⊂480 | `XyGenerateOptions` 别名 | 内核词表 | 词表薄留（level / budgets / system / max_tokens / obs 快照字段） | 否 |
| `thinking.rs` `apply_thinking_*` + DeepSeek 解析 | ⊂480 | 写进厂商 body | 内置插件 | 插件重写 | 否 |
| `thinking.rs` `ObsSessionContext` 结构 | ⊂480 | 书签身份快照 | 内核词表 | 词表薄留（纯结构） | 否 |
| `provider/obs_session.rs` TLS / 进程槽 | 372 | idle 回退；generate 已禁止读槽 | 重写不迁 | 重写不迁。新路径只带快照，不设全局槽 | 否（行为耦合多） |
| `wire_policy/` | 206 | `compat` × extra 开关；请求装配 | 内置插件 | `Compat` 名字可进核心配置词表；`WirePolicy` 整包跟 openai 插件（anthropic 只用 DeepSeek overlay） | 否 |
| `hooks.rs` `HeaderBag` | ⊂46 | `serde_json::Map` 别名 | 内核词表 | 词表薄留或直接用 `Map`；不要独立 crate | 否 |
| `hooks.rs` `HttpHooks` + run_* | ⊂46 | HTTP 中间件口 | 配置启用插件 / 方言私有 | 插件重写（hooks 插件提供；方言消费 port） | 否 |
| `error.rs` `AiBridgeError` | 32 | 再 map 成 `XyError` | 重写不迁 | 方言直接 `XyError`。杀掉 `src/infra/provider/map.rs` | 否 |
| `provider/mod.rs` `AiBridgeLlmAdapter` | ⊂20 | 平行于 `XyModel` 的第三层 | 重写不迁 | 方言 `impl XyModel`。杀掉 `AdapterXyModel` | 否 |
| `fake/mod.rs` | 571 | 测 / designing 假模型 | 测支持 | 插件重写：直接 `impl XyModel`。不进内核 | 否 |
| `provider/tool_wire.rs` | 115 | 工具名过网关 | 内核小函数或 tools 插件 | 词表薄留（两插件不能互依） | 否 |
| `provider/native/openai*.rs` + assembler + hooks mw | ~3510 | Responses / Completions / `async-openai` | 内置插件 | 插件重写 `plugin-provider-openai` | 否 |
| `provider/native/anthropic_messages.rs` | 1194 | reqwest Messages | 内置插件 | 插件重写 `plugin-provider-anthropic` | 否 |
| `provider/dialect/deepseek.rs` | 71 | `compat=deepseek` overlay | 内置插件 | 跟 openai 插件（anthropic overlay 同文件或同插件私有） | 否。不要单独 crate |
| `provider/reqwest_bridge.rs` | 59 | HeaderBag ↔ reqwest | 内置插件 | 跟 anthropic 插件私有 | 否 |
| `provider/factory.rs` | 56 | 按 YAML 建 adapter | runtime 装配 | 插件重写：组合根调各插件工厂 | 否 |
| `provider/trace.rs` | 775 | `provider-trace.jsonl` / fastrace | 内置插件 / 观测 | 插件重写。不进内核 | 否 |
| `provider/attribution.rs` | 197 | OpenCode 网关 courtesy | 内置插件 | 跟 openai 插件 | 否 |
| `config.rs` | 115 | 曾复制主仓配置路径 | lab | 已剪（W1.15）：helper 在 `lab_responses_prompt_cache.rs` | 已做 |
| `src/infra/provider/map.rs` + `adapter/xy_model.rs` | 主仓 | 第三层壳 | 重写不迁 | 随 `AiBridgeLlmAdapter` 一起消失 | 否 |

行数 `wc -l`，包内合计约 9.2k。准入仍是 P1 两方言、P6 Anthropic 构建不得编 `async-openai`。

现在剪（W1.15，skip-SDD）：已把 `config.rs` helper 写进 `lab_responses_prompt_cache.rs`，库不再 `pub mod config`。不要顺手删 TLS / Fake / Adapter。

不要：按文件再拆 assembler；DeepSeek 单独 crate；等 S1 再拆 HTTP；把 Fake / WirePolicy / trace / TLS 放进内核；新建第四个「LLM 叶」crate；`AiBridgeLlmAdapter` 进 core。

批次：W1.15 现在剪 config；W2.2b 按本表重写两个方言插件并删 `crates/xylitol-ai-bridge`（在 W2.1 / W2.2 之后、W2.3 之前）。`package-ai-bridge*.feature` 主语过期则 validate-only 改句子。

---




## 3. 批次

编译痛点在工具链，不在拆 crate。WS0 已完成，记录见 [附录 B](#附)。

### 3.1 大重写前（WS1 收口）

crate 拆分前只改描述和已错事实。不改标识符，不开新产品，不把目标 crate 图提前写进现行规则。

| 批 | 做什么 | 准入 | SDD | 完成 |
|---|---|---|---|---|
| W1.13 | 按 STC 改写架构组织描述与 `layer-architecture` BDD；顺手把已错规则句改成现状 | 重写时少跟错句子、少跟超长规则段 | skip-SDD（AGENTS / 产品文）+ validate-only（`.feature` + 步骤绑定） | 否 |
| W1.14 | 死码分诊（`l8ng-audit-dead-code`），缩小搬家面 | 不挡 W2.0 | skip-SDD | 是 |
| W1.15 | `xylitol-ai-bridge`：`config.rs` 搬进 lab，库不再导出配置路径。剪枝全表见 [§2.2](#LLM) | 少把 lab 重复件带进重写 | skip-SDD | 是 |

W1.13 写什么：

1. `src/AGENTS.md`、根 `AGENTS.md` 分层摘要：STC **for instruction writing**（强度只用必须 / 不得 / 宜 / 不宜 / 可 / 不必 / 能 / 不能）。现行事实仍是单 crate 逻辑分层。
2. 组织级产品文：`docs/architecture/产品分层总览.md`、`库与多客户端.md`、`扩展与开闭.md`，以及 `docs/architecture/AGENTS.md`。STC **for document**。主语写名称，不用第一人称复数。
3. `llmanspec/specs/layer-architecture/layer-architecture.feature` 规则句与绑定步骤：情态词用 `llmanspec/AGENTS.md` 的必须 / 不得 / 宜。purpose 仍写单 crate。多 crate 句子留给 W2.0b。
4. 已错事实一并改：r1530 default 须含 `mcp`（`Cargo.toml` 已是 `cli,tui,otel,server,mcp`）；r1529 删不存在的 ACP / `infra-acp`，可选 flag 写成现用名，不虚构「域前缀」。
5. 改 Gherkin 步骤文本时，同步改 `tests/bdd` 精确绑定。文档探针（`steps_layer_probe.rs`）只改中文，本批不删扫描闸。

W1.13 不写什么：其余 `docs/architecture/` 能力细则（会话树、固定区等）；全量 `.feature` compact；[§2](#TARGET) crate 图（W2.0）；`layer-architecture` 改成多 crate（W2.0b）。

规则：`/home/l8ng/Projects/__straydragon__/STC-Simplified-Technical-Chinese`。AGENTS 走 instruction；产品文走 document；spec / BDD 走 llman 情态词表。检查：`node …/tools/cli.mjs check <文件> --profile <场景> --anti-echo`。

W1.8 / W1.9 / W1.10（`wire` 改名、双 `model`、host 三义）随 W2 搬 crate 时做，不单独全树 rename。

本轮默认不做：W1.1b（default 去掉 `server`）、W1.2b（默认二进制不含 MCP）、W1.3（`otel` 出 default）、W1.4（单包 `image` feature；解码器随 W2 工具集 crate 下沉）、W1.11（gherkin 改名）。delayed-changes / next-todo 仍 park，不升格。Inspect wasm、键位可视化、gpui 仍推迟。

已完成：W1.0 带走范围、W1.1 `server` 可关、W1.2 `mcp` 可关、W1.3b otel 路径可关、W1.5 hooks、W1.6 TUI 内 designing（`xylitol tui designing`，废止 `/debug`）、W1.7 session export、W1.12 删 CLI `resources` 与 `serve install`、W1.14 死码分诊、W1.15 `ai-bridge` lab 配置路径。说明见 [附录 B](#附)。



### 3.2 WS2 契约叶 + 重写双写

验收：边界就位 + `just qa` 全绿。编译变快只是副产品。回退靠 git revert 该批。

| 批 | 抽什么 | 准入 | SDD | 完成 |
|---|---|---|---|---|
| W2.0 | 改写 `src/AGENTS.md` 与根 `AGENTS.md`：crate 图见 [§2](#TARGET) | 否则代理按旧规则回滚 | skip-SDD | 否 |
| W2.0b | `layer-architecture.feature` 组织句：多 crate；分层纪律留在 crate 内 | spec 钉了组织 | validate-only | 否 |
| W2.1 | `xylitol-core`（词表 + ports + errors） | P5 | skip-SDD + validate-only | 否 |
| W2.2 | `xylitol-protocol`（信封 + fory 帧）；顺手去掉 `wire` 撞车名 | P5 | 同上 | 否 |
| W2.2b | 按 [§2.2](#LLM) 重写两个 provider 插件；core 只收「词表薄留」；删 `xylitol-ai-bridge` | P1 / P6 | skip-SDD + validate-only | 否 |
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
| designing（场景槽 + 假 Host） | 产品 TUI designing 组合 | `xylitol tui designing`（仅 debug 构建） | 核（开发协作；[c2930](llmanspec/changes/archive/2026-10-18-c2930-add-tui-preview/)） |
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
| OpenAI 兼容 + Anthropic | 今日 `xylitol-ai-bridge` + `infra/provider`；目标薄词表 + 两个 provider 插件 | 主线 | 核 |
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
| designing 框选 / Inspect wasm | 另 change；本轮只做 TUI 内场景槽 | 推迟（c2930 范围外） |

---



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
8. 发给模型的形状（今日在 `xylitol-ai-bridge`；重写只把 [§2.2](#LLM)「词表薄留」写进 `xylitol-core`）。
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
2. 不把场景表当产品 slash；场景源归 designing 组合（已归档 c2930）。
3. 不删 `vendor/gherkin`。
4. 不删 salvo / tokio-tungstenite 的产品用途。
5. 不提供 CLI `xylitol resources`。
6. 不提供 `xylitol serve install`。

---



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
| `crates/xylitol-ai-bridge` | 拆完 HTTP 后不再是桥；`bridge` 与 TUI/DTO 多层撞车 | 删包；去留见 [§2.2](#LLM) |

不改：`xylitol-tui` 引擎包名、`XyDriver`、`XyEvent`、`XyModel`、`vendor/gherkin`。

---



## 7. 本轮明确不做

1. gpui / Web 控制台 / DAP / LSP / Sub-Agent 空壳 crate。
2. S1 引入 rutis，以及 S2–S5。
3. 按 layer 拆 crate。
4. 抽 `xylitol-domain`；把发给模型的形状从 bridge 搬进今日主仓 `xylitol`；把 `xylitol-ai-bridge` 非 HTTP 整包倒进 `xylitol-core`（薄留清单见 [§2.2](#LLM)）。
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



## 8. 抽出契约叶前必须改的 AGENTS / spec

改写只换稳定规则，不写进度，不留墓碑。先做 [W1.13](#PRE)（现行单 crate 事实按 STC 改写），再换 crate 图。两刀不要混在一次提交里。

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



## 9. 来源

| 主题 | 路径 |
|---|---|
| 团队怎么做 | `.claude/skills/xylitol-dev/SKILL.md` |
| STC 简明技术中文 | `/home/l8ng/Projects/__straydragon__/STC-Simplified-Technical-Chinese` |
| 代码架构 SSOT | `src/AGENTS.md` |
| 产品主线 | `docs/architecture/产品分层总览.md` |
| spec 组织句 | `llmanspec/specs/layer-architecture/layer-architecture.feature` |
| 编译调优 | `~/.config/llman/skills/rust-build-tune/` |
| rutis 重写方向 | `/home/l8ng/Projects/__straydragon__/rutis/xylitol-x-rutis-temp/skills/xylitol-rutis-rewrite/` |
| 拆分闸门 | 同上 `references/split-principles.md`（P1–P6） |
| 目标 crate 图 | 同上 `references/crate-graph.md` / `crate-naming.md` / `crate-split-detail.md` |
| 迁移阶段 | 同上 `references/migration-stages.md` |

---



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

#### W1.14

真死已删：`AgentCapabilities::build_current_model`（调用方走 `ModelManager`）、未接线的 `enable_modify_other_keys`（alt-screen rearm 已写同一 CSI）、无调用的 `PtySession::spawn_demo_with_prompt`、`product_slash_commands` 的空 `mut`。tmux `send` / `send_text` 有调用方，去掉假 `allow`。designing 注入路径注释不再写已退役 `/debug`。

未删：BDD `@executable` 合约（session/settings/hooks/factory 等 `allow(dead_code)`）、`server`/`mcp`/`otel` feature 门、tui tests/support 分 target harness、`lab_*`、`agent_demo*`。搬家面没有因此明显变小；大量 `allow` 是 spec 缝，不是 crate 拆分对象。

#### W1.15

删 `crates/xylitol-ai-bridge/src/config.rs`。live-provider 路径解析只留在 `lab_responses_prompt_cache.rs`。库不再 `pub mod config`。不改 TLS / Fake / `AiBridgeLlmAdapter`。

#### CLEAN

`a77a96c5`：删 `designing/`、一次性 lab、crate `PI_DELTAS`；cccc 内联；Palette 门。

#### CONFIGS

`e11f1e76`：`configs/secret.env.example`。

#### DOC

`c6dfaec8`。FR [#11](https://github.com/StrayDragon/xylitol/issues/11)–[#27](https://github.com/StrayDragon/xylitol/issues/27)；roadmap [#28](https://github.com/StrayDragon/xylitol/issues/28)。research 留 `maka-runtime-kernel-lessons-2026.md`。政策已写入 `docs/AGENTS.md`。

---



## 附录 C. designing

TUI 内 designing（场景槽 + 假 Host，废止 `/debug`）：[c2930-add-tui-preview](llmanspec/changes/archive/2026-10-18-c2930-add-tui-preview/)。入口是 `xylitol tui designing`（仅 debug 构建），不是顶层 `preview` / `designing`。designing preview = 把某场景注入画布的预览设计。

框选覆盖层 / `InspectFrame` / 薄 HTML overlay / Inspect wasm **不在** c2930。另开 change。不建 `designing/`、本轮不装 wasm target。



## 附录 D. designing 工作流

WHAT / 转移表 / BDD / 场景 / 眼睛 五层不要压成一层。c2930 只落地 **场景 + 假 Host + TUI 画布**。眼睛（命名框）后置。细则写在已归档 c2930 的 `design.md`，此处不复述。
