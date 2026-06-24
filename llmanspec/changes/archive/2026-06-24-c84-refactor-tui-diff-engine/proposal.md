---
depends_on: [c82-fix-tui-core]
---

# c84-refactor-tui-diff-engine

## Why

c82-fix-tui-core 修复了 pi 式差分渲染引擎的三个 P0 阻塞 bug,使 TUI 可用。但
随后出现的 c83-migrate-tui-to-ratatui 试图把整个渲染层替换成 ratatui 现成组件
+ crossterm alternate screen —— 这个方向是错的,与项目目标(对齐 `../pi` 的裸终端
体验)相悖。

**pi 式架构是正确选择,理由:**

1. **原生文本选择**:pi 不用 alternate screen(`terminal.ts:138` 只开 raw mode),
   用户可以用终端原生选择/复制。ratatui alternate screen 会接管整个屏幕,
   失去这个能力。
2. **scrollback 保留**:不进 alternate screen 意味着 TUI 输出进入终端正常
   scrollback,历史可向上翻滚、可被外部分页器捕获。
3. **差分 ANSI 渲染已被 c82 验证可用**:`engine/` 的 `TuiRenderer` + `diff.rs`
   正是 pi `tui.ts:doRender()` 的 Rust 对应实现(`Vec<String>` 行 + 行级 diff
   + `\x1b[?2026h/l` 同步刷新)。
4. **依赖已就位**:crossterm `event-stream` feature 已在 `Cargo.toml`。差分渲染
   不需要 ratatui 任何组件。

**c83 被废弃**:其 ratatui 重写方向错误,proposal/design/spec/tasks 直接删除
(不进 `not-planning/`,经用户确认)。

**但 c82 引入了一处真实的架构缺陷必须修**:`mod.rs` 启动时立即调用
`agent_loop.run(prompt)`(`prompt=""`),即在用户输入任何消息前就触发一次模型
API 调用。这是 `no-premature-agent` 要修复的点。

## What Changes

### 1. 反转 spec 架构方向(modify 5 条现有 req)

正式 `tui-interface` spec 由 c80 归档时固化了 ratatui + alternate screen 方向。
本变更反转其中强绑 ratatui 的 5 条:

| req | 旧行为(被改) | 新行为 |
|-----|--------------|--------|
| r1 `component-architecture` | Layer B = ratatui widgets | Layer B = `engine/` 差分 ANSI |
| r24 `initial-paint` | "after entering the alternate screen" | "after entering raw mode" |
| r25 `full-frame-render` | "every Terminal::draw frame" | "every draw via Vec<String> diff" |
| r26 `quit-cleanup` | "leave alternate screen" | "no alternate screen used" |
| r45 `p0-text-only` | "ratatui Paragraph" | "ANSI-styled lines" |

### 2. 新增 3 条 req(锚定方向)

- **r47 `no-alternate-screen`**:MUST NOT 用 alternate screen,只 raw mode
- **r48 `no-premature-agent`**:首条消息提交前 MUST NOT 调 `agent_loop.run()`
- **r49 `differential-rendering`**:MUST 用 `Vec<String>` + 行级 diff,非 `Terminal::draw`

### 3. 代码改动(最小)

| 文件 | 改动 |
|------|------|
| `src/interface/tui/mod.rs` | 移除启动时 `agent_loop.run(prompt)`;agent stream 初始为 `None`,首次用户提交才创建(复用 c82 已有的 `pending_prompt` 机制) |
| `src/interface/cli/mod.rs` | `run_tui_engine` 签名去掉 `prompt` 参数;调用点传 `session_id` + `model_name` |
| `engine/`(diff.rs/renderer.rs/ansi.rs/...) | **不动**,c82 实现已满足 r49 |

### 4. 废弃 c83

`llmanspec/changes/c83-migrate-tui-to-ratatui/` 整个删除(不进 `not-planning/`,
经用户确认)。

## Capabilities

- `tui-interface`

## Impact

- **Spec 修改**:`tui-interface` spec r1/r24/r25/r26/r45 语义反转,+ r47/r48/r49
- **代码修改**:`src/interface/tui/mod.rs`、`src/interface/cli/mod.rs`
- **不变**:`engine/*`、`state/*`、`input/*`(纯逻辑层 + 差分渲染已就绪)
- **删除**:`llmanspec/changes/c83-migrate-tui-to-ratatui/`
- **零影响**:agent loop、session、config、tools、BDD 测试
- **TUI 单元测试**(c82 已有的 ~90 条)继续全部通过
