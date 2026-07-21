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
3. 工具主视线形态：`<ToolName> <path>[:range]`（例 `Read src/main.rs:42-80`）；ToolName **首字母大写**，加粗 + accent；path 用 on-surface；**行范围后缀**用 `{colors.skill-ref}`（mauve，勿用 warning 黄）。cwd 内相对路径，之外绝对路径。输入侧（bash `$ cmd` / 路径）**MUST NOT** 按固定字符数截断；终端宽不够则折行留痕。**MUST NOT** 使用 `⚙` 等装饰 glyph。
4. **默认展开**：`tools_expanded` 默认 true（工具/Edit/Diff 正文默认可见）；**Alt+E** 在 hide ↔ unhide 间切换（Edit 的 `display_diff` **MUST** 服从同一开关，不得常驻展开）。
5. **Viewport**：正文展开后仍有高度上限（tool/bash/write/diff）；**Ctrl+O** 切满高。与 Alt+E 正交。
6. 快捷键切换（thinking toggle、tools expand）由产品面绑定；包组件只渲染给定展开态。
7. 折叠行旁 MUST 提示对应快捷键，格式为括号包裹的完整和弦（`thinking  (Ctrl+T)`、`Read path  (Alt+E)`）。
8. **工具块背景三态**：pending → `{colors.tool-pending-bg}`；成功 → `{colors.tool-success-bg}`；失败 → `{colors.tool-error-bg}`。
9. **Edit / Diff 一体块**：`tool-*-bg` 洗底包住 header +（展开时）正文；正文 **MUST NOT** 再叠 `diff-*-bg` 行底。
10. **Tool 详情不重复命令**：摘要行已含命令时，展开详情 MUST NOT 再 echo 同一命令。
11. **Bash / Read 工具结果**：bash 呈现 shell 式 stdout/stderr；read 呈现 `content` 正文（可附 truncation hint），**MUST NOT** 默认甩 `{"content":…,"total_lines":…}` / `exit_code` 等 machine JSON。write/edit 成功 JSON 静默（diff 走 `display_diff` 块）。
12. **详情视口（max-height）**：块已展开（Alt+E）后，长输出仍有第二层折叠——默认只保留末尾 N 行；**Ctrl+O** 全文。硬截断时 MUST NOT 展开全文。

色板：tool-name≈accent、tool-path≈on-surface、range≈warning；playground 可继续试色。

## 已验证（c453 / c462 / c466 · agent_demo）

- **c453**：Thinking / Tool / Diff 共用折叠；Ctrl+T / Alt+E；流式 thinking 展开再折叠；块旁 `(Ctrl+T)` / `(Alt+E)`。
- **c462**：seed Tool 全块 tint；Edit/Diff **一体块** tint（header+正文）+ word_wash；Pending→Success；harness RGB。
- **c466**：长 bash + Ctrl+O 视口；与 Alt+E 正交。

## 策略（可后续细化）

默认折叠哪些、是否记住、是否全局一键展开工具 → 接线后可再议；本文件锁定「需要可展开」+「块旁键位提示」+「工具 bg 三态」+「详情 max-height 视口」。

Diff 块也可套同一展开壳（见 [`diff-block.md`](./diff-block.md)）；展开策略与 tool 块共用状态机（**c453 已归档**）。
