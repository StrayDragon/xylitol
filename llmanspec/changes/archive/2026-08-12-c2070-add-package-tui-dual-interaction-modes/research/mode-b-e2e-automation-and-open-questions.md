# Mode B：e2e 自动化缺口与后续开放题

> 日期：2026-08-12
> Change：`c2070-add-package-tui-dual-interaction-modes`
> 人类决策（同日）：产品 app TUI **B-only**；库保留 **Inline + ApplicationOwned** 双独立入口（Pi 式双 renderer 方向）；demo **拆两文件**（禁 env 切模式）；fold cascade 前要 **更高自动化** 覆盖 Mode B 用户行为。
> 性质：Change 调研；非 live specs；仅本文件。

## 1. Current e2e coverage map

**闸与入口**（`justfile`；边界 SSOT `packages/xylitol-tui/AGENTS.md`「验证」；how-to `.claude/skills/test-tui-harness/SKILL.md`）：

| 层 | 命令 | 事实 |
|---|---|---|
| 1–4 包测 | `just test-tui` | 进 `just qa` |
| 5 PTY | `just test-tui-e2e-pty` | `#[ignore]`；`tests/tui_e2e/pty.rs` |
| 5 tmux | `just test-tui-e2e-tmux` | 缺 tmux 则 skip |
| 满闸+5 | `just qa-e2e` | `qa` + 全层 5 |
| 人验 Mode B | `just demo-tui-alt-screen` | `--example agent_demo_alt`（无 env 切模式） |

**层 5 已覆盖（几乎全是 Mode A / 默认 demo）** — `tests/tui_e2e.rs` + `pty.rs` / `tmux.rs`：

- **pty demo**：就绪针 `DEMO_READY_NEEDLE`（`theme:dark`）、键入存活、Kitty 查询、bracketed paste、Enter 提交、Command Plate / Settings、窄屏 CJK。
- **pty mouse lab**：`pty_agent_demo_mouse_opt_in_enable_then_exit` — `XYLITOL_TUI_MOUSE=1` 的 Enable/Disable CSI；**不是** `ApplicationOwned` / alt-buffer 会话。
- **pty product Fake**：resize、hello/exit、bang、session tree、大 scrollback 等（产品仍默认 Inline / ath30）。
- **tmux**：启动、SGR、CJK 提交、palette/settings、窄屏 CJK。

**Mode B 已自动化处（进程内，非真终端）** — `packages/xylitol-tui/tests/interaction_modes_test.rs`（+ BDD `tests/bdd/steps_package_tui_interaction_modes.rs`）：alt+mouse begin/end、dock 排除、paint 高度帽、OSC52、suspend/resume 重挂、滚轮视口、copy-notice、exit dump、dock Down 回落 Editor。`virtual_terminal_test.rs` 另有 suspend 不刷帧。

**层 5 未自动化的 Mode B 用户行为**（人验 H1–H7 / `tasks.md`；`_HANDOFF.md`）：

| 行为 | 现状 |
|---|---|
| alt-buffer 进出 CSI（`?1049h/l`）真 PTY | 仅 VT 单测；e2e **无** `MODE=b` spawn |
| transcript 拖选高亮 → 松手 OSC52 | 单测有；**无** pty 注入 mouse CSI 断言 |
| 滚轮 sticky / 视口滚 | 单测有；e2e **无** |
| 拖选进 dock 夹边续选 | 单测+人验；e2e **无** |
| 退出 dump 主屏可上翻 | 单测有；e2e **无**（且 `CapturedScreen` 注释：**无 alt-screen / scroll region** — `tests/tui_e2e.rs`） |
| Editor 多行选区 / 空点不 copy | 人验 PASS；e2e **无** |
| Copied 短提示可见 | 单测 flag；屏上文案 e2e **无**；产品落点延后 `_HANDOFF.md` |
| suspend/resume（Ctrl+G）真终端 | VT 有；pty/tmux **无** Mode B 路径 |
| 边沿 autoscroll（Editor） | **已裁 / WONTFIX** — 勿再自动化回归「应有」 |

## Landed (2026-08-12) — demo split + Mode B PTY MUST 子集

| Item | Status |
|---|---|
| `agent_demo` / `agent_demo_alt` + `agent_demo_impl` | ✅；`just demo-tui` / `demo-tui-alt-screen`；禁 env 切模式 |
| PTY `pty_agent_demo_alt_mode_b_alt_mouse_and_exit_dump` | ✅ alt `?1049h/l` + mouse enable/disable + dump marker |
| PTY `pty_agent_demo_alt_mode_b_drag_select_osc52` | ✅ SGR drag → OSC52 |
| PTY wheel / dock clamp / suspend | ✅ `pty_agent_demo_alt_mode_b_{wheel_smoke,dock_clamp_copy,suspend_resume_restores_alt}` |
| 层 5 仍以 Inline `agent_demo` 为主场景 | 不变；Mode B 为新增 `*_alt_*` 用例 |

剩余 SHOULD：tmux Mode B 粗烟、CapturedScreen alt 支持、Editor 多行选区 pty。

## 2. Proposed automation matrix

行 = ptim / H 行为；列 = harness。**MUST** = 产品 B-only / fold cascade 前；**SHOULD** = 加固或 nightly。

