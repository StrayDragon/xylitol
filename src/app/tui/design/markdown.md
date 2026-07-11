---
version: "alpha"
name: "markdown"
description: "Copy-friendly, token-efficient Markdown — SGR for hierarchy; markers only where round-trip needs them."
tokens_from: "../DESIGN.md"
components:
  md-body:
    textColor: "{colors.assistant}"
  md-h1:
    textColor: "{colors.accent}"
  md-h2:
    textColor: "{colors.accent}"
  md-h3:
    textColor: "{colors.on-surface}"
  md-h4:
    textColor: "{colors.on-surface}"
  md-h5:
    textColor: "{colors.muted}"
  md-h6:
    textColor: "{colors.muted}"
  md-link:
    textColor: "{colors.accent}"
  md-link-url:
    textColor: "{colors.accent}"
  md-code:
    textColor: "{colors.success}"
  md-quote:
    textColor: "{colors.muted}"
  md-hr:
    textColor: "{colors.muted}"
---

# Markdown

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 包：`Markdown` + `MarkdownTheme`（`highlight_code` 回调）。**实现须对齐本节**；未落地前以本文为 SSOT。

助手正文：在终端里**直观、安静**，框选粘贴进下一轮时**省 token**，且尽量仍像可再解析的 Markdown。

## 顶层原则

1. **Copy = 可见字符**。多数粘贴会剥掉 ANSI；占上下文 token 的是字形，不是颜色/粗体/下划线。
2. **层级与强调优先用 SGR**（色 / bold / underline / dim / italic / strikethrough），少加纯装饰字符（`│`、`┌─┐`、全宽 `─`、语言标签条等）。
3. **信息不丢**。终端多半不能内联跳转 → 链接/图片 URL **必须明文**出现在可见文本里。
4. **Round-trip 标记白名单**（用户决议）：为可再编辑/再解析，下列标记**保留在可见文本**中（会占 token，但是有意的）：
   - 行内代码：`` `code` ``
   - 粗体：`**text**`
   - 斜体：`*text*`（或 `_text_`，实现选一种并固定）
   - 删除线：`~~text~~`
5. **禁止用装饰换层级**。标题**MUST NOT**输出 `#` / `##` 前缀；用色组 + bold/underline 表达级别。
6. **fg / bg 分相**：元素着色走 fg；若需消息底色，在行宽 padding 后再套 `bgColor`（`apply_background_to_line`）。

## 标题色组（显示）

| 级 | 色 | 属性 | 可见前缀 |
|---|---|---|---|
| H1 | `{colors.accent}`（`md-h1`） | bold + underline | 无 |
| H2 | `{colors.accent}`（`md-h2`） | bold + underline | 无 |
| H3 | `{colors.on-surface}`（`md-h3`） | bold | 无 |
| H4 | `{colors.on-surface}`（`md-h4`） | bold | 无 |
| H5 | `{colors.muted}`（`md-h5`） | bold 可选 | 无 |
| H6 | `{colors.muted}`（`md-h6`） | 仅色 | 无 |

复制后（剥 ANSI）= 纯标题文字（无 `#`）。层级信息在粘贴进 LLM 时会变弱——接受此权衡以省 token；需要层级时靠段落结构与用词。

## 语法 → 显示 / 复制

### 块级

| 语法 | 显示（终端） | 复制可见字符 | 备注 |
|---|---|---|---|
| 段落 | `{colors.assistant}` / body | 原文 | 段间最多一空行 |
| AT1–H6 | 上表色组 + SGR | 纯标题字 | **无** `#` 前缀 |
| 无序表 | `- ` + 正文 | 同左 | 必要标记；嵌套用空格缩进，**勿**树线 |
| 有序表 | `1. ` … | 同左 | 保留原始序号（`preserve_ordered_list_markers`） |
| 任务列表 | `- [ ]` / `- [x]` | 同左 | 扩展开启时；勿画框 |
| 引用 | 仅 dim + italic（`md-quote`） | 纯引用正文 | **MUST NOT** `│` / 竖线装饰 |
| 代码块 | 语法高亮（SGR）；可选 2 空格缩进 | 纯代码行 | **MUST NOT** fence、语言标签条、行号墙、边框 |
| 表格 | **列宽空格对齐（方案 A）**；表头 bold + underline | 对齐纯文本，**无** `\|`、**无**盒线 | 见下「表格」 |
| 分隔线 | 短 muted 线（约 4–8×`─`）或单空行 | 少数字符或无 | **MUST NOT** 拉满终端宽的装饰线 |
| 图片 | `alt (url)`（无 alt 则用 url） | 同左 | 与链接同：不丢 URL；**勿**只靠不可选 OSC |

