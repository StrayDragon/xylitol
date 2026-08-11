# Research: Keep or drop product Mode A (inline)?

> 日期：2026-08-12
> Change：`c2070-add-package-tui-dual-interaction-modes`
> 问题：产品 app TUI 是否应放弃双模式、只交 Mode B（alt-screen / `ApplicationOwned`），Mode A 仅留在 `packages/xylitol-tui` / demos / escape hatch？还是像 Pi 一样产品双模式？
> 性质：Change 调研；**不是** live specs；未改 specs、未 commit。
>
> **历史注（2026-08-12）**：产品侧 `HostSession::apply_interaction_mode` 热切换栈 **已删除**；模式仅构造期绑定。下文「换栈 / A↔B 往返」作废，仅作决策对照。

## 1. Verdict options（三选，非强制单一答案）

| 选项 | 含义 | 代价 / 收益 |
|---|---|---|
| **A) 产品双模式 forever**（对齐 Pi） | 产品可切换 Inline ↔ ApplicationOwned；默认可仍 A（今日 `ath30`）或日后 B | 长期维护换栈、dock 同步、设置文案；用户保留「终端原生选区 + scrollback」手感 |
| **B) 产品 B-only，库保留 A** | 产品 host 固定 B（或仅 env/内部 flag 逃生）；库 / `agent_demo` / PTY lab 仍测 Inline | 产品面删掉模式开关与多数 A 分支；折叠/鼠标路径单一；仍付「库双栈」实现税 |
| **C) kill A everywhere** | 删 Inline 差分路径、`previous_viewport_top` 语义、主屏 teardown | **极高**：引擎身份来自 pi MainScreen 差分；demo/e2e/历史 scrollback UX 一并崩；无充分证据支撑 |

**对照事实**：Pi 产品**双模式**，默认 `regular`，fullscreen 标 **experimental**（见 §3）。xylitol live specs 今日也是库双模式 + 产品默认 A（`ptim01`/`ptim08`/`ptim14`，`ath30`）。

## 2. Fork cost map

### Pi：两套类，产品择一换栈

- 库：`TuiMainScreen`（`mode = "regular"`，主屏 + scrollback）— `../pi/packages/tui/src/tui-main-screen.ts`
- 库：`TuiAltScreen`（`mode = "fullscreen"`，`?1049h` + mouse CSI + ScrollView 选区）— `../pi/packages/tui/src/tui-alt-screen.ts`
- 产品：`createInteractiveTui` — `tuiMode === "fullscreen"` → AltScreen，否则 MainScreen — `../pi/packages/coding-agent/src/modes/interactive/interactive-mode.ts`（约 343–354）
- 运行中切换 = 停旧 renderer、装新栈（同文件 `switchTuiMode`）；布局语义不可混用 — `../pi/tui-plan.md`「Why main-screen and alternate-screen layouts differ」

### xylitol：单 `TUI` + 运行时分支（非两 class）

| 分叉点 | 证据 |
|---|---|
| 模式枚举 | `InteractionMode::{Inline, ApplicationOwned}`，默认 Inline — `packages/xylitol-tui/src/interaction_mode.rs` |
| paint | `application_session_active` 时 `ModeBRuntime::project_frame` 压到 ≤ 终端高；否则 `previous_viewport_top` 差分 — `packages/xylitol-tui/src/tui.rs`（约 1528–1563） |
| mouse | Mode B：`mode_b.handle_mouse` 先消费；Mode A：产品不开 capture — `tui.rs` `dispatch_event`；`src/app/tui/AGENTS.md` 鼠标规则 |
| 选区 / 视口 | `selection.rs` + `mode_b.rs` **仅** Mode B 会话 — `packages/xylitol-tui/src/{selection,mode_b}.rs` |
| host 换栈 | `HostSession::apply_interaction_mode`：end B → set mode → rebuild children → begin B + `sync_mode_b_dock_rows` — `src/app/tui/host/mod.rs`（约 213–261） |
| 产品默认 | `TuiRunOptions.interaction_mode = Inline` — `src/app/tui/mod.rs`；`ath30` 钉默认 A |

相对 Pi「两实现 + Proxy」，xylitol 把分支压进同一引擎；产品若 B-only，**app** 分支可收缩，**库** 分支多数仍在。

## 3. Mode A 独有、B 的 dump/teardown 替不掉的部分

