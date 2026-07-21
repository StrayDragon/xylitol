# src/app/tui/

本面专属边界。分层与 seam：`src/AGENTS.md`。引擎库：`packages/xylitol-tui/AGENTS.md`。AGENTS 写法：根 `AGENTS.md`。

写/改本面：**先读本节布局与职责**，再跟 `write-tui` skill。

**刻意差异台账**（不得静默覆盖）：本目录 [`PI_DELTAS.md`](./PI_DELTAS.md)；包层 [`packages/xylitol-tui/PI_DELTAS.md`](../../../packages/xylitol-tui/PI_DELTAS.md)。

## TUI File Layout

基于 `xylitol-tui` 的 **host 驱动**产品面（CLI 无参默认 TUI）。原子交互优先在 `packages/xylitol-tui` `agent_demo` 验证后再接线。

| 路径 | 职责（一句话） |
|---|---|
| `mod.rs` | 生产 `run_host_loop`：终端 / tick / agent 流 / bang 扇入 |
| `host.rs`（及子模块） | `HostSession::step` 同步步进机；`pending` / `input_policy` / `session_ops`；busy/idle 输入策略 |
| `effects/` | **唯一** `drain_pending` → XyDriver / dispatch；slash / pending_ui / bang 分文件 |
| `commands.rs`（可多文件） | slash / bang 解析 → pending；不执行副作用 |
| `bridge/` | `XyEvent` → `UiModel`（`model.rs`）；family handlers |
| `layout/` | 产品壳：`EditorSlot`、`UiRoot`（`root/`：slot_input / slot_nav / render）、theme、`slash_catalog` |
| `widgets/` | scrollback / queue strip / glyphs（组合件，非通用引擎） |
| `terminal_guard.rs` | 终端生命周期 / panic 恢复 |
| `harness.rs` / `tests.rs` | 合成切片；`HostEvent` + `TestTerminal` |
| `DESIGN.md` + `design/` | 视觉 / UX SSOT；playground 见 `design/AGENTS.md` |

历史合约 id「chrome」= layout/widgets；**新文案用 layout / 树槽 / widget**。勿用 `shell`/`scene` 命名（避 bash / 泛化场景混淆）。

## Module Responsibilities

- **`mod` / `HostSession`**：协调者。新逻辑能单测的 → 先下沉到 `effects` / `bridge` / `commands` / `layout` 子模块，**禁止**继续把业务堆进 God 文件。体量软顶/硬顶与超标表 → [`../QUALITY_RETUNE.md`](../QUALITY_RETUNE.md)（测试 `harness`/`tests` 另计）。
- **`effects`**：唯一异步副作用泵；harness MUST 复用，禁止第二套 slash/steer match。
- **`bridge`**：只更新 `UiModel`；layout MUST NOT match `XyEvent`。
- **`commands`**：只解析与 pending 类型；执行经 `drain_pending` → `protocol::Command` / `dispatch` 或 XyDriver。
- **`layout` / `widgets`**：呈现与局部交互；**MUST NOT** 直接调 XyDriver / 读写 session。
- **Trust**：CLI `trust_gate`，不在本面 Choice stub 上扩活逻辑。
- **Plate / Settings / Choice 槽**：stub 冻结，产品未拍板前勿扩。

## 硬约束

- 渲染/通用组件只用 `xylitol_tui`；缺能力先改包再接线。产品路径 **host 驱动**；勿调 `TUI::start()`（demo 专用）。
- Agent 只经 `app/core/driver::XyDriver`；禁止 reach `agent::session` / `runtime` / `infra`。
- Esc 归属（摘要；细节见 stage-QA design）：
  - Idle 空 editor → 双 Esc 开树（`UiRoot::on_escape`）
  - Busy 无 overlay → abort latch（host `try_busy_input`）；**立刻**臂装 Xy 抑制，drain 仍 MUST 调 `XyDriver::abort`
  - Busy + overlay → 先关槽，不 abort
  - Agent abort → `note_user_abort`（Aborted + `suppress_xy`）；Bang abort → `note_bash_cancelled`（`(cancelled)`，无 `suppress_xy`）
- 颜色走本面 theme token（`DESIGN.md`）。已确认需求须有 harness / BDD 护栏。

## 视觉 / Specs / Debug

- **视觉 SSOT**：`DESIGN.md` + `design/*`。三层：浏览器静图 / `just demo-tui` / 本目录生产。
- **Specs**：`app-tui-*`（勿再堆单体 `app-tui`）。steer/follow-up 经 XyDriver 队列（c461），本面不持有 ReAct 队列。
- **日志**：debug 默认写 `{agent_dir}/logs/xylitol.log`（默认 agent_dir 见 `DefaultResourceLoader::default_agent_dir`）；`RUST_LOG` / `XYLITOL_DEBUG=1`；埋点 `target: "xylitol::tui"`；禁止 `println!`。装配：`app/cli/logging.rs`。排障窄读：skill **`xylitol-inspect-runtime-logs`**。

## 验证

包侧五层 SSOT：[`packages/xylitol-tui/AGENTS.md`](../../packages/xylitol-tui/AGENTS.md)「验证」。

| 角色 | 做什么 |
|---|---|
| **Agent 必跑** | 相关 `harness.rs` / `tests.rs`（lib）；核心 BDD（`cargo test --test bdd -- --test-threads=1`）；`just fmt` + 相关 clippy；change `--strict`。产品 TUI 交互护栏以 harness 为准，不再维护 `app-tui-*.feature`。 |
| **Agent 尽量跑** | 真终端：`just test-tui-e2e-pty`；会话树满路径见 c705 `pty_product_fake_session_tree_*` |
| **人类确认** | 最短手测观感；**不**替代 harness |

人类路径示例：debug 构建 `/debug ` Tab 选场景 → 双 Esc；分叉用 `session-tree-branched`。夹具：`src/app/debug_fixtures/`。

## HOW

| 任务 | 去哪 |
|---|---|
| 写/改本面 | `write-tui` skill（先读上表布局） |
| 刻意差异 | [`PI_DELTAS.md`](./PI_DELTAS.md) |
| UX / token | `DESIGN.md` + `design/*` |
| 包能力 / 五层测 | `packages/xylitol-tui/AGENTS.md`；`test-tui-harness` |
| 新增应用面 | `write-surface` |
| 排查 | skill `xylitol-inspect-runtime-logs`（`just obs-*` / 级别 log `tail`；勿整文件灌上下文） |

历史/分支 UX 以会话树为准；live 只进 scrollback。不做 Codex TranscriptView；不做 Settings/Plate 运行时改配置。
