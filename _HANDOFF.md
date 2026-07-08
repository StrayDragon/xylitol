# _HANDOFF — xylitol-tui：pi-tui 完整 Rust 重写（交接给下一个 agent）

> 最后更新：2026-07-09（show_all/Markdown table 修复后）
> 分支：`feat/tui-dev`，working tree dirty（含未提交 RenderError/Markdown table 修复）
> 最新 commit：`13845c7 feat(tui): align exports + add show_all kitchen-sink demo`

---

## 〇、接手 agent 必读（30 秒）

**目标**：把 `packages/xylitol-tui` 完整对齐 pi-tui（`kimi-code/packages/pi-tui`），**不考虑障碍，做好底层支持**。对齐后才替换 `src/app/tui/engine/`（路线 B）。

**当前状态**：阶段 0-4 + c405/c410/c415/c420/c425/c430 已完成并入 commit；`show_all` kitchen-sink demo 已加入。**22/22 可移植模块已建，editor/autocomplete 已大幅补齐；stdin-buffer 走 crossterm 替代路线，不再移植独立模块**（见 §二）。

**怎么工作**：每个移植任务 = 一个 llman SDD 变更（`/llman-sdd-propose` → apply → archive → commit）。测试走 c405 五层 harness（见 §四）。pi 源在 `../kimi-code/packages/pi-tui`。

**下一步**：复核 Markdown/Editor/overlay 细节 parity，补窄宽/emoji/真实终端 E2E；package 稳定后再进入 §五 的 src/app/tui 路线 B 对接。

---

## 一、当前进度总览

| 指标 | 数值 |
|---|---|
| 源码行数 | **11,751 行**（packages/xylitol-tui/src）|
| 测试数 | **246 全绿**（xylitol-tui，本轮新增满宽 diff + Markdown table 回归）|
| E2E | **6 既有 E2E**（3 pty 含 kitty query + 2 tmux + bracketed paste）；本轮手动 tmux 108x40 验证 `show_all` |
| clippy | `-p xylitol-tui --all-targets -D warnings` clean |
| 已落地 spec | `tui-testing`(tt01-06) / `terminal-protocol`(tp01-04) / `paste-burst`(pb01-03) / c420-c430 SDD artifacts |
| commit（重写起）| 25（`5c55d86`→`13845c7`）|

### 已完成变更

| 变更 | 内容 | commit |
|---|---|---|
| 阶段 0-4 | pi-tui 22/22 可移植模块移植 + doRender 核心管线 | `5c55d86`→`b0a22b5`（14 commits）|
| **c405** | 五层 TUI 测试 harness | `451e8c4`/`0860818`/`3e9bc66` |
| **c410** | terminal 协议（Kitty 探测 + modifyOtherKeys + OSC + drain）| `e7b0f8f` |
| **c415** | paste-burst 移植 | `9495996` |
| **c420** | autocomplete debounce + fd + CancellationToken | `5d977da` |
| **c425** | editor core VisualLine+stickyColumn+pageScroll+history+PasteBurst | `af656b7` |
| **c430** | editor autocomplete SelectList 集成 + SDD artifacts | `9b2c18d` |
| stdin-buffer 决策 | 删除独立 stdin_buffer，采用 crossterm 事件解码 | `a21b829` |
| show_all demo | 对齐导出 + 新增 kitchen-sink demo | `13845c7` |
| 未提交修复 | render batch 临时关闭 DECAWM 自动换行；Markdown table 窄宽/表头/样式 cell 修复 | working tree |

---

## 二、待移植清单（核心交接内容）

**原则**：完全对齐 pi，每个任务一个 SDD 变更（c420+），配套测试随功能写。

### 完整模块对照表（pi .ts → xy .rs，按缺口大小排序）

| 模块 | pi 行 | xy 行 | 缺口 | 状态 |
|---|---:|---:|---|---|
| **components/editor** | 2415 | 1914 | -21% | ✅ c425 VL/sticky/PasteBurst/history + c430 autocomplete 集成；仍需细节 parity 复核 |
| autocomplete | 912 | 802 | -12% | ✅ c420 补齐 async + fd + debounce |
| stdin-buffer | 434 | 0 | — | ✅ 不移植独立模块；`a21b829` 删除，crossterm 负责 escape/bracketed paste/Kitty 事件 |
| tui | 1710 | 942 | -45% | ✅ doRender 核心完整；未提交修复补 DECAWM 满宽行保护 |
| keys | 1400 | 1163 | -17% | ✅ |
| utils | 1214 | 1087 | -10% | ✅ |
| terminal | 531 | 354 | -33% | ✅ c410 补齐协议，stdin 接入跳过 |
| markdown | — | 1050 | — | ✅ 表格已支持窄宽约束；未提交修复补 `TableHead`/styled cell 解析 |
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
