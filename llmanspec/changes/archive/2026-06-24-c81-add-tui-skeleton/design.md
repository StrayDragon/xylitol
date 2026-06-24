# c81-add-tui-skeleton — Design

## Context

- PRD: `docs/tui-prd.md` (draft v1) — 定义 Codex 风格单列流式 TUI 架构
- 已有 ratatui 代码: `src/interface/diff_review/cli.rs` (843 行同步 ratatui, 保持原样不改动)
- 依赖: ratatui 0.30.2 + crossterm 0.29 + ratatui-textarea 0.9.2
- 本 change 只实现 P0 骨架，后续 change 叠加能力

## 架构决策

### D1: 两层 retained 而非全局组件框架

| | 旧方案 (c80) | 新方案 (本 change) |
|---|---|---|
| 状态管理 | 全组件 retained + is_dirty/mark_clean | state/ 层状态常驻 + render/ 层每帧全量投影 |
| 渲染 | 字符串行差分 | ratatui Buffer cell diff (免费) |
| Markdown 缓存 | 无 | 唯一的 retained widget (MarkdownView, P3 实现) |
| 理由 | — | ratatui Terminal::draw 已有 Buffer diff, 再做组件 dirty 是重复 |

详见 PRD §2.1。

### D2: select! 三路事件循环

```rust
loop {
    tokio::select! {
        Some(evt) = agent_rx.recv() => { app.transcript.apply(evt); dirty = true; }
        evt = input_rx.recv()       => { dispatch(input::translate(evt), &mut app); dirty = true; }
        _ = frame_timer.tick()      => { if dirty { terminal.draw(|f| render::project(&app, f))?; dirty = false; } }
    }
}
```

- agent_rx: `AgentEventStream` (impl `futures::Stream`)
- input_rx: crossterm `EventStream` (feature `event-stream`)
- frame_timer: 16ms (60fps 节流)
- `dirty: bool` 只控制 draw 是否被调度, 不控制 frame 内画什么

### D3: 三层键盘模型 (decode → keymap → action)

```
crossterm::Event
  → input/decode.rs: 规范化
    → InputKey { key: KeyCode, modifiers: KeyModifiers }
  → input/keymap.rs: 表驱动映射 (纯函数)
    → (InputKey, FocusCtx, AgentState) → Option<Action>
  → input/action.rs: 副作用执行
    → Action::apply(&mut App, &mut AgentHandle)
```

state/ 和 input/keymap.rs 是纯函数, 不依赖 ratatui, 可终端无关单测。
keymap 是表驱动的, 支持未来从 `~/.xylitol/keymap.toml` 加载用户覆盖。

### D4: 纯文本 fallback (P0)

MVP 不做 markdown 解析。`TranscriptEntry::Assistant` 的 text 直接渲染为 ratatui `Paragraph`。
`#[allow(dead_code)]` 预埋 `MarkdownView` 骨架, P3 时填充实现。

### D5: 选中复制 (Zellij 模式 + OSC 52)

```rust
// render/select.rs
struct Selection {
    start: Option<(u16, u16)>,  // (viewport_line, col)
    end: Option<(u16, u16)>,
    active: bool,
}

impl Selection {
    fn start_from_mouse(row: u16, col: u16) { ... }
    fn extend_to(row: u16, col: u16) { ... }
    fn extract_text(&self, entries: &[TranscriptEntry]) -> String { ... }
    fn copy_osc52(&self) { ... }  // \x1b]52;c;{base64}\x07
}
```

## 模块文件结构

```
src/interface/tui/
├── mod.rs              // run_tui() 入口; select! 循环; terminal setup/restore
├── state/
│   ├── mod.rs          // App 聚合根 (Transcript + Composer + FocusCtx)
│   ├── transcript.rs   // AgentEvent → Transcript 状态 reduce (纯函数)
│   └── composer.rs     // 输入缓冲 + queue/bang + TextArea wrapper (纯函数)
├── render/
│   ├── mod.rs          // project(&App) → 布局 + widget 组装
│   ├── transcript.rs   // 消息列表投影 (纯文本 fallback)
│   ├── composer.rs     // 输入区投影 (ratatui-textarea)
│   └── select.rs       // Selection + OSC 52 复制
├── input/
│   ├── decode.rs       // crossterm → InputKey
│   ├── keymap.rs       // 表驱动映射 (纯函数)
│   └── action.rs       // Action enum + apply()
└── theme.rs            // 调色板 (Codex 风格)
```

## Key 绑定表 (P0)

| 输入 | FocusCtx | AgentState | Action | 说明 |
|---|---|---|---|---|
| Enter | Composer | Idle | Submit | 非 bang 时提交 |
| Tab | Composer | Running | QueueInput | 排队不提交 |
| Tab | Composer | Idle | (bang? NoOp : Submit) | 只有 bang 时不提交 |
| Esc | Composer | Any | ClearComposer | 清空草稿 |
| Esc (空) | Composer | Any | PrimeBacktrack | 预备回溯 |
| Esc+Esc (空) | Composer | Idle | LoadLastUserMessage | 装入上条消息 |
| Ctrl+C | Any | Any | Quit | 退出 |
| Ctrl+D | Any | Any | Quit | 退出 |
| MouseDown | Transcript | Any | SelectionStart | 开始选择 |
| MouseDrag | Transcript | Any | SelectionExtend | 扩展选择 |
| MouseUp | Transcript | Any | SelectionEnd + Copy | 复制选中文本 |

## 与旧 spec (c80) 的差异

| 维度 | c80 (旧) | c81 (本 change) |
|---|---|---|
| 布局 | 全屏 TUI 带分栏 | Codex 风格单列 transcript + composer |
| 组件模型 | ~25 个 Component trait 实现 | state/ + render/ 两层, 仅 state/ 是 retained |
| Markdown | termimad | P0 纯文本, P3 pulldown-cmark |
| 选中复制 | 未指定 | Zellij Selection + OSC 52 |
| 键盘 | RuntimeKeymap 抽象 | 三层 decode→keymap→action |
| 输入框 | 自研 | ratatui-textarea wrapper |
| diff_review | 复用 c75 内嵌 | 保持原样, 不迁移 |
