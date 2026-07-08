---
change_id: c400-fix-tui-layout-viewport
---

# c400 Design — Viewport Anchoring Fix

## D1: 问题根因（视口锚定公式不一致）

c399 对齐 pi 的 line-array 架构，但 `previous_viewport_top` 锚定逻辑不一致：

|| full_render | diff_render |
|---|---|---|
| anchor 公式 | `n.saturating_sub(height)` | floor 纠偏（`final_cursor.saturating_sub(height-1)`） |
| working-area | 无 padding | 无 padding |
| 依赖 | 自足 | 依赖上一帧的 `previous_viewport_top` + scroll block 增量 + floor 下界 |

pi 的做法（`tui.ts:1318-1319`）：
```ts
const bufferLength = Math.max(height, newLines.length);
this.previousViewportTop = Math.max(0, bufferLength - height);
```

关键差异：pi 始终以 `max(height, n)` 为逻辑总行数后取视口顶——**短内容时 viewport_top=0，长内容时 viewport_top=n-height，无中间态、无增量依赖**。`diff_render` 的 scroll block 在帧内 handle 视觉效果，保存状态时不带增量依赖。

**xylitol 问题**：
1. `full_render` 用 `n - height`（不含 bufferLength padding），短内容时可能出错（`n < height` 时 `saturating_sub` = 0，尚可；但 resize 后 `n >= height` 转 `n < height` 时 viewport 会短暂锚定到非零值再被下方 diff 纠正）。
2. `diff_render` 依赖 floor 纠偏——floor 使用 `final_cursor`（局部变量，可能不等于 `n_new-1`，比如仅末尾行变更的帧 `final_cursor = n_new-1` 是对的，但中间行变更的帧 `final_cursor = last` 且 `last < n_new-1`）。这种增量依赖在连续多帧快速追加时可能累积偏差：某帧视口落后→下一帧 scroll block 基于错误基准→偏离扩大。
3. **缺少 `bufferLength = max(height, n)` padding**：pi 通过 this padding 确保即使内容短于终端，viewport_top 也基于「终端高度」而非「内容长度」计算，避免从短内容→长内容过渡时 viewport 短暂脱节。

## D2: 修复策略（集中锚定，取消增量公式）

### 核心变更

删除 `diff_render` 和 `full_render` 各自设置的 `previous_viewport_top` 路径。在 `do_render` 末尾，**统一**锚定为一个集中点：

```rust
// do_render 末尾 save-state（替代各分支分散设置）
let buffer_len = height.max(n_new);
self.previous_viewport_top = buffer_len.saturating_sub(height);
```

同时保留 `diff_render` 的 scroll block（负责帧内光标移动和滚动——这是视觉行为，不是锚定状态），但删除末尾的 floor 纠偏和 `previous_viewport_top += scroll` 语句。

### 为什么安全

- **diff 仍正确**：scroll block 的 `prev_viewport_top += scroll` 是帧内局部变量（`prev_viewport_top`），用于计算 `move_to_row` 的屏幕坐标——删除后改用当前帧的 `self.previous_viewport_top` 即可。
- **floor 不再需要**：floor 纠偏是因分散锚定导致的防御代码——集中锚定后自然消除需要纠偏的偏差来源。
- **与 pi 100% 对齐**：此设计使 `previous_viewport_top` 的语义与 pi 完全一致——视口顶 = `max(0, bufferLength - height)`，每帧重置，无跨帧增量。

### diff_render 内部调整

| 项目 | 旧 | 新 |
|---|---|---|
| `prev_viewport_top` 局部 | 来自 `self.previous_viewport_top`，scroll block 修改 `self.previous_viewport_top += scroll` | 来自 `self.previous_viewport_top`，**不修改** `self` 的字段，只在局部使用 |
| 末尾 floor 纠偏 | `let floor = final_cursor.saturating_sub(height-1)` | **删除** |
| 末尾 `max_lines_rendered` | 保留 | 保留（统计高水位，用于 clearOnShrink） |

