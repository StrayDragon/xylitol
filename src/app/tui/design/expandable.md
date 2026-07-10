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

对齐 pi interactive：折叠一行摘要，展开完整内容。实现落在**应用面** transcript 子块，不是包内通用 Chat 组件。

## MUST

1. 默认 **详略得当**：折叠时一行（或短摘要）；展开后显示完整 thinking / 工具输出。
2. 折叠态仍须复制友好：摘要行本身可读，**MUST NOT** 只有图标。
3. 快捷键切换（thinking toggle、tools expand）由产品面绑定；包组件只渲染给定展开态。
4. 折叠行旁 MUST 提示对应快捷键，格式为括号包裹的完整和弦（demo 榜样：`thinking  (Ctrl+T)`、`tool/diff  (Alt+E)`），避免 `^T` 缩写与无括号裸键；勿只靠 footer 快捷键墙。
5. **工具块背景三态**（吸取 pi `ToolExecutionComponent`）：pending → `{colors.tool-pending-bg}`；成功 → `{colors.tool-success-bg}`；失败 → `{colors.tool-error-bg}`。全行宽 padding 后套 bg（`apply_background_to_line`），ANSI 只重置背景（`\x1b[49m`），勿冲掉内容 fg。

## 已验证（c462 / agent_demo）

- seed：success Diff/Tool + error Tool 均带 truecolor 全行 tint。
- 脚本：Tool/Edit 先 `Pending` 再翻 `Success`（摘要 `· running` → `· ok`）。
- harness：视口断言 `48;2;…` 与 `49m`（见 `agent_demo_test`）。

## 策略（可后续细化）

默认折叠哪些、是否记住、是否全局一键展开工具 → 接线后可再议；本文件锁定「需要可展开」+「块旁键位提示」+「工具 bg 三态」。

Diff 块也可套同一展开壳（见 [`diff-block.md`](./diff-block.md)）；展开策略与 tool 块可共用状态机（c453）。
