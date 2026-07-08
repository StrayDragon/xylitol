# _HANDOFF — xylitol-tui：pi-tui 完整 Rust 重写（交接给下一个 agent）

> 最后更新：2026-07-08（c415 后）
> 分支：`feat/tui-dev`，working tree clean
> 最新 commit：`9495996 feat(tui): c415 paste-burst 移植`

---

## 〇、接手 agent 必读（30 秒）

**目标**：把 `packages/xylitol-tui` 完整对齐 pi-tui（`kimi-code/packages/pi-tui`），**不考虑障碍，做好底层支持**。对齐后才替换 `src/app/tui/engine/`（路线 B）。

**当前状态**：阶段 0-4 + c405/c410/c415 完成。**22/22 可移植模块已建，但 editor/autocomplete/stdin_buffer 有内容缺口**（见 §二）。

**怎么工作**：每个移植任务 = 一个 llman SDD 变更（`/llman-sdd-propose` → apply → archive → commit）。测试走 c405 五层 harness（见 §四）。pi 源在 `../kimi-code/packages/pi-tui`。

**下一步**：§二 的「待移植清单」，按依赖序从 stdin_buffer（6.5）→ editor（6.4，最大）推进。autocomplete（6.3）c420 已完成。

---

## 一、当前进度总览

| 指标 | 数值 |
|---|---|
| 源码行数 | **10,650 行**（packages/xylitol-tui/src）|
| 测试数 | **243 全绿**（xylitol-tui）；workspace 全绿 |
| E2E | **6 实跑通过**（3 pty 含 kitty query + 2 tmux + bracketed paste）|
| clippy | `-p xylitol-tui --all-targets -D warnings` clean |
| 已落地 spec | `tui-testing`(tt01-06) / `terminal-protocol`(tp01-04) / `paste-burst`(pb01-03) |
| commit（重写起）| 19（`5c55d86`→`9495996`）|

### 已完成变更

| 变更 | 内容 | commit |
|---|---|---|
| 阶段 0-4 | pi-tui 22/22 可移植模块移植 + doRender 核心管线 | `5c55d86`→`b0a22b5`（14 commits）|
| **c405** | 五层 TUI 测试 harness | `451e8c4`/`0860818`/`3e9bc66` |
| **c410** | terminal 协议（Kitty 探测 + modifyOtherKeys + OSC + drain）| `e7b0f8f` |
| **c415** | paste-burst 移植 | `9495996` |
| **c420** | autocomplete debounce + fd + CancellationToken | 待 commit |
| **c425** | editor core VisualLine+stickyColumn+pageScroll+history+PasteBurst | 待 commit |
| **c430** | editor autocomplete SelectList 集成 | 待 commit |

---

## 二、待移植清单（核心交接内容）

**原则**：完全对齐 pi，每个任务一个 SDD 变更（c420+），配套测试随功能写。

### 完整模块对照表（pi .ts → xy .rs，按缺口大小排序）

| 模块 | pi 行 | xy 行 | 缺口 | 状态 |
|---|---:|---:|---|---|
| **components/editor** | 2415 | 408 | -83% | ✅ c425 VL/sticky/PasteBurst + c430 autocomplete 集成 |
| autocomplete | 912 | 534 | -41% | ✅ c420 补齐 async + fd + debounce |
| stdin-buffer | 434 | 158 | -64% | ⏳ 6.5 待补 OSC/turbo |
| tui | 1710 | 940 | -45% | ✅ doRender 核心完整 |
| keys | 1400 | 1163 | -17% | ✅ |
| utils | 1214 | 1087 | -10% | ✅ |
| terminal | 531 | 354 | -33% | ✅ c410 补齐协议，stdin 接入跳过 |
| terminal-image | 488 | 529 | +8% | ✅ |
| terminal-colors | 73 | 179 | +145% | ✅ |
| 其余 12 模块 | — | — | — | ✅ 全部对齐或超出 |

### 跳过项（有意不移植，已确认）

| pi 模块 | 原因 |
|---|---|
| `native-modifiers.ts` | macOS 原生二进制，不可移植 |
| `index.ts` | 纯导出索引，等价 xy `lib.rs` |
| `stdin-buffer.ts` | crossterm 已处理 escape 拼接/bracketed paste/Kitty/OSC/mouse——stdin_buffer 是 pi 在 Node.js raw stdin 上必需的补丁，xy 不需要。已删 `stdin_buffer.rs` |
| terminal.ts 的 `enableWindowsVTInput` | crossterm 已处理跨平台 |
| terminal.ts 的 `normalizeAppleTerminalInput` | macOS 专属，xy 跑 Linux/crossterm |
| terminal.ts 的 `writeLogPath` 调试日志 | xy 用 tracing |

---

## 三、llman SDD 工作流（每个移植任务必须走）

```
/llman-sdd-propose <id>   # 生成 proposal + spec + design + tasks
# 实现...
just qa                   # fmt + clippy + test
/llman-sdd-archive <id>   # 归档（合并 spec）
git commit
```

- **change id**：`c{priority}-{verb}-{subject}`，priority 5 的倍数，递增（下一个 c420）
- **spec 命名**：领域名词（如 `paste-burst`，不是 `add-paste-burst`）
- **TOON 格式坑**：值含空格/逗号/冒号/方括号必须双引号；`\x1b` 转义不支持（写 `CSI`）；数组声明 `[N]` 必须匹配行数
- **strict 校验**：提案阶段 tasks 未勾选会报 warning（正常），全部完成 + design.md 存在才 strict 过
- 提案参考：`llmanspec/changes/archive/2026-07-08-c415-port-paste-burst/`（最新最简的范例）