### full_render 内部调整

删除 `self.previous_viewport_top = n.saturating_sub(height)` 一行。

## D3: Tests — scrollback-aware VirtualTerminal

### 缺口

现有 `CapturingTerminal` 只记 ANSI 字节流，不模拟终端滚动行为（`\r\n` 推旧行出视口）。因此 diff 输出字节正确但「终端视觉效果」未验证。这是三轮手动验证都漏掉 U1 的根本原因。

### 设计

在 `src/app/tui/engine/virtual_terminal.rs` 新增 `ScrollbackTerminal`：

```rust
/// A terminal emulator that simulates scrollback: `\r\n` at the bottom row
/// pushes the top row into a scrollback buffer. Useful for verifying that
/// viewport anchoring keeps the footer (input/loader) visible.
pub(crate) struct ScrollbackTerminal {
    /// Rows currently shown on screen (max `height`). Index 0 = top of screen.
    pub screen: Vec<Vec<Cell>>,
    /// Rows pushed out of the screen by \r\n scrolling.
    pub scrollback: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub width: u16,
    pub height: u16,
}
```

提供方法：
- `write(&str)`：解析 ANSI，处理 `\r\n` 时的滚动（往 scrollback push 顶部行）
- `screen_text(row) -> String`：提取纯文本
- `screen_contains(&str, row) -> bool`：某行含特定文本
- `footer_visible(line_count, padding) -> bool`：验证最后 `line_count` 行在屏幕底部可见

### 新增集成测试场景

```gherkin
Scenario: footer stays visible as content grows past terminal height
  Given a terminal height of 10 rows and an empty screen
  When the engine renders frames with content growing from 5 to 15 rows
  Then the last 2 rows (loader + input) remain in the last 2 screen positions

Scenario: viewport anchors to tail during streaming
  Given a terminal height of 10 rows and content at 8 rows
  When 5 TextDelta frames append 1 row each
  Then each frame shows the newest content at the bottom of the screen

Scenario: fullRender after shrink preserves viewport anchoring
  Given content at 20 rows shrinks to 5 rows (clearOnShrink)
  When a full redraw occurs
  Then the viewport shows the footer at the bottom without stale orphan rows
```

## D4: 不变式

- **C1**: 每帧 `do_render` 末尾，`previous_viewport_top = max(0, max(height, n_new) - height)`
- **C2**: `diff_render`/`full_render` 只负责输出，不修改 `self.previous_viewport_top`
- **C3**: scrollback buffer 测试必须通过——内容超终端高度后，输入区与 loader 在屏幕底行位置
- **C4**: 现有 780 测试中的 engine 相关测试（diff append, shrink, clearOnShrink, viewport floor 等）全部保留并通过（必要时调断言适配新锚定逻辑）

## D5: 范围外

- **不改 `Container::render`**：扁平 line-array 是正确的（同 pi），无需引入区域划分
- **不加 `ScrollbackTerminal` 到生产路径**：测试 oracle only
- **不改 widget 层**：transcript/input/loader/widgets 无变更
- **不改 host 侧 `mod.rs`**：组件组装顺序不变

## D6: 风险评估

| 风险 | 可能性 | 缓解 |
|---|---|---|
| 集中锚定破坏 resize 后视口 | 低 | pi 同样在 fullRender 中设 `previousViewportTop = max(0, bufferLength - height)`，resize 触发的 fullRender 正常 |
| diff 帧内的 scroll block 行为变化 | 低 | scroll block 只是 `\x1b[B` + `\r\n` 序列，属于帧内输出行为，不受 `previous_viewport_top` 状态语义影响 |
| 现有 diff 相关单测（780 测中 ~20 个 engine 测）因锚定变化失败 | 中 | 逐个审查，适配断言——预期锚定变严（floor 被统一公式替代），多数测试应更稳定而非更脆弱 |
