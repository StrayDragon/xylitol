# c110-update-tui-codex-parity — Design (Explore)

本设计文档在探索阶段作为“调研结果 + 交互真值(SSOT)”，目标是把 `../codex` 的 TUI 交互语义以可实现/可测试的方式抽取出来，并映射到 xylitol 的架构（AgentEvent/ToolRegistry/Security/ReviewEngine）。

> 约束：不改 `src/interface/diff_review/` 的 review diff comment UI；其余 TUI 可重写。

## Evidence (codex)

- Keymap defaults: `../codex/codex-rs/tui/src/keymap.rs`
  - `Ctrl+T` transcript
  - `Ctrl+G` external editor
  - `Ctrl+O` copy last response
  - `Ctrl+L` clear
  - `Alt+R` raw output
  - Reserved: `Ctrl+C` interrupt/quit, `Ctrl+D` quit, `/` slash popup, `!` shell, `@` file paths, `$` connector mentions
- Composer semantics: `../codex/codex-rs/tui/src/bottom_pane/chat_composer.rs`
  - `Tab` key = composer.queue
  - Queue rule: `Tab` queues when task is running; otherwise it submits immediately (except `!`)
  - Slash popup: when active, `Tab` completes selected slash command; `Enter` runs selected command
  - `Esc` dismisses slash popup (not quit)
  - `!` bang-shell command special-case: when not running, `Tab` must NOT submit; it’s for indentation/completion or no-op
- User-facing hints: `../codex/codex-rs/tui/tooltips.txt`
  - `Esc` backtrack description
  - `Tab` queue description
  - `Ctrl+O` copy description

## Mapping to xylitol

xylitol 当前 TUI 是三块：

- `App`：tokio::select! 事件循环；持有 `ChatComponent` / `ToolPanelComponent` / `InputComponent` / overlays。
- `AgentLoop`：输出 `AgentEvent::{TextDelta, ToolCallStart, ToolCallEnd, StepComplete, Error, RepeatDetected}`。
- `ApprovalHub + SecureApprovalToolWrapper`：工具审批阻塞执行。

为了复刻 codex 交互，我们需要补齐：

1. **Runtime keymap**：不要在业务逻辑里散落 `match KeyCode`。
2. **Composer state machine**：输入框不只是 textarea，需要把 slash popup / bang shell / queue/submit/cancel/backtrack 统一建模。
3. **Transcript / Pager overlay**：独立于 diff review overlay。
4. **Copy / Raw mode**：对 chat 渲染策略的切换与 clipboard 输出。

## SSOT: Desired Interactions (Phase 1)

以下是本 change 的交互真值（优先对齐 codex，必要时为了 xylitol 机制做最小差异）。

### Global keys

- `Ctrl+D`: quit interactive mode (已有)
- `Ctrl+C`: interrupt running agent (已有)
- `Ctrl+L`: clear chat (已有)
- `Ctrl+G`: open $EDITOR with current composer buffer (已有)

新增（codex parity）：

- `Ctrl+T`: open transcript overlay (new)
- `Ctrl+O`: copy last assistant response as markdown (new)
- `Alt+R`: toggle raw output mode (new)

### Composer keys

- `Enter`: submit now (when not running)
- `Tab`: queue when running; otherwise behave like submit *except* bang shell command (`!`)
- `Esc`:
  - if slash popup active: dismiss popup
  - else: cancel draft / clear input (xylitol 当前没有这个语义，需要新增)

### Slash popup

- Typing `/` as the first non-space character opens the popup.
- Popup supports:
  - query filter (typing characters)
  - `Up/Down` or `Ctrl+P/Ctrl+N` move selection (optional parity)
  - `Tab`: complete selected command into the composer buffer (adds a trailing space)
  - `Enter`: execute selected command immediately
  - `Esc`: close popup without altering input

### Bang shell mode

- Draft starting with `!` is treated as shell mode draft.
- Queue/submit rules:
  - running: `Tab` queues (do not execute immediately)
  - not running: `Tab` does NOT submit (`!` special-case)
  - submit requires `Enter` (or another explicit submit binding)

### Backtrack (minimal)

Codex: when composer is empty, `Esc` can prime backtrack and `Enter` confirms (per tooltips). xylitol 先实现“Esc Esc edit last user message”最小版本：

- If composer is empty and not running:
  - `Esc` primes backtrack hint (optional)
  - second `Esc` loads previous user message into composer for editing

后续可扩展为 overlay 预览选择多个历史 user turn。

## Risks / Non-goals

- 不在本 change 内实现 codex 的多 thread / side conversation / apps / plugins / mcp 管理 UI。
- 不实现 codex 的 paste burst/IME 复杂逻辑；xylitol 先保证主路径与快捷键语义。
- 不承诺与 codex 100% 像素级一致；目标是交互语义一致。