---

## 四、测试 harness（c405 五层，已落地）

**后续移植必须配套用对应层测试**。详见 `packages/xylitol-tui/AGENTS.md`。

| 层 | 工具 | 定位 | 落点 | 何时用 |
|---|---|---|---|---|
| 1. 按键序列→状态 | `TuiTestHarness` | model 断言（抄 helix）| `tests/support/mod.rs` + `tests/harness_test.rs` | 组件交互测试主力 |
| 2. insta snapshot | `viewport_snapshot()` | 整屏渲染回归 | `tests/snapshot_test.rs` + `tests/snapshots/` | 布局/颜色/换行变化 |
| 3. 时序 | `Clock`/`MockClock` 或 `Instant` 参数 + `#[tokio::test(start_paused)]` | 确定性时间测试 | `src/clock.rs` | debounce/paste-burst/动画 |
| 4. proptest | 随机按键 + 不变量 | 状态机崩溃边界 | `tests/property_test.rs` | editor 移植后加不变量 |
| 5a. E2E 主力 | portable-pty + `CapturedScreen` | crossterm 真 PTY | `tests/tui_e2e.rs` + `tests/tui_e2e/pty.rs` | 启动序列/协议验证 |
| 5b. 真终端冒烟 | tmux 手写 wrapper | 真 SGR 颜色 | `tests/tui_e2e/tmux.rs` | 颜色回归（`#[ignore]`）|

**关键约定**：
- 1-4 层：`cargo test -p xylitol-tui`（快、in-process）
- 5 层：全 `#[ignore]`，只经 `just test-tui-e2e` 跑
- 时序**禁止** `thread::sleep`（必 flaky）；同步逻辑用 `Instant` 参数注入（c415 实战验证比 MockClock 更轻）或 `MockClock`；async 用 `start_paused`
- snapshot 变更用 `INSTA_UPDATE=always cargo test --test <name>` 接受，人工复核
- **prek trailing-whitespace hook 会删 .snap 尾空格**——snapshot 空行必须无尾空格（`viewport_snapshot` 已处理）

---

## 五、阶段 7：对接 src/app/tui/（package 完全对齐后）

**前置**：§二 待移植清单全部完成。然后走 `write-surface` skill：

1. **死代码分诊**（`.agents/skills/audit-dead-code/`）：扫 `src/app/tui/`，保留 seam（`RenderedLine`/`xyevent_to_rendered`/`commands.rs`/`Msg`/`HostAction`/`TuiApp`/`StreamBuffer`），删旧引擎
2. **写 TuiSurface seam**：`src/app/tui/surface.rs`，封装 `xylitol_tui::TUI`，提供 `push_event(XyEvent)` / `render()` → `Vec<RenderedLine>`，不破坏 arch_guard
3. **替换**：删 `src/app/tui/engine/{tui,component,keybindings,outcome}.rs` + `widgets/`；适配 `mod.rs`/`render.rs`/`app.rs`
4. **验证**：`just qa` + `just test-tui-e2e` + 手动启动

---

## 六、commit 历史（重写期）

```
9495996 feat(tui): c415 paste-burst 移植 — 非 bracketed paste 的 Enter 抑制检测器
e7b0f8f feat(tui): c410 terminal 协议补齐 — Kitty 键盘协议探测 + modifyOtherKeys + OSC
3e9bc66 fix(tui): viewport_snapshot 空行去尾随空格，避免 prek trailing-whitespace 冲突
0860818 docs: 更新 _HANDOFF — c405 完成，路线 B 两阶段规划（先补 package 再对接）
451e8c4 feat(tui): 建立 c405 五层 TUI 测试 harness — 覆盖按键序列/快照/时序/状态机/真终端
b0a22b5 docs(tui): 新增 showcase example 综合演示全部特性
bac2054 docs: 更新 _HANDOFF — 阶段 0-4 完成，183 测试全绿，对齐 pi-tui 22/22 模块
7cefd64 chore(tui): 修复全部 clippy warnings — 183 测试全绿，clippy clean
d49edda feat(tui): 移植 editor 组件（pi editor.ts → Rust，~500 行）
55063e8 feat(tui): 阶段 3 — 补齐 terminal_colors, editor_component, terminal_image, image, autocomplete, markdown
886e633 feat(tui): 移植 settings_list（pi components/settings-list.ts → Rust）
32398c4 feat(tui): input strict slice + Component::tick + loader 自驱动（2c 完成）
747939b feat(tui): differential render viewport scroll + diff 策略补全（doRender 2b-3b 完成）
0e75ee6 feat(tui): overlay 合成重写——样式继承 + workingHeight + viewportStart（2b-3a）
2fff0c2 feat(tui): 宽度溢出保护 + fullRender viewport 状态（doRender 2b-2）
3b79fee feat(tui): 加 render 节流调度 + viewport 状态字段（doRender 重写地基）
8730968 feat(tui): 移植 extract_segments（overlay 合成样式继承根源）
72eb70f fix(tui): 修 3 项独立高危偏差对齐 pi
d0212e3 test(tui): 建 vte-backed VirtualTerminal cell-grid 测试 harness
5c55d86 build(tui): 纳入 xylitol-tui workspace member 并对齐版本
```
