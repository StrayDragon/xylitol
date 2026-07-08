# _HANDOFF — TUI 转向：pi-tui 完整 Rust 重写

> 最后更新：2026-07-07 18:00
> 分支：`feat/tui-dev`
> 当前阶段：**✅ 阶段 0-4 完成（全线对齐 pi-tui）→ 下一步：阶段 5 对接 src/app/tui/**

---

## 〇、当前进度总览

| 指标 | 数值 |
|---|---|
| commit 数（重写起） | 14（到 `7cefd64`） |
| 测试数 | **183 全绿** |
| clippy | `--all-targets -D warnings` **clean** |
| Rust 源码行数 | **9,352 行** |
| pi-tui 参考行数 | 12,144 行 |

### 已完成阶段

| 阶段 | 内容 | 结果 |
|---|---|---|
| 0 | workspace 纳入 + vte test harness | 13 测试 |
| 1 | 3 项高危修复（onSubmit/clear_on_shrink/Resize） | 通过 |
| 2a | extract_segments（overlay 样式继承） | +5 测试 |
| 2b-1 | render 节流（16ms） + viewport 状态 | +5 测试 |
| 2b-2 | 宽度溢出保护 + fullRender viewport | +3 测试 |
| 2b-3a | overlay 合成重写（workingHeight/viewportStart） | +2 测试 |
| 2b-3b | differential viewport scroll（CUD+\r\n） + diff 策略 | +3 测试 |
| 2c | input strict slice + Component::tick + loader 自驱动 | +3 测试 |
| 3 | 补齐 7 个缺失模块（terminal_colors/image/autocomplete/markdown/editor 等） | +7 模块 |
| 4 | 修 clippy warnings，编辑器与 markdown 补测 | 183 全绿 |

---

## 一、完整模块对照表（pi-tui → xylitol-tui）

### 核心模块（src/*.rs）

| pi-tui (.ts) | 行数 | xylitol-tui (.rs) | 行数 | 状态 |
|---|---|---|---|---|
| `tui.ts` | 1,714 | `tui.rs` | 940 | ✅ doRender 核心管线完整 |
| `utils.ts` | 1,188 | `utils.rs` | 1,087 | ✅ |
| `keys.ts` | 1,400 | `keys.rs` | 1,163 | ✅ |
| `terminal.ts` | 531 | `terminal.rs` | 85 | ✅ crossterm 薄封装 |
| `keybindings.ts` | 244 | `keybindings.rs` | 241 | ✅ |
| `fuzzy.ts` | 137 | `fuzzy.rs` | 198 | ✅ |
| `kill-ring.ts` | 46 | `kill_ring.rs` | 58 | ✅ |
| `undo-stack.ts` | 28 | `undo_stack.rs` | 29 | ✅ |
| `word-navigation.ts` | 117 | `word_navigation.rs` | 147 | ✅ |
| `stdin-buffer.ts` | 434 | `stdin_buffer.rs` | 158 | ✅ 核心功能，部分细节简化 |
| `terminal-colors.ts` | 73 | `terminal_colors.rs` | 128 | ✅ + 测试 |
| `terminal-image.ts` | 488 | `terminal_image.rs` | 400 | ✅ Kitty/iTerm2 协议 + 能力检测 |
| `autocomplete.ts` | 786 | `autocomplete.rs` | 534 | ✅ 文件/命令补全（不含 fd 递归） |
| `editor-component.ts` | 74 | `editor_component.rs` | 46 | ✅ trait 接口，不含 autocomplete |
| `native-modifiers.ts` | 59 | — | — | ⏭️ 跳过（macOS 原生二进制，不可移植） |
| `index.ts` | 114 | `lib.rs` | 41 | ⏭️ 跳过（纯导出索引，等价于 lib.rs） |

### 组件（src/components/*.rs）

| pi-tui (.ts) | 行数 | xylitol-tui (.rs) | 行数 | 状态 |
|---|---|---|---|---|
| `editor.ts` | 2,333 | `editor.rs` | 408 | ✅ 核心编辑功能，不含 autocomplete 集成 |
| `markdown.ts` | 858 | `markdown.rs` | 951 | ✅ pulldown-cmark 后端 + hook 高亮 |
| `input.ts` | 447 | `input.rs` | 468 | ✅ |
| `select-list.ts` | 229 | `select_list.rs` | 581 | ✅ |
| `settings-list.ts` | 250 | `settings_list.rs` | 606 | ✅ |
| `loader.ts` | 92 | `loader.rs` | 143 | ✅ |
| `image.ts` | 126 | `image.rs` | 160 | ✅ |
| `text.ts` | 106 | `text.rs` | 101 | ✅ |
| `truncated-text.ts` | 65 | `truncated_text.rs` | 62 | ✅ |
| `cancellable-loader.ts` | 40 | `cancellable_loader.rs` | 60 | ✅ |
| `spacer.ts` | 28 | `spacer.rs` | 26 | ✅ |
| `box.ts` | 137 | `panel.rs` | 319 | ✅ 重命名（功能完全一致） |

> **唯一有意跳过**：`native-modifiers.ts`（macOS 原生二进制）和 `index.ts`（导出索引 = `lib.rs`）。其余 **22/22 个可移植模块全部对齐**。

---

## 二、之后规划（阶段 5：对接 src/app/tui/）

### 优先级 P0：正确的 seam 设计（下周开始）

当前 `packages/xylitol-tui` 是独立可测的 TUI 引擎 crate。下一步是**替换 `src/app/tui/engine/` 和 `src/app/tui/widgets/`**，走 `write-surface` 方法论：

**步骤 1：死代码分诊**（见 `.agents/skills/audit-dead-code/`）
- 扫描 `src/app/tui/` 下当前自研引擎代码，区分「真死/逻辑死/预留」
- 标记需要保留的 seam：`RenderedLine`、`xyevent_to_rendered`、`commands.rs`、`Msg`、`HostAction`、`TuiApp`/`StreamBuffer`

**步骤 2：写新 seam（TuiSurface）**
- 新 crate 不直接暴露 `TUI`/`Component` 给 `src/app/tui/`，而是经一个新 seam：`TuiSurface`
- `TuiSurface` 封装 `xylitol_tui::TUI`，提供 `push_event(XyEvent)` / `render()` → `Vec<RenderedLine>` 接口
- 不破坏 `arch_guard`（TUI 层不经 `use crate::agent`）

**步骤 3：替换并删除旧引擎**
- 删除 `src/app/tui/engine/tui.rs`、`component.rs`、`keybindings.rs`、`outcome.rs`
- 删除 `src/app/tui/widgets/` 全部
- 适配 `src/app/tui/mod.rs`（host loop）、`render.rs`（seam）、`app.rs`（StreamBuffer）

**步骤 4：验证**
- `cargo test` 全量通过
- `just qa`（fmt + clippy + test + docs + prek）
- 手动验证 TUI 启动

### 已知差距（Pre-1.0.0 之前不处理）

| 事项 | 说明 | 风险 |
|---|---|---|
| `editor.rs` autocomplete 集成 | pi editor 内嵌了完整的 autocomplete 管线（AbortController、debounce、SelectList popup） | 中等（编辑体验影响小） |
| fuzzy 评分公式 | pi 用连续匹配 -consecutive×5 / gap / word boundary，Rust 评分简化 | 低（补全排序细微差异） |
| `stdin_buffer.rs` | 当前 158 行 vs pi 434 行，OSC reply 拦截、turbo 模式未完整移植 | 低（终端颜色探测基本工作） |
| `terminal.rs` 偏薄 | 未做 Kitty 键盘协议协商、cell dimension 查询 | 低（crossterm 已归一化输入） |
| Thai/Lao AM 规范化 | pi `normalizeTerminalOutput` | 极低（罕见 case） |
| `run_event_loop` async wrapper | 目前同步驱动，async 包装留待需要 | 极低（host loop 可自主驱动） |

### 设计决策总结

| 决策 | 说明 |
|---|---|
| render 节流 | 同步 `request_render(force)` + `try_render()` 检查 16ms，host 驱动 |
| 宽度保护 | 比 pi 更严：fullRender 和 diff 前都检查 |
| markdown 高亮 | `syntax_highlight: Option<Box<dyn Fn(&str, Option<&str>) -> Vec<String>>>` — 包不依赖 syntect |
| editor autocomplete | 延迟到对接阶段再集成（当前仅核心编辑基元） |
| `native-modifiers.ts` | macOS 原生二进制，跳过，Rust 替代方案留待以后 |
| keybindings | 沿用 pi 全局 pattern（`with_keybindings` 惰性初始化） |

---

## 三、commit 历史（重写期）

```
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

## 四、对接清单（阶段 5 执行时需要）

### 删除
- `src/app/tui/engine/tui.rs`
- `src/app/tui/engine/component.rs`
- `src/app/tui/engine/outcome.rs`
- `src/app/tui/engine/keybindings.rs`
- `src/app/tui/widgets/` 全部

### 适配
- `src/app/tui/mod.rs` — host loop 改用新引擎
- `src/app/tui/render.rs` — `xyevent_to_rendered` seam 保留
- `src/app/tui/app.rs` — `TuiApp`/`StreamBuffer` 保留

### 新增
- `src/app/tui/surface.rs` — `TuiSurface` 封装（新引擎 → RenderedLine 桥接）

---

## 五、TUI 测试 harness（c405，已落地）

五层测试架构，覆盖从纯函数到真终端的全部动态行为。**后续 editor/paste-burst/autocomplete 移植必须配套用对应层测试**。

| 层 | 工具 | 定位 | 落点 |
|---|---|---|---|
| 1. 按键序列→状态 | `TuiTestHarness` + `MutableComponent` | model 断言，抄 helix | `packages/xylitol-tui/tests/support/mod.rs` + `tests/harness_test.rs` |
| 2. insta snapshot | `viewport_snapshot()` → `assert_snapshot!` | 整屏渲染回归 | `packages/xylitol-tui/tests/snapshot_test.rs` + `tests/snapshots/` |
| 3. 时序 | `Clock`/`MockClock` + `#[tokio::test(start_paused)]` | paste-burst/debounce 确定性测试 | `packages/xylitol-tui/src/clock.rs` |
| 4. proptest | 随机按键 + 不变量 | editor 状态机崩溃边界 | `packages/xylitol-tui/tests/property_test.rs` |
| 5a. E2E 主力 | `portable-pty` + `CapturedScreen` | crossterm 真 PTY 事件解析 | `tests/tui_e2e.rs` + `tests/tui_e2e/pty.rs` |
| 5b. 真终端冒烟 | tmux 手写 wrapper | 真 SGR 颜色回归 | `tests/tui_e2e/tmux.rs` |

**关键约定**：
- 第 1-4 层跑在 `cargo test -p xylitol-tui`（快、in-process、精确）
- 第 5 层全 `#[ignore]`，只经 `just test-tui-e2e` 跑（慢、需真 PTY/tmux）
- 时序测试**禁止** `thread::sleep`（必 flaky）；同步逻辑用 `MockClock`，async 用 `start_paused`
- snapshot 变更用 `INSTA_UPDATE=always cargo test` 接受，人工复核
- E2E spawn 的是 `xylitol-tui` demo example（非完整 `xylitol` 二进制），解耦 LLM provider 依赖
