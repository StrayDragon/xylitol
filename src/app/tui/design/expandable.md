---
version: "alpha"
name: "expandable"
description: "Collapsible thinking / tool / diff blocks with parenthesized key hints."
tokens_from: "../DESIGN.md"
components:
  thinking-header:
    textColor: "{colors.muted}"
  tool-header:
    textColor: "{colors.tool}"
  tool-pending:
    backgroundColor: "{colors.tool-pending-bg}"
  tool-success:
    backgroundColor: "{colors.tool-success-bg}"
  tool-error:
    backgroundColor: "{colors.tool-error-bg}"
  key-hint:
    textColor: "{colors.muted}"
---

# Expandable（thinking / tool）

> Token 根源：`{colors.*}` / `{components.*}` → [`../DESIGN.md`](../DESIGN.md)。

对齐 pi interactive：折叠一行摘要，展开完整内容。实现落在**应用面**（demo 优先）；**不是**包内通用 Chat 组件，也**不是** Codex 式 TranscriptView 的一部分。

## MUST

1. 默认 **详略得当**：折叠时一行（或短摘要）；展开后显示完整 thinking / 工具输出。
2. 折叠态仍须复制友好：摘要行本身可读，**MUST NOT** 只有图标。
3. 快捷键切换（thinking toggle、tools expand）由产品面绑定；包组件只渲染给定展开态。
4. 折叠行旁 MUST 提示对应快捷键，格式为括号包裹的完整和弦（demo 榜样：`thinking  (Ctrl+T)`、`tool/diff  (Alt+E)`），避免 `^T` 缩写与无括号裸键；勿只靠 footer 快捷键墙。
5. **工具块背景三态**（吸取 pi `ToolExecutionComponent`）：pending → `{colors.tool-pending-bg}`；成功 → `{colors.tool-success-bg}`；失败 → `{colors.tool-error-bg}`。全行宽 padding 后套 bg（`apply_background_to_line`），ANSI 只重置背景（`\x1b[49m`），勿冲掉内容 fg。
6. **Diff 块例外**：状态 tint **只铺摘要/header 行**；展开后的 Diff 正文 MUST 只用 Diff 自身的 added/removed/context fg（及 word-level），**MUST NOT** 再套 `tool-*-bg`（否则红/绿行与块级绿底打架）。
7. **Tool 详情不重复命令**：摘要行已含命令时，展开详情 MUST NOT 再 echo 同一命令（可只留 exit / stderr / 预览）。
8. **详情视口（max-height）**：块已展开（Alt+E）后，长输出仍有第二层折叠——默认只保留末尾 N 行视觉行（wrap-aware），**块尾**插 dim 提示 `... (N earlier lines, ctrl+o to expand)`（MUST NOT 插在工具头行下一行）；**Ctrl+O** 全局切换「视口 / 全文」。流式时折叠态贴尾（新行进尾、earlier 计数涨）。**硬截断**（含 `[Full output:`）时 MUST NOT 展开全文，hint 改为 expand disabled。write 正文默认亦为 Tail（跟流）；仍允许 Ctrl+O。包实现：`ExpandableOutput` / `render_expandable_output`（`packages/xylitol-tui`）；与 Alt+E「块有无详情」正交。

## 已验证（c453 / c462 / c466 · agent_demo）

- **c453**：Thinking / Tool / Diff 共用折叠；Ctrl+T / Alt+E；流式 thinking 展开再折叠；块旁 `(Ctrl+T)` / `(Alt+E)`。
- **c462**：seed Tool 全块 tint；Diff **仅 header** tint + 正文 Diff 配色；Pending→Success；harness RGB。
- **c466**：长 bash + Ctrl+O 视口；与 Alt+E 正交。

## 策略（可后续细化）

默认折叠哪些、是否记住、是否全局一键展开工具 → 接线后可再议；本文件锁定「需要可展开」+「块旁键位提示」+「工具 bg 三态」+「详情 max-height 视口」。

Diff 块也可套同一展开壳（见 [`diff-block.md`](./diff-block.md)）；展开策略与 tool 块共用状态机（**c453 已归档**）。
