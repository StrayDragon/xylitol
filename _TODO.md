# xylitol 大调整规划脊柱

本文件是规划真值，进 git 便于 diff。不升格为 `AGENTS.md` / spec。
进度只改本文件。稳定规则仍写进 `AGENTS.md`。行为合约仍写进 `llmanspec/`。

交流风格: ../STC-Simplified-Technical-Chinese

盘点日期：2026-10-17。
依据：仓库实读 + `xylitol-dev` + `rust-build-tune` + `xylitol-x-rutis-temp/skills`。

进度：完成列见 §4（附加说明链到 [附](#附) 同名 ID）。下一动作：§13 核心三面推荐列 → W0.1 基线 → 按列清理 → 再 W1.3b / W1.4 / W2.0。

---

## 0. 结论

本轮做四件事，顺序不得颠倒：

1. 工具链加速编译（profile / mold / sccache / `-Zshare-generics`）。
2. 让已声明的 feature 真能关，并把非标准名字改到目标词表。
3. 按插件边界抽出多 crate（S0，行为不变），给后续 rutis 聚合留落点。
4. 记下 S1 起的 rutis 重写靶心。本轮不引入 rutis。

不为编译速度拆 crate。编译慢先调工具链。
crate 边界 = 插件的替换 / 开关 / 生命周期边界，不是 `protocol` / `agent` / `infra` / `app` 各一包。

产品 TUI 默认 attach 本机 Host。HTTP Host **不得删**。
MCP 是配置后置产品，**不得当死代码删**。
未实现的 gpui / LSP / DAP / Sub-Agent **不得预埋空壳**。

---

## 1. 固定约束

1. 行为不变的组织改动可跳过 llman-sdd 生命周期。live `.feature` 主语 / 路径过期时，直接改 feature，再跑 `llman-sdd validate`。
2. 产品可观察行为变化必须走完整 SDD：explore → propose → apply → verify → archive。
3. 任何批次合并前必须 `just qa`。触及真终端时再加 `just qa-e2e`。
4. 试验脚本只许 `lab_` 前缀。不得再引入 `evidence_` / `experiment_`。
5. 提交用 Conventional Commits。未要求则不自动 commit。
6. `AGENTS.md` 只收硬约束 / 稳定原则。不得把本文件进度抄进去。移除即改写，不留墓碑。
7. 同 crate 的不同 worktree **禁止**共用 `CARGO_TARGET_DIR`。
8. 改名一次性改调用点。不留兼容别名，不留 `#[deprecated]`。

### 1.1 SDD 路由（三条）

| 类别 | 何时 | 做什么 |
|---|---|---|
| skip-SDD | 纯组织 / 工具链 / 编译隔离，且产品行为不变 | 直接改代码 + `just qa` |
| validate-only | 组织演进导致 live `.feature` 过期 | 直接改 `.feature` + `llman-sdd validate` + `just qa` |
| must-SDD | 改了用户能看见或默认二进制含什么 | 完整 change 生命周期 |

发现「以为组织改动、实际改了行为」时立刻停，转 propose。

---

## 2. 现状盘点（事实）

### 2.1 体量

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

Workspace 成员：根包 `xylitol` + `crates/xylitol-tui` + `crates/xylitol-ai-bridge`。
`vendor/gherkin` 是 `[patch.crates-io]`，不是 member。
`cargo metadata` resolve 节点约 594 个。

### 2.2 Feature 与依赖污染

根 `Cargo.toml` default = `cli, tui, otel, server, mcp`。

| 项 | 事实 |
|---|---|
| `cli = []` | 空 feature，不裁依赖 |
| `server` | optional；W1.1 后可关（default 仍开） |
| `mcp` | optional `rmcp`；W1.2 后可关（default 仍开；运行时仍需配置才装配） |
| `image` / `fory` / `jsonschema` / `minijinja` / `schemars` / `fastrace` | 非 optional，始终进主 crate |
| `salvo` / `tokio-tungstenite` / `xylitol-tui` / OTel 三件套 | 随 feature 可选 |

### 2.3 编译配置

已落地（磁盘优先，不是速度优先）：

- dev：`opt-level = 0`，依赖 `opt-level = 3`，`debug = "line-tables-only"`，`incremental = false`，`codegen-units = 16`
- release：`opt-level = "z"`，`lto = "thin"`，`strip = "symbols"`，`panic = "abort"`，`codegen-units = 1`

已落地（续，W0）：`.cargo/config.toml` 里 Linux mold + `-Zshare-generics`（同一份 rustflags）；CI `apt install clang mold`。Cranelift 仍全局不用。
主 worktree 用仓库内 `target/`；`scripts/cargo_worktree_env.sh` 只在额外 worktree / 显式 eval 时隔离（有 sccache 则导出 `RUSTC_WRAPPER`）。CI 缓存仍是 `Swatinem/rust-cache`，不强制 sccache。

本机 `target/` 约 45G，其中 `debug/build` 约 44G（约 5087 个 fingerprint 目录，属历史堆积）。
集成测试二进制 23 个（`xylitol-tui` 占 19 个）。BDD 收成 1 个 `tests/bdd.rs`。
Cranelift 受 `aws-lc-rs` / `ring` 阻断，全局不用。

说明：用户痛点是编译慢。现行 profile 用关增量换磁盘。这是真实冲突，见第 8 节决策 1。

### 2.4 产品主线 vs 树内后置

来源：`docs/architecture/产品分层总览.md`。

主线：对话循环、内置工具、会话持久化 / 导出、压缩、两家模型 API、Trust、图像附件。
默认入口：产品 TUI attach 本机 Host（未在听失败）。Print / 嵌入可同进程。
后置：MCP（有配置才装配）、更多厂商。

spec `layer-architecture.feature` `@req:r1532` 要求提供 server 应用面（`POST /rpc` + `WS /rpc`）。删 Host 会改行为合约。

未实现：gpui、Web 控制台、DAP、LSP、Sub-Agent（GitHub FR，索引 [#28](https://github.com/StrayDragon/xylitol/issues/28)；草案在 `delayed-changes/`）。

### 2.5 现行规则冲突点

`src/AGENTS.md` / 根 `AGENTS.md`：单 crate 逻辑分层，**不为分层拆 crate**；禁止再抽 `xylitol-domain`。
`layer-architecture.feature` purpose 注释写了「单 crate」。
rutis 重写 skill：crate 边界镜像插件边界；S0 必先行。

S0 第一批抽取之前必须改 AGENTS。spec 上的「单 crate」走 validate-only 直接编辑。

---

## 3. 最终目标形态

目标 workspace 约 16–18 个 crate（甜区 15–25）。
来源：`xylitol-rutis-rewrite/references/crate-graph.md` + `crate-naming.md`。

```text
crates/
  xylitol-core/                 词表 + ports + errors
  xylitol-protocol/             四象限信封 + fory-v3 + PROTOCOL_VERSION
  xylitol-kernel/               组合根 + trust + config→装配（S0 仍手工；S1 换 rutis）
  xylitol-host/                 HostServer 载体 + 会话槽
  xylitol-client/               XyDriver + 载体（in-process / http-ws）
  xylitol-bin/                  CLI / Print
  xylitol-tui/                  已存在，引擎
  xylitol-ai-bridge/            LLM 叶：DTO + accounting + WirePolicy + FakeProvider（无 HTTP）
  xylitol-rutis/                S1 才建；唯一 use rutis
  plugins/
    xylitol-core-plugin-react/
    xylitol-core-plugin-session/
    xylitol-core-plugin-compaction/
    xylitol-core-plugin-provider-openai/      含 OpenAI HTTP（async-openai）
    xylitol-core-plugin-provider-anthropic/   含 Anthropic HTTP（reqwest）
    xylitol-core-plugin-tools/
    xylitol-core-plugin-mcp/     feature mcp；rmcp 只进这里
    xylitol-core-plugin-hooks/
    xylitol-core-plugin-tui/     复用 xylitol-tui
```

依赖方向：插件只依赖 `xylitol-core`（必要时只读 protocol DTO），插件不横向依赖。
provider 方言插件额外依赖 `xylitol-ai-bridge` 叶（DTO / accounting），不依赖另一个方言插件。
`kernel` → core + protocol。`host` / `client` → kernel。`bin` → host + client。
S1 之后只有 `xylitol-rutis` 直接 `use rutis`。

每个 crate **内部**仍守：`app → agent|infra → protocol`；`agent` ↛ `infra`；`wire` ↛ `ports`。
禁止按层各拆一 crate。禁止 `xylitol-domain`。禁止把 LLM 叶从 bridge 搬回主仓。

内核（S1 起）必须不知道 ReAct、压缩、provider、TUI 的存在。
S0 先把文件按插件边界切开；内核「不知情」在 S1 用依赖方向强制。

`vendor/gherkin` 保持上游包名 + path patch。不加 `xylitol-` 前缀。

---

## 4. 工作流

### WS0 工具链（最先）

编译痛点在工具链，不在拆 crate。顺序：profile → mold → sccache → 泛型去重。crate 图最后。

| ID | 项 | SDD | 回退 | 完成 |
|---|---|---|---|---|
| W0.1 | `lab_` 基线：`cargo --timings`、主路径墙钟、`du` 拆账 | skip-SDD | **临时**：本文件全部 W* 收口后删脚本或 freeze 归档 | [是](#W0.1) |
| W0.2 | 新增 `.cargo/config.toml`：Linux mold。rustflags 必须与 share-generics 写在同一份 | skip-SDD | 删 config | [是](#W0.2) |
| W0.3 | 主开发接通 sccache（`just cargo-wt-env` 已有脚本） | skip-SDD | 去掉 `RUSTC_WRAPPER` | [是](#W0.3) |
| W0.4 | `-Zshare-generics`（23 个集成测试二进制） | skip-SDD | 去掉 rustflags | [是](#W0.4) |
| W0.5 | `incremental` / `codegen-units`：等第 8 节决策 1 | skip-SDD | 复原现行磁盘 profile | [是](#W0.5) |
| W0.6 | 授权后 `cargo clean` 清 45G 死历史 | skip-SDD | 不可逆；下一轮冷编译变慢 | [是](#W0.6) |
| W0.7 | 补写或删除悬空 `docs/research/rust-disk-worktree-cache-2026.md` 指针 | skip-SDD | 复原注释 | [是](#W0.7) |
| W0.8 | 全局 Cranelift | **不做** | — | — |

### WS1 瘦身 + 改名（S0 之前）

先缩小表面、改到目标词表，再搬文件。少一次全树 rename。

| ID | 项 | SDD | 说明 | 完成 |
|---|---|---|---|---|
| W1.0 | 核心三面梳理：用户 / Host / 观测必须带走；其余暂不进 S0 | skip-SDD | 人改 [§13](#CORE) 推荐列后再动代码 | 否 |
| W1.1 | 切断 `wire_v3` / `ws` 对 `HostState` 的无条件 import，让 `server` feature 真能关 | skip-SDD | default **仍含** `server` | [是](#W1.1) |
| W1.1b | default 去掉 `server` | **must-SDD** | 改产品 TUI 启动。本轮默认不做 | — |
| W1.2 | `rmcp` 下沉到 `mcp` feature；default 仍编进产品二进制 | skip-SDD | 编译隔离 ≠ 删除 MCP | [是](#W1.2) |
| W1.2b | 默认二进制不再含 MCP | **must-SDD** | 本轮默认不做 | — |
| W1.3 | `otel` 从 default 拿掉 | **must-SDD** | 本轮默认不做 | — |
| W1.3b | 只修 otel 代码路径，default 不动 | skip-SDD | 可做 | [否](#W1.3b) |
| W1.4 | `image` crate feature-gate，default 仍开 | skip-SDD | 模块小、依赖重 | 否 |
| W1.5 | hooks 保持产品口；S0 再下沉到 plugin crate | skip-SDD | ~1.6k，不删 | 是 |
| W1.6 | 审计 `debug_fixtures`（~653 行）：测试夹具下沉 `tests/` | skip-SDD | 若砍用户可达调试面 → must-SDD | 否 |
| W1.7 | session export 保持主线 | 不改 | `app/core` 产品面 | 是 |
| W1.8 | `wire` 模块随 `xylitol-protocol` 抽取改名（见 WS2） | skip-SDD + validate-only | 不单独做一次全树 rename | 否 |
| W1.9 | 双 `model` 模块改名（protocol DTO vs agent 选择） | skip-SDD + validate-only | 不改 `XyModel` 语义 | 否 |
| W1.10 | 内部类型：host 三义 → role / HostServer / carrier | skip-SDD | 改 embed 公开类型则 must-SDD | 否 |
| W1.11 | `vendor/gherkin` 改 `xylitol-gherkin` | **不做** | 上游 patch | — |

### WS2 S0 crate 抽取（行为不变）

验收：边界就位 + `just qa` 全绿。编译变快只是副产品。S0 无架构 kill，回退靠 git revert 该批。

建议分批，不一次搬 11.8 万行：

| 批 | 抽什么 | 准入 | SDD | 完成 |
|---|---|---|---|---|
| W2.0 | 改写 `src/AGENTS.md` 与根 `AGENTS.md` 的「不拆 crate」 | 否则代理按旧规则回滚 | skip-SDD | 否 |
| W2.0b | `layer-architecture.feature` purpose / 组织句：单 crate → 多 crate 插件边界；分层纪律留在 crate 内 | spec 钉了组织 | validate-only | 否 |
| W2.1 | `xylitol-core`（词表 + ports + errors） | P5 发布契约 | skip-SDD + validate-only | 否 |
| W2.2 | `xylitol-protocol`（信封 + fory 帧）；顺手消掉 `wire` 撞车名 | P5 | 同上 | 否 |
| W2.2b | 拆 `xylitol-ai-bridge`：叶留下、HTTP 方言进 provider 插件。见 [§16](#AI-BRIDGE) | P1 / P6 | skip-SDD + validate-only | 否 |
| W2.3 | `xylitol-core-plugin-mcp` / `-hooks` / `-provider-*`（方言插件吃 W2.2b 的 HTTP） | P1 / P2 / P6 | skip-SDD | 否 |
| W2.4 | `xylitol-host` / `xylitol-client` / `xylitol-bin` | P2 / P4 / P5 | skip-SDD + validate-only | 否 |
| W2.5 | `react` / `session` / `compaction` / `tools` / `tui` | P1 / P2 | skip-SDD + validate-only | 否 |
| W2.6 | 每批后 `just qa`；TUI 批加 `just qa-e2e` | 门禁 | 强制 | 否 |

S0 **不**创建：`xylitol-rutis`、空 LSP/DAP crate、dylib 加载器。

### WS3 S1+ rutis（本文件只记靶心）

| 阶段 | 动作 | 本轮 | 完成 |
|---|---|---|---|
| S1 | 引 rutis 只做组装运行时；热路径延迟不劣化 | 不做。这是整次重写的 go/no-go | — |
| S2 | 会话 = instance；删 `RuntimePorts` 手工物化 | 不做 | — |
| S3 | hooks→bus、MCP boot→loader、freeze→generation | 不做 | — |
| S4 | 多语言叶 + dylib | 不做 | — |
| S5 | TUI 作客户端事件订阅者 | 不做 | — |

S1 失败则整次 rutis 重写可中止。S0 的内聚 crate 仍留下。

### 附

无附加则完成列只写 是 / 否 / —。有提交或落点才在此按 ID 开节（标题即锚点）。



#### W0.1

脚本：`python3 scripts/lab_compile_baseline.py`（`--cold` = `cargo clean -p` 三成员后重编）。本机 `CARGO_TARGET_DIR=~/.cache/cargo-targets/xylitol/3a096cc8d643`。

| 条件 | 墙钟 | du target | bin |
|---|---|---|---|
| 指纹无改 | 0.2s | 8.1G→（后 `--cold` 落到 4.6G） | 260.3M |
| `--cold` + sccache 全命中 | 1.7s | 4.6G | 260.3M |
| `--cold` 且 `env -u RUSTC_WRAPPER` | **16.0s** | 4.6G | 260.3M |

对照用最后一行（成员 crate 重编，依赖已在）。timings HTML：`.tmp/lab_compile_baseline/20261017T141652Z/`。全量 `cargo clean` 后再编依赖会到分钟级，不作为本基线。

**临时脚本**：`scripts/lab_compile_baseline.py` 不进 qa。本文件全部 W* 完成后删除，或把 summary 进 freeze `research/` 再删脚本。禁止升格 just recipe。



#### W0.2

`073eccfc`。`.cargo/config.toml` Linux mold；rustflags 与 W0.4 同份。CI `apt install clang mold`。



#### W0.3

`scripts/cargo_worktree_env.sh`：有 sccache 则导出 `RUSTC_WRAPPER=sccache`。



#### W0.4

同 W0.2（`073eccfc`，`-Zshare-generics`）。



#### W0.5

锁定 §8.1 (a)：`incremental = false` 不动。



#### W0.6

已 `cargo clean`；`debug/build` 另补删。同 §8.6。



#### W0.7

`Cargo.toml` 注释改指 `.cargo/config.toml`。



#### W1.1

同 W0.2（`073eccfc`）。



#### W1.2

`b3acd3f8`。default 仍含 `mcp`。



#### W1.3b

产品 leftover [#27](https://github.com/StrayDragon/xylitol/issues/27)；本项只修 compile 路径，default 仍开 `otel`。



#### CLEAN

`a77a96c5`：删 `designing/`、一次性 lab、crate `PI_DELTAS`；cccc 内联；Palette 门。



#### CONFIGS

`e11f1e76`：`configs/secret.env.example`。



#### DOC

`c6dfaec8`。FR [#11](https://github.com/StrayDragon/xylitol/issues/11)–[#27](https://github.com/StrayDragon/xylitol/issues/27)；roadmap [#28](https://github.com/StrayDragon/xylitol/issues/28)。research 留 `maka-runtime-kernel-lessons-2026.md`。政策已写入 `docs/AGENTS.md`。

---

## 5. 瘦身表：KEEP / SLIM / DELETE

编译隔离与产品删除必须分开。本轮默认走保守：default features 不动，只让关 feature 真能关。激进方案见第 8 节决策 2。

**KEEP（主线，S0 后仍默认装配）**

1. Print + 产品 TUI。
2. Host 操作器 + 本机监听载体（default 含 `server`）。
3. 跨端信封 + `XyDriver` / `XyEvent`。
4. ReAct、session store、compaction、OpenAI 兼容 / Anthropic、内置工具、trust。
5. MCP（运行时：有配置才装配）。
6. hooks（embed / `hooks_config` 已是产品口）。
7. session export。
8. `xylitol-ai-bridge`。
9. fastrace + `log`（禁止再引入 `tracing` 双栈）。

**SLIM（compile 隔离；default 建议仍开）**

1. HTTP Host 代码（~4.3k）+ 始终编译的 wire：修好 `server` feature。
2. `rmcp` → `mcp` feature / 未来 plugin crate。
3. `image` crate → feature，default 仍开。
4. `jsonschema` / `minijinja` / `schemars`：S0 下沉到使用方 crate，不预删。
5. `cli = []`：S0 抽 `xylitol-bin` 时给它真实依赖，不单独发明空开关。
6. 23 个集成测试二进制：不删测试；靠 `-Zshare-generics` 降重复。

**DELETE（极窄）**

1. 不新增 gpui / Web 控制台 / DAP / LSP / Sub-Agent 空壳。
2. `debug_fixtures`：确认全是测试夹具后再下沉或删生产模块。
3. 不删 `vendor/gherkin`。
4. 不删 salvo / tokio-tungstenite 的产品用途。

---

## 6. 今日非标准名字

只列现状不标准者。S0 搬 crate 时顺手改。

| 现状 | 问题 | 目标 |
|---|---|---|
| `src/protocol/wire/`、`app/server/wire_v3`、`host_client/wire_v3_client` | `wire`（线协议）与 `wiring`（组合根接线）撞车 | crate `xylitol-protocol`；中文「跨端协议」 |
| `HostState` / 角色 host / `HostClient` | host 一词三义 | 角色保持「执行权威」；提供方称 HostServer；进程内 / WS 称 carrier。`XyDriver`、`HostClient` 保留 |
| `protocol/model*` 与 agent 侧 `model` | 双 `model` 顶栏 | 契约侧对齐 `XyModel`；agent 侧用选择 / 装配语义 |
| 新公开类型第三套品牌 | `Xy*` 与 `AgentMessage` / `Command` / `Event` 混用本身合法 | `Xy*` = 库入口契约；`AgentMessage` = protocol 根词表；Command/Event = 信封载荷。新类型不得再发明第四套 |
| 根包仍叫 `xylitol` 且含全部 lib | 布局不标准 | 根收缩为 `xylitol-bin`（或等价 bin crate）；家族 crate 全在 `crates/`，插件在 `crates/plugins/` |
| feature 名 `server` | 产品词是 HostServer / 载体 | S0 可仍叫 `server`（Cargo 名）或改为 `host`；不得改 attach 行为 |

不改：`xylitol-tui`、`XyDriver`、`XyEvent`、`XyModel`、`vendor/gherkin` 包名。`xylitol-ai-bridge` 叶包名保留；HTTP 方言拆到 `xylitol-core-plugin-provider-*`（[§16](#AI-BRIDGE)）。

---

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
10. 为对称预拆 `xylitol-core-plugin-lsp` 等。

---

## 8. 需要拍板（未决）

未拍板时执行保守默认（括号内）。

1. **增量 vs 磁盘。** 已锁定 (a) 磁盘优先。`incremental = false` 不动。
   - (a) 保持磁盘优先，只加 mold / sccache / share-generics（**已执行**）
   - (b) `incremental = true` + 保持 `line-tables-only`，接受 `target/` 回涨
   - (c) 另加 `dev-fast` profile，不改默认
   说明：S0 之后 crate 变多，增量策略可能再变一次。
2. **瘦身激进程度。**
   - 保守：default 仍是 `cli,tui,otel,server`；只让关 feature 真能关；`rmcp` / `image` 变成可关（**默认**）
   - 激进：default 去掉 `otel`，甚至默认不编 MCP / image（全部 must-SDD）
3. **S0 一次切完 vs 分批。** 建议分批（W2.0 → W2.5）。一次切完与 ~11.8 万行、约 2493 个 `#[test]` 冲突更大。
4. **crate 搬移是否跳过 SDD。** 规则已允许行为不变则 skip-SDD。需要一句授权：「S0 搬移 skip-SDD；仅当产品行为变化才 propose」。
5. **mold 是否进 CI。** 续做时已写入 CI（`apt install clang mold`）。若要撤回，删 workflow 那两包，并改 `.cargo/config.toml` 不再强制 mold。
6. **是否授权** `cargo clean`**。** 已执行。`cargo clean` 未删尽 `debug/build`，已补删。
7. `wire` **改名时机。** 推荐随 W2.2 抽取做，不单独全树 rename。

---

## 9. S0 前必须改的 AGENTS / spec

改写只换稳定规则，不写 TODO，不留墓碑。

1. `src/AGENTS.md`「单 crate 逻辑分层（不为分层拆 crate）」→ crate 边界 = 插件替换 / 开关 / 生命周期；每个 crate 内部仍守分层；禁止按层各一 crate；继续禁止 `xylitol-domain`。
2. 分层图里 `protocol/{wire, ports, model, session}` 同一 crate 顶栏 → `wire` 进 `xylitol-protocol`，ports / 词表进 `xylitol-core`，session 实现进 plugin。`wire` ↛ `ports` 升为跨 crate 规则。
3. 「组合根才同时 import agent + infra」落点从 `app/core` → `xylitol-kernel`。S1 前仍是手工装配。
4. 根 `AGENTS.md`「主 crate 内逻辑分层（不拆 crate）」与 `src/AGENTS.md` 同步。`crates/` 角色表换成目标图。
5. `layer-architecture.feature` purpose「单 crate」→ validate-only 直接编辑。大的组织方向（分层依赖、crate 边界）可留在 spec。
6. 「开箱 vs 配置后置」拆成两句：Server 主线且 feature 真可关；MCP 配置后置且 compile 也可关。
7. `RunId` 不得进 `XyEvent` / wire → 「wire」换成跨端协议 / 信封。规则本身保留。
8. 扩展开闭：S0 的 `xylitol-core-plugin-*` 是官方静态 crate 边界，不是插件市场，不是 dylib。S4 才涉及动态加载。必须写明，避免与「不为未来插件堆抽象」对打。

不改的硬规则：Trust 门禁项目本地资源；工具开箱 allow-all；MCP 有配置才装配；产品 TUI 默认 attach 本机 Host；host ≠ 监听器定义；改名不留别名；`just qa`；worktree 独立 `CARGO_TARGET_DIR`。

---

## 10. 执行备忘

下一动作：

1. 改 [§13](#CORE) 推荐列（核 / 瘦 / 推迟 / 删）。未改列不删代码。
2. 按 §13 推荐列做低价值清理（行为变则 must-SDD）。
3. [W1.3b](#W1.3b) / W1.4，然后 W2.0。S0 里 `xylitol-ai-bridge` 拆分见 [W2.2b](#W2.2b) / [§16](#AI-BRIDGE)。
4. 预览 / wasm 不进本轮；结论见 [§14](#DESIGN-WASM) / [§15](#UI-WORKFLOW)。

每批提交信息（真正提交时）：

- 工具链：`build:` / `chore:`
- 组织搬移：`refactor:`
- 行为：`feat:` / `fix:`
- SDD 礼仪：`chore(sdd):` / `archive(sdd):`

---

## 11. 文档精简

完成：[是](#DOC)。不碰：`docs/architecture/`、`llmanspec/`、`_INTERVIEW/`、`delayed-changes/legacy/`。

---

## 12. 来源

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



## 13. 核心三面（拆 crate 前债务闸）

拆 S0 之前先收口「必须带走什么」。**三面 = 用户侧 + Host/服务端 + 可观测**。其余默认不进第一批 crate 图。

改 **推荐** 列：`核`（S0 必须搬走）/ `瘦`（代码留、先 feature-gate 或下沉测试）/ `推迟` / `删`。未改列 = 不准动。行为可见的删走 must-SDD。

### 13.1 用户侧

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
| `/debug` 场景 | `app/debug_fixtures` | 仅 debug_assertions；模块自述可整删 | 瘦（下沉 tests，见 W1.6） |
| `xylitol resources list\|info\|doctor` | `app/cli/resources` | 只读诊断 | 核 |
| CLI `serve install` | stub「未实现」 | 空壳 | 删 |
| 键位可视化 / `/hotkeys` | 未做 | GH #23 | 推迟 |
| gpui 桌面 | 未做 | GH #12 | 推迟 |

### 13.2 Host / 服务端

| 能力 | 落点 | 现状 | 推荐 |
|---|---|---|---|
| Host 监听 + 四象限信封 | `app/server` + `protocol/wire` | 产品 TUI 默认依赖；HTTP **不得删** | 核 |
| `XyDriver` 会话槽 / 单写者 ReAct | `app/core` + `agent/runtime` | 主线 | 核 |
| 内置工具 read/write/edit/bash/grep/find/ls/todo/ask | `infra/tools` | 开箱 | 核 |
| Trust 门禁 | `infra/trust` + CLI | 开箱 | 核 |
| 会话持久化 / 导出 HTML+JSONL | `infra/session` + `session_export` | 主线 | 核 |
| 自动压缩 + `/session-compact` | `agent/compaction` | 主线 | 核 |
| OpenAI 兼容 + Anthropic | `xylitol-ai-bridge` + `infra/provider` | 主线 | 核 |
| MCP（配置才装配） | `infra/mcp` | default feature 仍开 | 核（不删；零成本未配置） |
| hooks | `infra/hooks` + embed | 空配置零成本 | 核（W1.5） |
| `infra/tools/patch` | 模块在树，默认工具表未挂 | 无产品入口 | 删或并入 edit |
| serve 生命周期 `stop` | 只印 SIGTERM 说明 | 无 lock-file | 瘦 |

### 13.3 可观测侧

| 能力 | 落点 | 现状 | 推荐 |
|---|---|---|---|
| `provider-trace.jsonl` + fastrace 过程树 | `infra/observability` | 对照底座 | 核 |
| `xylitol.log`（`log`，永不 stdout） | `app/cli/logging` | 排障 | 核 |
| 窄读 `just obs-summary` / `obs-recent` / `obs-tui-lag` | `scripts/inspect_provider_trace.py` | 开发核 | 核 |
| inspect：requests / turns / lag / lifecycle / channel | 同脚本 | 人跑 | 核 |
| 可选 OTLP（`[otel]` 默认 none） | feature `otel` | default 仍开；lane/采样未做 | 核（不扩；W1.3b 只修路径） |
| samply `profile-suite` | `scripts/profile_tui_suite.py` | 维护 | 推迟（不进 crate 图） |
| 自研检视台 / Jaeger 必选 | 无 | GH #27 leftover | 推迟 |

### 13.4 暂不带走（债务）

| 项 | 说明 | 推荐 |
|---|---|---|
| gpui / LSP / DAP / Sub-Agent / Loop / Computer-Use | 未实现；禁止空壳 crate | 推迟 |
| delayed-changes 草案 | 不进 S0 文件搬家 | 推迟 |
| `agent_demo*` / `host_loop` | 引擎演示，非产品 SSOT | 留在 `xylitol-tui` |
| maka research | S1 对照 | 留 `docs/research/` |
| 23 个集成测试二进制 | 不删测试 | 瘦（share-generics 已做） |
| 重做 designing（wasm 预览） | [§14](#DESIGN-WASM)：当前无 wasm 基础 | 推迟 |

---



## 14. 调研：人-agent 共用「眼睛」看真实 TUI（2026-10-17，v2）

上一版（同日）答错了题：把问题收成「浏览器预览帧 / wasm 编不编过 crossterm」。你要的是：**真实产品组件出现在共享画面上，画面上有框选覆盖层（HTML inspect 那类），人和 LLM 对着同一套 UI/UX/状态讨论，并有可规范化的协作手续。** TUI 里很难直接画不破坏布局的覆盖层，所以才想到把 TUI app 编进 wasm、系统相关 patch 掉。本节按这个意图重写。未落地；不建 `designing/`、不装 wasm target、不进 S0。

### 14.1 问题拆开

| 需要 | HTML 世界已有的 | TUI 为什么难 |
|---|---|---|
| **真组件** 同一套 render | React 就是 DOM | 渲染真值是 ANSI 行，不是 DOM；第二套 TS 格子会漂 |
| **眼睛** 看见同一帧 | 人看浏览器，agent 看 a11y 树 / 截图 | agent 现在看 insta 文本或真 PTY 截图，没有「点这个框」 |
| **框选覆盖层** 标组件 | DevTools outline / Storybook inspect | 覆盖层若画进格子会改用户所见；终端仿真器也不给应用 HTML overlay |
| **状态目录** 切 UI/UX | Storybook stories | 有 `/debug` scene（`src/app/debug_fixtures`），但是手测，不是 inspect 目录 |
| **规范化协作** | locator / ref / selected node | 没有共享的「框 id → 固定区词汇 / 槽 / FoldTarget」契约 |

wasm 是**运输**（把真组件送到「能叠 HTML overlay 的表面」），不是目标。目标是 **Inspect 表面 + 框目录 + 人/agent 同一份帧契约**。

术语：框 = overlay box（网页层，不进 transcript）；格子 = 产品 `Component::render` 的单元格。固定区词汇仍走 `docs/architecture/TUI信息呈现与固定区词汇.md`（覆盖弹出框 / 侧栏 / 槽 / 通知条…）。

### 14.2 仓库里已经像「眼睛」的东西

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
| 旧预览 | 已删：`shell-frame.json` + TS `cell-grid.ts` | 有格子无框、无 agent 契约、且 TS 再绘 |

结论：不必从零发明 hit-test。缺的是 **命名框目录（inspect tree）** 和 **不污染格子的 overlay 宿主**。

### 14.3 HTML overlay 对 TUI 的正确类比

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

### 14.4 wasm 在这条链里扮演什么（patch 不是拦路牌）

若宿主是静态网页、不想每次 `cargo test` 出帧，才需要 wasm。编的不是「完整 xylitol TUI app」，是 **Inspect 切片**：scene → render_frame → InspectFrame。系统相关 **按切片 patch**，不是把 Host/MCP/fd 搬进浏览器。

| 模块 | 进 Inspect wasm？ | patch |
|---|---|---|
| `Component::render` / layout / widgets | 要 | 原样 |
| `Terminal` 实现 | 要一个 | `VirtualTerminal` 升库 API；或写格子到 JS。**不要** crossterm 的 TTY backend |
| `crossterm::event` 类型 | 要（键鼠形状） | 可继续当纯数据；mio/TTY 用 `cfg(not(target_arch = "wasm32"))` 挡掉 `crossterm` 的 terminal 特性。上游 [crossterm#995](https://github.com/crossterm-rs/crossterm/issues/995) 仍开 → 我们 **自管 Terminal，不求 crossterm wasm** |
| `PointerPipeline` / `HitRegionList` | 要 | 原样；inspect 框从这里 + 固定区 footprint **命名后**导出 |
| `autocomplete_fd` / `std::process` | 不要 | `cfg` 掉 |
| tokio rt | 尽量不要 | inspect 同步 `render_frame` 即可 |
| syntect / two-face | 可选 | wasm 体积大；story 可关 highlight |
| 产品 Host / MCP / Trust / 图像协议 | **不要** | scene 注入 `UiEntry` / `EditorSlot` / `FixedZoneOp`（现成 `debug_fixtures`） |
| 产品 slash / keybindings | 后 | 键注入走 `dispatch_event`，先只看态 |

wasm32-unknown-unknown **可以**走通的前提是：Inspect 切片从第一天就不链 crossterm-tty / fd / Host。这是产品分层已经部分满足的（host 驱动 render），不是幻想。工作量仍大（独立 SDD），但是 **为 overlay 宿主服务**，不是为「能在浏览器里聊天」。

对照业界：Termless（xterm.js/Ghostty 测终端 UI、可 PNG）解决的是「仿真器一致性」，**不**给组件框；Storybook 给 stories + inspect，但是 DOM。我们要的是二者的缝： **真 ANSI 组件 + Storybook 式框。**

### 14.5 三条宿主（同一契约，wasm 只是 C）

契约先于宿主。`InspectFrame` 稳定后，宿主可换。

| 宿主 | 人怎么看 | agent 眼睛 | 真组件？ | 何时 |
|---|---|---|---|---|
| **A 原生 dump** | 终端看格子；框在旁边 JSON / 另开 HTML 文件 | JSON | 是（VT） | **先做**：把 VT 升出 test-only；scene 渲一帧 dump |
| **B 薄 HTML overlay** | 格子 DOM + `position:absolute` 框（DevTools 样） | 同一 JSON；可选截图 | 是（帧来自 A，或 wasm） | A 能 dump 之后；**不要**再写组件 |
| **C Inspect wasm** | B 的页里 `wasm` 切 story / 键入 | 同一 JSON | 是（浏览器内 render） | 仅当「改代码立刻在页里切态」值得独立 SDD |

旧结论「优先 B 不要 wasm」在 **预览帧** 意义上仍对；在 **眼睛** 意义上：A/B 已够人-agent 讨论；C 是交互密度升级，不是正确性升级。

### 14.6 框目录（规范化的核心）

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

§13 若把 `debug_fixtures` 标成「瘦」：inspect 若立项，它更接近 **核（story 源）**，不是手测垃圾。等你改完 §13 推荐再动刀。

### 14.7 人-agent 协作手续（规范化，不是工具清单）

最小闭环（对标 Cursor 浏览器：snapshot → 点 ref → 再 snapshot）：

1. **开 story**：`scene_id` 来自 `DEBUG_SCENES`（或以后独立 catalog）。
2. **出帧**：得到 `InspectFrame`（格子 + 框）。
3. **指认**：人点 overlay **或** agent 报 `id` / `vocab`；双方只引用框 id，不说「上面那块绿的」。
4. **操作（可选）**：向真组件 `dispatch_event`（键/合成指针），再出帧。wasm 让这一步留在同一页；原生 A 则是「跑一次 harness」。
5. **记结论**：框 id + 期望（对比 DESIGN 词 / Palette），可落 insta 或 BDD；**不要**用 HTML DOM 当回归 SSOT。

agent 侧 skill 以后应读 JSON 框树，而不是先截图。截图只解决「色/间距好不好看」。

### 14.8 明确不做

| 不做 | 原因 |
|---|---|
| 产品 TUI 全量进 wasm（Host/MCP/Trust/图像） | 眼睛不需要真模型循环；scene 注入即可 |
| 在 ANSI 里画 inspect 框 | 破坏用户所见，也破坏 ApplicationOwned 选区 |
| 复活 bun+Vite 组件实现 | 第二套 UI，正是已删 `designing/` 的失败模式 |
| 为预览去 fork crossterm-wasm | 我们不需要它的 TTY；需要的是 VT |
| 本轮建 `designing/`、装 wasm target、开 SDD | 你还在改 §13；这是调研 |

### 14.9 若立项，建议切片（仍不本轮）

| 步 | 产出 | skip-SDD？ |
|---|---|---|
| 0 | `InspectFrame` / `InspectBox` 形状 + 与固定区词汇对齐（spec） | **must-SDD**（新用户/agent 可见面） |
| 1 | VT 升 crate 可选 API；scene→dump JSON；一个静态 HTML overlay 读文件 | dump 可 lab_；HTML 壳若仅开发者不 must |
| 2 | 补固定区无名框（footer 等） | 行为若只服务 inspect 宿主，随 spec |
| 3 | agent 读框树的 skill / 约定 | 团队 how-to → xylitol-dev |
| 4 | 可选 Inspect wasm 切片 | 独立 change，不进 S0 |

**结论（v2）**：要「真实组件 + 眼睛 + 框选 + 人/LLM 规范协作」，关键产物是 `**InspectFrame`（真渲染格子 + 命名框目录）**，宿主可以是原生 dump、薄 HTML overlay、或以后的 wasm 切片。wasm 合理，但只编 Inspect 切片并把 TTY/`fd`/Host patch 掉；**不是**把产品 TUI app 塞进浏览器。现有 `HitRegion` + `debug_fixtures` + 固定区词汇已经覆盖「半只眼睛」；缺的是命名、overlay 层分离、以及 agent 与人共用的帧契约。本轮只记，不实现。

工作流原语（story / xstate / BDD）与「rutis 之后 wasm 优先产品 TUI」见 [§15](#UI-WORKFLOW)。

---



## 15. 调研：TUI 交互工作流原语，以及 rutis 后 wasm 优先？（2026-10-17）

接 §14：web 想要的是 **GUI 式人-agent 协作面**（框选、指认、切态），不是第二套 UI。本节回答两问：① story + 代码态机 + XState/BDD 能否收成从设计到迭代的原语；② rutis 插件化之后，产品 TUI **wasm 优先**是否可行。未落地。

### 15.1 五层真值，不要压成一层

| 层 | 写什么 | 今日落点 | 管什么 |
|---|---|---|---|
| **WHAT** | 产品可观察行为 | `llmanspec/specs/app-tui-*.feature` 规则句 | 「Esc 须先关槽」 |
| **图** | 闭集转移 `from / event / guard / to` | 场景大纲 + llman-sdd XState **E1–E4**（表结构 / 确定性 / 可达 / 死锁） | 「有没有漏态、双 dest、死岛」 |
| **可执行抽样** | 假如/当/那么 + harness | `tests/bdd` + `TuiTestHarness` 按键→状态 | 这一跳在真组件上发生 |
| **Story** | 命名可达态（给眼睛看） | `DEBUG_SCENES` / `/debug` | 「Models 槽打开、busy、短终端」这一帧 |
| **眼睛** | 格子 + 命名框 | §14 `InspectFrame`；宿主宜 web | 人点框、agent 报 id |

XState 在本仓的角色是 **validate 时的图论门禁**，不是产品运行时。已用于 `RuntimePhase`（`agent-runtime` r2905）和 server 就绪态（`server-core` r1786）。**不要**在 TUI 里跑 xstate.js。运行时仍是 Rust 穷举枚举（`EditorSlot` 已是互斥脸）。

漂移记一笔：c2922 曾把 `specs.check_transitions: true` 写入 config；现行 `llmanspec/config.yaml` **没有**该键。图门禁是否还在跑，立项前先核对，不在本轮修。

### 15.2 用代码描述 UI/UX/状态——对，但分清三种「代码」

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

### 15.3 rutis「插件化」到底卸掉什么

对照 `xylitol-x-rutis-temp`（S1 靶心，本轮不引入 rutis）：

| 已钉 | 含义 |
|---|---|
| TUI = **客户端插件（轴 D）** | 订阅事件，**自己合帧** |
| **TUI 合帧内部是红线** | 不把每帧拆成运行时插件 |
| 产品信封 ≠ rutis-bridge | 新载体仍走四象限 RPC / `PROTOCOL_VERSION` |
| 热数据不进跨进程插件 | 每帧/每 token 不经 rutis 叶子 |

**词表（纠正）**：「插件」是大概念（可替换实现，按平台换绑）。rutis 只是其中一种带生命周期开销的运行时库。现行代码词是端口 + 组合根（`XyTool` / `XyModel` / `XySessionStore` / `XyPermission` / `BashOperations` / `RuntimePorts`）。内置 Rust 实现、feature 换绑、rutis fiber 都是这个大概念下的不同装配方式。单写者、信封、Trust 的产品语义仍可由内置 Rust 模块执行，不必先上 rutis。Core 能不能叫插件，以后再谈。

### 15.4 叫「预览」，不要叫 profile / 检视工坊

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

### 15.4b 先前写错的两句

上一版把「wasm 优先」写成「默认入口从 TTY 换成浏览器」。你没说过要换默认。要的是预览，产品入口仍是 TTY attach。

上一版把「bash/读盘/MCP/Trust 进 wasm」写成「不行」。那句话只对产品真 Host 成立，对预览的假 Host 不成立。见下。

### 15.5 为什么「真」bash/盘/MCP/Trust 不能当浏览器主线（以及为什么 mock 可以）

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

### 15.6 和 rutis 的关系（收窄）

rutis 不决定预览能不能做。预览依赖的是端口已存在、组合根能换绑、以及格子导出。rutis 以后若来，换绑可以变成卸一个 fiber、挂另一个。现在用 Rust 内置实现加开关就够。Core 算不算插件、插件层级怎么切，以后再谈。这里只记：大概念插件不等于必须是 rutis。

### 15.7 明确不做（修正后）

| 不做 | 原因 |
|---|---|
| 等 rutis 才开始 VT / 假 Host 绑定 | 端口和 scene 已在 |
| 把预览写成产品默认入口 | 预览用于设计和开发协作 |
| 预览 wasm 里真 exec 用户仓库 | 保真度错了；用磁带 |
| 用 xstate 解释器驱动产品 TUI | E1–E4 只看表 |
| 现在就钉「core 必须是 rutis 插件」 | 架构另开；先端口 |

**结论（v5）**：这形态叫 **预览**。同一套 TUI client，挂 **假 Host**，开在 **预览页** 上，用 **场景** 切换状态。代码里已有 `PreviewInject` 和 `/debug`。缺的是把假 Host 收成与真 Host 对称的组合根，以及带框选的预览页。本轮只记，不实现。

---

<a id="AI-BRIDGE"></a>
<a id="W2.2b"></a>

## 16. 调研：拆 `xylitol-ai-bridge`（2026-10-18）

结论：现在这个包把 LLM 叶 DTO 和两套 HTTP 栈捆在一起。S0 必须拆。叶留下；OpenAI / Anthropic HTTP 分别进 `xylitol-core-plugin-provider-*`。不要再抽 `xylitol-llm-types`，也不要把 DTO 搬回主仓。

引用：`../rutis/xylitol-x-rutis-temp/skills/xylitol-rutis-rewrite/references/` 的 `crate-graph.md`、`crate-split-detail.md`、`split-principles.md`。那份调研把 `xylitol-ai-bridge` 标成「迁入 crates/ 不在范围」，并让两个 provider 插件都依赖整包。本仓要补上这一刀。

准入：P1 两个真实方言；P6 `async-openai` 不得进 Anthropic 构建。行数来自 `crates/xylitol-ai-bridge/src` 的 `wc -l`，合计 9172。

### 16.1 功能分类

| 块 | 约行 | 热度 | 准入 | 去向 |
|---|---|---|---|---|
| DTO（`dto/message.rs` + `chunk.rs` + `json_parse.rs`） | 594 | 契约稳；流式块每 token | P5 | **叶** `xylitol-ai-bridge` |
| accounting | 189 | compaction 切点（`heuristic_token_count`） | 与 DTO 同叶 | **叶**。禁止再引入 tiktoken / RemoteCount |
| usage 正规化 | 359 | 响应结束时 | 跟方言 JSON 一起变 | 叶可留类型；from_openai / from_anthropic 函数跟方言走 |
| WirePolicy + defaults | 206 | 请求装配 | 两方言共享常量 | **叶** |
| thinking 解析 | 480 | 请求前一次 | 解析留叶；`apply_thinking_*` 跟方言 | 拆函数，不整文件跟 HTTP |
| FakeProvider | 571 | 测 / 预览 | 无 HTTP | **叶** |
| error / hooks HeaderBag | 78 | 契约 | P5 | **叶** |
| OpenAI Responses + Completions + client + assembler + hooks mw | ~3510 | **变更热点**；`async-openai` 重 | P1 / P6 | **`xylitol-core-plugin-provider-openai`** |
| Anthropic Messages | 1194 | 热点；栈是 reqwest | P1 / P6 | **`xylitol-core-plugin-provider-anthropic`** |
| dialect/deepseek | 71 | 冷；OpenAI 兼容 courtesy | 跟 Completions 一起改 | openai 插件 |
| tool_wire | 115 | 工具名过网关 | 两方言 | **叶** |
| trace + obs_session + attribution | 1344 | 观测，非每 token 模型正文 | 随请求走 | 跟方言插件；共享 attr 可留叶 |
| config（live-provider 路径） | 115 | lab | 冷 | lab / 叶 config，不进内核 |
| 主仓 `src/infra/provider`（`AdapterXyModel` + `map.rs`） | 另计 | DTO→`XyChunk` 壳 | 跟方言装配 | 各方言插件内，不要第三层 wrapper crate |

### 16.2 和目标图怎么接

rutis 目标已经是 `xylitol-core-plugin-provider-openai` / `-anthropic`。缺的是：这两个插件今天若依赖整包 `xylitol-ai-bridge`，Anthropic 构建仍会编 `async-openai`。

目标依赖：

```text
xylitol-ai-bridge          叶：DTO / accounting / WirePolicy / FakeProvider / 可移植 hooks 类型
        ▲                         ▲
plugin-provider-openai     plugin-provider-anthropic
（async-openai）            （reqwest Messages）
        ▲                         ▲
              xylitol-bin / kernel 装配
```

插件不横向依赖。主仓 `infra/provider` 的 map 壳搬进对应插件。`AiBridgeLlmAdapter` trait 留在叶上。

### 16.3 不要做

| 不要 | 原因 |
|---|---|
| 抽 `xylitol-llm-types` | `crates/xylitol-ai-bridge/AGENTS.md` 禁止第三套类型包 |
| DTO 搬回主仓 | 同上；叶仍是 LLM 形状 SSOT |
| 按文件对称再拆 assembler / hooks mw / client | 和 Responses 一起改，拆开会让一次方言改动跨多个 crate（split-principles §2.1） |
| 为 DeepSeek 单独 crate | 71 行 overlay，跟 Completions |
| 等 S1 rutis 再拆 HTTP | P6 在 S0 就能隔离依赖 |
| 把 provider-trace 当内核 | 观测跟 LLM 请求走 |

### 16.4 批次

W2.2b 在 W2.2 之后、W2.3 之前。行为不变则 skip-SDD；`package-ai-bridge*.feature` 主语若写成单包 HTTP，validate-only 改句子。

本轮只记分类，不搬代码。