### 行内

| 语法 | 显示 | 复制可见字符 |
|---|---|---|
| 粗体 | bold + 可见 `**…**` | `**…**` |
| 斜体 | italic + 可见 `*…*` | `*…*` |
| 删除线 | strikethrough + 可见 `~~…~~` | `~~…~~` |
| 行内代码 | `{colors.success}` + 可见 `` `…` `` | `` `…` `` |
| 链接 | `text (url)`；url 可用 accent + underline | `text (url)` |
| 裸 URL / 自动链接 | 全文展示（可 underline） | 全文 |
| 脚注 | 行内 `[n]`；文末短列表 | 明文 | 后置；勿大框 |
| HTML | 当文本或剥离危险标签 | 可见文本 | 最小处理 |

### 表格（方案 A · 已决议）

- **显示**：按列计算宽度，**空格垫齐**；表头行 bold + underline；表头与正文之间**不要** `│`/`─┼─`/`┌┐` 盒线。
- **复制**：即显示中的可见字符 → 对齐的纯文本表（无 pipe、无盒线）。
- **不选 B**（对齐 GFM `\|` 表）除非产品日后改决议；B 更利 round-trip，但每行多个 `\|` 更胀。

示例（示意，空格对齐）：

```text
Name     Age  Role
alice     30  eng
bob       28  design
```

（终端里 `Name Age Role` 一行为 underline+bold；上表仅示字符。）

## MUST（实现检查清单）

1. 标题：色组 + SGR 分级；**MUST NOT** 输出 `#`/`##` 前缀。
2. 链接 / 图片：**MUST** 渲染为 `text (url)` / `alt (url)`；**MUST NOT** 只留不可选中的 OSC 或丢弃 URL。
3. 代码块：语法高亮即可；**MUST NOT** 边框、`` ``` `` fence、语言标签条、行号墙。
4. 行内代码 / 粗体 / 斜体 / 删除线：显示与复制均**保留**对应 MD 标记（`` ` `` / `**` / `*` / `~~`）。
5. 引用：**MUST NOT** 竖线或盒线装饰；仅用 quote 色 + italic。
6. 表格：方案 A（空格对齐 + 表头 underline）；**MUST NOT** 盒线表；**MUST NOT** 为装饰输出 `\|`。
7. 列表：`- ` / `1. `；嵌套空格缩进；**MUST NOT** `│`/`├`/`└` 树线。
8. HR：短线或空行；**MUST NOT** 近全宽装饰线墙。
9. 高亮库（syntect 等）注入主 crate / demo；**MUST NOT** 打进 `xylitol-tui` 默认依赖（c452）。
10. 段落间最多一空行；fg 内联、bg 延后到行宽 padding。

## 非目标

- 把终端变成浏览器（可点击 OSC 不能替代明文 URL）。
- 为「好看」增加复制后无意义的装饰字符。
- 在包内捆绑语法高亮引擎。

## 实现状态

| 项 | 状态 |
|---|---|
| 本文规范 | SSOT |
| `packages/xylitol-tui` Markdown 组件 | **已按本文收敛**（c530-update-package-tui-markdown） |
| `agent_demo` | seed 全语法 showcase + 流式代码块（源 fence / 显示无围栏） |
| playground Markdown 槽 | 示意对齐本文；非运行时 |