1. **Emulator-owned selection**：Mode A MUST 保持终端原生选区路径；且 MUST NOT 承诺「开 mouse 仍完整原生选区 + 无修饰应用点选」— `ptim08`（`llmanspec/specs/package-tui-interaction-modes/spec.toon`）。oneof 论证 — `research/emulator-vs-app-selection-oneof.md`。
2. **终端 scrollback UX**：MainScreen / Inline 把历史交给仿真器；用户滚历史上翻不经 app ScrollView — Pi `tui-main-screen.ts` 注释；xylitol 差分同族 — `pi-dual-tui-modes-and-xylitol-cost.md`。Mode B 退出 `finish_inline` **dump** 到主屏（`tui.rs` 1087–1107）只恢复「事后可上翻」，**不能**复现会话中「边聊边用终端滚轮扫历史 / 原生拖选」手感。
3. **历史上滚不可点**：inline 下进 scrollback 的行应用点不到 — `research/alt-hold-capture-and-scrollback-hit.md`；Pi 亦承认 main-screen 无法可靠对 scrollback 做 hit-test — `../pi/tui-plan.md`。
4. **Pi 对 fullscreen 的 experimental 谨慎**：CLI「`regular` (default) or experimental `fullscreen`」— `../pi/packages/coding-agent/docs/usage.md`；settings「fullscreen mode is experimental」— `../pi/packages/coding-agent/src/modes/interactive/components/settings-selector.ts`（约 637–640）；`getTuiMode` 默认 regular — `../pi/packages/coding-agent/src/core/settings-manager.ts`（约 136、1130–1131）。终端差异（iTerm 图、trackpad、链接 hover）见同目录 `terminal-setup.md`。
5. **SSH / 脆弱终端**：一手未单独证明「SSH 必坏 alt-screen」；可观察的是 Pi 把 fullscreen 当实验面 + 多终端 workaround。产品 B-only 等于去掉这条逃生阀——风险需人验（Kitty/foot/tmux/SSH），不是已证伪。

**Zellij（仅 Mode B 行为参照，非「删 A」证据）**：应用选区状态机与松手复制 — `zellij-server/src/panes/selection.rs`、`zellij-server/src/tab/mouse_handler.rs`；摘要 — `research/zellij-selection-scroll-copy-input.md`。Zellij 本身是 multiplexer 画布，**不是** coding-agent 双 TUI 先例。

## 4. 若产品 B-only，库仍必须保留什么

| 必须留 | 原因 |
|---|---|
| **差分 paint 引擎**（`previous_lines` / dirty 行） | Mode B 仍对**可见**帧做差分；只是先 `project_frame` 再 diff — `tui.rs`；删引擎 = 重写 |
| **`finish_inline` / teardown / dump** | Mode B 退出路径复用同 API；dump 是 ptim02 SHOULD — `tui.rs`、`mode_b.rs` `exit_dump_lines` |
| **`ModeBRuntime` + `selection` + mouse 管道** | 产品 B 的核心；`ptim03–07`、`ptim09–15` |
| **Inline 构造 / demos / lab** | `InteractionMode::Inline`、`just demo-tui`、PTY e2e、包测对照；`ptim01` 库 MUST 双模式 |
| **`previous_viewport_top`（A 路径）** | 仅 Inline 生长 scrollback 需要；B 会话内清零 — `tui.rs`。产品不用 A **不**等于可删字段，除非杀 C |

**可降级为「库能力但产品不接线」**：运行时 `set_interaction_mode(Inline)` 产品设置 UI、ath30「用户可选 A」文案。

## 5. 产品丢双模式后的简化机会

若选 **B（产品 B-only，库留 A）**：

| 可简化 | 说明 |
|---|---|
| 产品默认改 B | 今日 `TuiRunOptions` / `ath30` 默认 A → 需 **propose** 改 `app-tui-host`（本文件不改 specs） |
| `apply_interaction_mode` 开关面 | 可收成「启动只 begin B + 周期 `sync_mode_b_dock_rows`」；测里 A↔B 往返可缩 |
| 隐藏/冻结双模式设置 | 无 Pi 式 `/settings` TUI mode 时，直接不暴露；内部 `XYLITOL_*` 逃生可选 |
| `AGENTS.md` 产品鼠标规则 | 从「inline 不开 mouse」改为「产品 = Mode B 开 capture」；env 仍仅 lab |

**不宜假装能删**：`sync_mode_b_dock_rows`、copy-notice chrome（`ath31`）、换栈 rebuild（若仍保留逃生切 A）、库 `ptim01`/`ptim08`/`ptim14`。

若选 **C**：与「差分 fork 身份」冲突（`packages/xylitol-tui/NOTICE` / `PI_DELTAS.md`）；无一手收益证明。

## 6. Recommendation for xylitol — **人拍（2026-08-12）**

**采纳强化版选项 B，并升级库形态：**

| 项 | 决定 |
|---|---|
| 产品 | **B-only**（无用户双模式） |
| 库 | **双入口分治**（分别实现/管理；允许后续功能分叉）— 非「单 TUI + 产品双故事」 |
| Inline | **库能力 / 历史遗留赋能**，非产品面 |
| Demo | **两 example 文件**，禁止 env 切模式 |
| 验收 | Mode B → PTY/tmux 等高自动化 |

与早期「推荐 B」一致，并明确：**不要**产品双模式 forever（否决选项 A）；**不要** kill 库 A（否决选项 C）。`ath30` 改写与 demo 拆分见 `proposal.md` / `tasks.md` 7.5–7.7。

---

## Open questions for human

1. 产品默认切 B 的时机：c2070 收尾即切，还是等 c2040 点折叠可演示后再切？
2. Mode A 逃生形态：完全隐藏 / 仅 env·文档 / 仍进 settings（是否沿用 Pi「experimental」反向——把 **A** 标成 legacy）？
3. SSH / tmux / 老终端：B-only 前是否强制做一轮人验清单（谁签）？
4. `ath30` / `ptim14`「默认可为 Mode A」：接受改成「产品 MUST 默认 B；库仍暴露 A」吗？（须走 propose，非本调研改 spec）
5. 退出 dump 是否足以替代「会话中终端 scrollback」对核心用户？不足则选项 A（产品双模式）权重上升。
