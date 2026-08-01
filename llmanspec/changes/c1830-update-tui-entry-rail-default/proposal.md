---
depends_on: []
branch: sdd/c1830-update-tui-entry-rail-default
base_sha: 5c66e84351b0ae6ff42f84e608143685b43ebbc9
checkpointed: false
---

# UiEntry rail 默认：边轨皮肤取代洗底

> Prototype（`uientry-remaster.html` + `agent_demo` `entry:rail`）已拍板观感：左侧状态色轨 + 1 格 gutter + 内容；条目间 ≥1 空行；User/Assistant flush。
> 本变更为 **产品默认**；本轮 Diff **只解绑**「整块 tool-*-bg 洗底包住 header+正文」；更炫的行级 diff 皮肤另案。

## Why

现行合约（`att4`/`att9`/`att10`/`att14`/`atc8`）把 pi 式 **全行蓝/绿/红洗底**钉成 MUST。扫读噪音大、复制易夹装饰，且 Diff 被三种 `tool-*-bg` 一体块绑架，难以独立进化。

rail 在 demo 已验证更干净、仍可辨成败；应对齐为 xylitol TUI **默认**主条目皮肤。

## What Changes

- **产品默认**：`render_scrollback` 以 rail 绘制 Thinking / Tool / Diff / Bash（及同类可展开块）：`[1-cell status rail][1 plain gutter][content]`；User / Assistant **无**轨、无 `user-message-bg` 全行淡底
- **成败语义**：pending/success/error → 轨色（对齐 DESIGN status tokens，可 soft-mix）；**MUST NOT** 默认整行铺 `tool-*-bg`
- **块间隙**：相邻条目仍 ≥1 行 untinted 空行（保留 `att10` 间隙半句；删除「整行洗底 / padding_y 空 tint」半句）
- **Diff 本轮**：header + Diff 正文 **不再**同属 tool-*-bg 洗底块；正文仍 **MUST NOT** 叠 `diff-*-bg` 行底（极性靠 fg + `word_wash_bg`，现状可保持）；**不做**新的行级 rail-in-diff 视觉大改
- **包原语**：`packages/xylitol-tui` 提供可复用 `paint_left_rail_line`（或等价）——轨色 + gutter + 内容宽；产品 MUST 调用包 API，MUST NOT 手写第二套
- **合约改写**：`app-tui-transcript`（att4/5/9/10/11/14 等）、`app-tui-chrome`（atc8）、必要时 `package-tui-theme`（轨绘制 helper）
- **设计文**：`design/transcript.md` / `expandable.md` / `DESIGN.md` 相关段落与 remaster 对齐；roadmap `TUI重制.md` M1 勾为进行中/兑现路径

## Capabilities

- `app-tui-transcript`（主）
- `app-tui-chrome`（atc8 退役/改写）
- `package-tui-theme`（rail paint helper；若现 capability 过窄可扩 statement）

## 测试缝（apply 用）

| 缝 | 断言 |
|---|---|
| 产品 `render_scrollback`（现有 transcript/chrome BDD + 单测） | 成功 tool/bang：**有** status 轨 ANSI、**无**整行 `tool-success-bg` 洗底 |
| 同缝 · user | User 行 **无** `user-message-bg` 全行淡底；`❯`（或 glyph）语义前缀保留 |
| 同缝 · gap | 相邻块之间 ≥1 untinted 空行 |
| 同缝 · Diff | edit 成功块：header+diff **不**共享 tool-*-bg；仍无 `diff-*-bg` 行底分层 |
| 包 `paint_left_rail_line` 单测 | 宽预算下轨 1 + gutter 1 + 内容；窄宽不 panic |

## Impact

- 视觉默认与 pi wash 分叉；键位 / fold / Ctrl+O / 摘要行格式 **不变**
- wash 相关 BDD 场景（`edit-unified-tint`、`full-width-tint`、`atc8` user-message-bg）须改写或删除
- `agent_demo` 可继续保留 `/entry-style wash` 作对照；产品 **无** 运行时皮肤开关（本 change 不做 dual-skin 配置）

## Out of scope

- 更炫 Diff（行级轨、仅增删侧着色条、side-by-side 大改）— **另开 change**
- 语义复制一等出口（roadmap M2）
- 整壳 chrome / footer / 树 / slash 重制
- 默认开启 wash 兼容开关或「经典 pi 模式」