| 行为 | unit | VirtualTerminal | pty | tmux | human | 闸级 |
|---|---|---|---|---|---|---|
| H1 alt 进出 + mouse Enable/Disable | ✓ | ✓ | **加** CSI 探针 | SHOULD | 抽检 | MUST pty |
| H2 拖选 + OSC52 | ✓ | ✓ | **加** mouse 序列 + raw OSC52 | — | Ghostty | MUST pty |
| H3 滚轮 sticky | ✓ | ✓ | SHOULD | — | Ghostty | SHOULD |
| H4 dock 夹边续选 | ✓ | ✓ | SHOULD（坐标敏感） | — | **MUST 抽检** | MUST human+unit |
| H5 exit dump scrollback | ✓ | ✓ | **加**（需扩展 capture 或 quit 后读主屏字节） | SHOULD | H5 | MUST pty 或加固 VT |
| H6 Editor 多行选区 | 部分 | ✓ | SHOULD | — | H6 | MUST unit；SHOULD pty |
| H7 copy-notice UI | ✓ signal | ✓ | SHOULD 针文案 | — | demo OK；产品延后 | MUST unit；产品 human |
| ptim11 suspend/resume | ✓ | ✓ | SHOULD | — | Ctrl+G | SHOULD |
| 产品 Fake + Mode B host | — | app harness | **加** Fake B-only smoke | — | 产品壳 | MUST（切默认 B 时） |
| fold 点选（c2040 前置） | 未来 | 未来 | 未来 | — | 人验 | 本票只留闸位 |

策略：优先 **pty + `agent_demo` Mode B 二进制**（拆文件后固定 spawn `agent_demo_alt` 类，禁 env 开关）；tmux 继续做「真仿真器可见」粗烟，不背负像素级拖选。

## 3. Environment matrix（Pi 一手）

来源：`../pi/packages/coding-agent/docs/terminal-setup.md`、`usage.md`（fullscreen）；产品选型背景见 `research/keep-or-drop-inline-mode.md`。

| 环境 | 风险（Pi / 本仓） | CI | Nightly | Human-only |
|---|---|---|---|---|
| portable-pty + xterm-256color | 无真 GPU/trackpad；alt 解析弱于真 emulator | **层 5 pty MUST** | — | — |
| tmux | Shift+Enter / Ghostty 旧 LF map；mouse 经 mux | **现有 smoke** | Mode B 粗烟 | 键位怪异时 |
| Ghostty | fullscreen：链接 hover 需 Shift+Ctrl/Cmd；人验拖选主场 `_HANDOFF` | — | 可选录屏 | **拖选/滚轮 MUST 抽检** |
| WezTerm | Option+Enter / IME 硬件光标（Pi） | — | — | 键位+IME |
| iTerm2 | trackpad 滚轮丢 delta；fullscreen 图→placeholder | — | — | **滚轮/图** |
| SSH | Pi：本机键盘探测失效；alt 逃生阀若产品砍 A 则更关键 | — | — | **B-only 前签一轮** |
| Windows Terminal | Alt+Enter 抢全屏（Pi） | — | — | 若交付 Win |

`CapturedScreen` 无 alt-screen → dump / 会话中主屏断言要么扩 oracle，要么 quit 后对 raw PTY 字节做弱断言，勿假装现网格拉得住 H5。

## 4. Open questions（深挖优先级）

1. **P0 — 双入口重构形状**：单 `TUI`+运行时分支 → Pi 式两类型/两 paint 路径？公共差分引擎如何抽？成本见 `keep-or-drop-inline-mode.md` §2。
2. **P0 — ath30 改写**：产品 MUST 默认 B；库仍暴露 A。propose 范围、默认切换时机（c2070 收尾 vs c2040 可点折叠后）。
3. **P0 — Mode B 层 5 最小集**：哪 2–3 条 pty（alt CSI、OSC52 拖选、dump）算「可 fold」？是否先扩 `CapturedScreen`？
4. **P1 — dump vs 会话中 scrollback**：dump 是否足以替代 Mode A 原生上翻？不足则 A 逃生阀形态（env / settings / legacy 标签）。
5. **P1 — fold 命中 vs 选区**：未修饰 click-expand 与拖选阈值谁优先？（`native-selection-vs-click-fold.md` / `pi-starline-…`）自动化如何拆「微拖 vs 单击」？
6. **P1 — demo 拆文件迁移**：`just demo-tui*`、就绪针、现有 pty/tmux spawn、`XYLITOL_AGENT_DEMO_MODE` 删除清单与兼容窗口。
7. **P2 — 产品 Copied / 误触**：壳层槽位 vs dock；单击阈值（`_HANDOFF` 延后）。
8. **P2 — SSH/tmux 人验签字人**：B-only 上线前谁签、清单是否进 app AGENTS。

## 5. Suggested next research tickets

1. **Mode B PTY 最小自动化设计** — 选定 alt/mouse/OSC52/dump 的注入与断言契约，含 `CapturedScreen` 是否扩 alt。
2. **双入口 API 草图对照 Pi** — `TuiMainScreen`/`TuiAltScreen` vs xylitol 拆分边界与共享差分核。
3. **ath30 / 产品 B-only spec 差分稿** — 默认、逃生阀、与 `ptim14` 下游清单对齐（不改 live 直至 propose）。
4. **Demo 双二进制迁移清单** — just / e2e / AGENTS / skill 指针一次性清 env 开关。
5. **Fold 命中与选区冲突矩阵** — 为 c2040 预置可测场景表（单击/微拖/dock/overlay）。
6. **终端人验签字板（B-only 门禁）** — Ghostty+tmux+SSH（+可选 iTerm/Wez）谁签、失败是否挡默认切 B。

---

**Sources**：`justfile`（`test-tui*` / `qa-e2e` / `demo-tui*`）；`packages/xylitol-tui/AGENTS.md`；`tests/tui_e2e{,/pty.rs,/tmux.rs}`；`.claude/skills/test-tui-harness/SKILL.md`；`packages/xylitol-tui/tests/interaction_modes_test.rs`；`tasks.md` / `_HANDOFF.md`；`research/keep-or-drop-inline-mode.md`；`../pi/packages/coding-agent/docs/{terminal-setup,usage}.md`。
