# c366 Design — ThinkingText markdown 接入与引用前缀

> 范围小、决策点少，本文件只记两个关键决策。

## 决策 1：ThinkingText 用独立 `for_thinking` 样式（而非复用 for_assistant）

`MarkdownStyle::for_thinking` 以 `p.thinking()`（dim italic）为 text 基色。理由：thinking 是 reasoning 模型的「内心独白」，视觉层级必须低于 assistant 正文（用户聚焦回复，不聚焦思考）。若复用 `for_assistant`（正常亮度），thinking 会与正文抢眼，破坏 c365「thinking 是次要内容」的层级约定。

结构样式（heading/code/list/quote）保留 markdown 解析——reasoning 内容常含「分析步骤」列表、「假设」代码块，平铺成裸字符串损失信息。但整体偏 dim：heading 不加 BOLD（for_assistant 加了），code_inline 用更暗的色调。

## 决策 2：引用前缀用 `▎`（而非 `>` 或 `│`）

| 字符 | 来源 | 取舍 |
|---|---|---|
| `>` | CommonMark 语法 / pi | 与引用语法符号重复，视觉上像「未渲染的源码」 |
| `│` | codex | box-drawing 全角宽度，CJK 终端占 2 列，宽度计算易错 |
| `▎` | kimi-code 同族（▏） | U+258E 左竖线，display width = 1，视觉轻 |

**选 `▎`**：display width 1（与现有 CJK 宽度计算兼容），视觉轻（不喧宾夺主），不与任何 markdown 语法符号重复。

### 嵌套引用的前缀

`quote_depth: usize` 在 `Start(Tag::BlockQuote)` 递增。前缀 = `"▎ ".repeat(depth)`——二级引用 `▎ ▎ `，三级 `▎ ▎ ▎ `。续行（wrap 产生的物理行）也带前缀，所以在 `flush_inline` 产出每行时统一加。

## 不在本变更范围

- 代码语法高亮——独立变更（库选型待定：ratatui-markdown fork vs syntect）
- 流式 markdown——独立后续
