---
version: "alpha"
name: "diff-block"
description: "Copy-friendly Diff block — unified / side-by-side / edit line format."
tokens_from: "../DESIGN.md"
components:
  diff-added:
    textColor: "{colors.diff-added}"
  diff-removed:
    textColor: "{colors.diff-removed}"
  diff-context:
    textColor: "{colors.diff-context}"
  diff-meta:
    textColor: "{colors.muted}"
  diff-gutter:
    textColor: "{colors.muted}"
spacing:
  side-by-side-min-cols: "{spacing.diff-side-by-side-min-cols}"
---

# Diff block

> Token 根源：`{colors.*}` / `{spacing.*}` → [`../DESIGN.md`](../DESIGN.md)（本文件 `tokens_from`）。

包组件：`packages/xylitol-tui` Diff（c451）。产品接线：transcript 内可展开块（c470 / c453）。生成侧：`infra` edit 工具的 `display_diff` / unified diff。

## 目标

复制友好的代码变更呈现：行级 +/-/context + 相邻行对的 word-level；宽屏可切左右对照；CJK/emoji 宽度正确。

## 输入

组件 MUST 至少接受其一：

| 形态 | 说明 |
|---|---|
| `display_diff` 文本 | 与 `generate_display_diff` 对齐的 gutter 行（`NNNN NNNN \| …` / `---/+++`） |
| unified diff 文本 | 标准 `---`/`+++`/`@@`/`+/-/ ` 行 |
| 结构化行对 | `old` + `new`（+ 可选 path）；包内用 `similar` 计算 |
| pi edit 行格式（c459） | `±NNNN content` / ` NNNN content` |

## 行级着色 MUST

| 行类 | Token 引用 | 视觉 |
|---|---|---|
| 删除 | `{colors.diff-removed}` | 红 / 语义色 |
| 添加 | `{colors.diff-added}` | 绿 / 语义色 |
| 上下文 | `{colors.diff-context}` | dim / muted |
| 头信息 `---`/`+++`/`@@` | `{colors.muted}` | dim |

主题经闭包注入（与 Markdown/SelectList 一致）；包 **MUST NOT** 硬编码产品色板。

## 行号 MUST

1. **默认 compact（pi）**：`±{pad}{lineNum} {content}` — 符号与行号同色、内容列对齐；`DiffInput::from_edit_pair` / `EditText` / 默认 `compact_line_numbers: true`。
2. **可选双 gutter**：`compact_line_numbers: false` 时旧/新两列行号（易视觉跳动，仅特殊需要）。
3. **Side-by-side**：左右栏各自 compact 前缀（左=`old_no`，右=`new_no`）；空半栏 MUST NOT 伪造行号。
4. **MUST NOT** 做成装饰性「行号墙」。

Agent Edit 路径：优先 `DiffInput::from_edit_pair(old, new)`（或 `generate_edit_text`），再包进 expandable Diff 块并默认展开预览。

可选内容高亮：`DiffTheme::highlight_line`（默认 identity；`highlight` feature 下可注入 syntect）。

## Word-level（行内）MUST

1. 当且仅当出现**恰好一对**相邻 `-` 行与 `+` 行时，对该对做词级（或字级）对比。
2. 变更片段用 **reverse**（或 `word_change` 闭包）高亮。
3. 多行连续增减 **MUST NOT** 做词级对比，只做行级着色。

算法锚点：pi `renderDiff` + `similar`（见 `docs/tui-research/pi.md`）。

## 布局 MUST

1. **Edit 工具 / 默认**：**unified compact**（pi `±N content`）。产品 Edit 结果块 MUST 用此形态，**MUST NOT** 默认 side-by-side。
2. Side-by-side：仅当调用方显式设置 `side_by_side_min_width` 且宽度达标时启用；列宽按**内容打包**（cap 半宽）；多行 replace hunk（`similar` 的 DD…II…）MUST **按行 zip** 成 L|R 同行，**MUST NOT** 先堆全部删除再堆全部添加。
3. **MUST NOT** 使用 Unicode 表线 / 树连接符装饰；对齐用空格。
4. 行宽按 `visible_width`（CJK/emoji）；超宽行 MUST 按 ANSI 感知宽度折行或截断策略与 Text/Markdown 一致，**MUST NOT** 按字节硬切。
5. 与 pi 整块 bg tint 的视觉对齐见 c462；**落地约束**：expandable 壳的 `tool-*-bg` 只铺 Diff **header**，正文见上表行级着色（见 [`expandable.md`](./expandable.md) §6）。

## 复制友好 MUST

1. 框选复制后仍可读：保留 `+`/`-`/` ` 语义或等价 gutter，不要只剩色块。
2. **MUST NOT** 加语言标签条、双边框墙、装饰性行号墙（gutter 数字可保留，但勿做成「行号墙」视觉）。
3. 空变更：一行 `(no changes)`（或等价 dim）。

## 可展开

产品面可将 Diff 包在 expandable 壳内（默认折叠为一行摘要，如 `edited path (+n -m)`）。展开策略见 [`expandable.md`](./expandable.md)；包组件本身可不处理展开键。

## Out of scope（本设计）

- 旧 `diff-review` 审批流
- 把 syntect 打进 Diff 默认路径（可选 `highlight_line` 钩子见 c459）
- 改 `infra` `generate_display_diff` 输出格式
