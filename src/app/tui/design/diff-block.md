---
version: "alpha"
name: "diff-block"
description: "Copy-friendly Diff block — unified / side-by-side / edit line format."
tokens_from: "../DESIGN.md"
components:
  diff-added:
    textColor: "{colors.diff-added}"
    backgroundColor: "{colors.diff-added-bg}"
  diff-removed:
    textColor: "{colors.diff-removed}"
    backgroundColor: "{colors.diff-removed-bg}"
  diff-context:
    textColor: "{colors.diff-context}"
  diff-added-word:
    textColor: "{colors.diff-added}"
    backgroundColor: "{colors.diff-added-word-bg}"
  diff-removed-word:
    textColor: "{colors.diff-removed}"
    backgroundColor: "{colors.diff-removed-word-bg}"
  diff-meta:
    textColor: "{colors.muted}"
  diff-gutter:
    textColor: "{colors.muted}"
spacing:
  side-by-side-min-cols: "{spacing.diff-side-by-side-min-cols}"
---

# Diff block

> Token 根源：`{colors.*}` / `{spacing.*}` → [`../DESIGN.md`](../DESIGN.md)（本文件 `tokens_from`）。

包组件：`packages/xylitol-tui` Diff（c451）。产品侧：作为 live scrollback / 工具块积木复用（**非** Codex 式 TranscriptView）。生成侧：`infra` edit 工具的 `display_diff` / unified diff。demo 验证：c453 / c462。

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

| 行类 | Token 引用 | Unified | Side-by-side |
|---|---|---|---|
| 删除 | `{colors.diff-removed}`（+ unified 时 `{colors.diff-removed-bg}`） | 红 fg + **整行淡红底** | **仅红 fg** + gutter；**MUST NOT** 套行底 |
| 添加 | `{colors.diff-added}`（+ unified 时 `{colors.diff-added-bg}`） | 绿 fg + **整行淡绿底** | **仅绿 fg** + gutter；**MUST NOT** 套行底 |
| 上下文 | `{colors.diff-context}` | dim / muted（无行底） | 同左 |
| 头信息 `---`/`+++`/`@@` | `{colors.muted}` | dim | dim |

主题经闭包注入（与 Markdown/SelectList 一致）；包 **MUST NOT** 硬编码产品色板。

**Unified**：行底在 pad 到行宽之后再套（`added_line_bg` / `removed_line_bg`）。
**Side-by-side（c464）**：半栏 **MUST NOT** 默认调用行底闭包——避免与产品 `tool-*-bg`（执行态）抢「红/绿墙」语义；极性只靠 fg + 左右栏。

## 行号 MUST

1. **默认 compact（pi）**：`±{pad}{lineNum} {content}` — 符号与行号同色、内容列对齐；`DiffInput::from_edit_pair` / `EditText` / 默认 `compact_line_numbers: true`。
2. **可选双 gutter**：`compact_line_numbers: false` 时旧/新两列行号（易视觉跳动，仅特殊需要）。
3. **Side-by-side**：左右栏各自 compact 前缀（左=`old_no`，右=`new_no`）；空半栏 MUST NOT 伪造行号。
4. **MUST NOT** 做成装饰性「行号墙」。

Agent Edit 路径：优先 `DiffInput::from_edit_pair(old, new)`（或 `generate_edit_text`），再包进 expandable Diff 块并默认展开预览。

可选内容高亮：`DiffTheme::highlight_line`（默认 identity；`highlight` feature 下可注入 syntect）。

## Word-level（行内）MUST

1. 当且仅当出现**恰好一对**相邻 `-` 行与 `+` 行时，对该对做词级（或字级）对比。
2. 变更片段用 **`word_change_removed` / `word_change_added`**：
   - **嵌在 tool / expandable 洗底内（Edit 默认）**：词底 = 当前块 bg（`tool-*-bg`）向极性色**轻量**混亮——`−` → 红系、`+` → 绿系（包 API：`word_wash_bg(block_bg, polarity)` ≈ `mix(block, lighten(polarity), **0.32**)`）；同行保持 added/removed **fg**；复位到**块 bg**（勿 `49m` 打断洗底）。要更醒目可提到 ~0.42，但默认保持淡洗。
   - **独立 unified 行底路径**（可选）：可用 `diff-*-word-bg` token，复位到行底。
   - **MUST NOT** 默认用 terminal reverse / 白底反色；也 **MUST NOT** 只把块 bg 加深（对比度不足或方向反了）。
3. 多行连续增减 **MUST NOT** 做词级对比，只做行级着色。

算法锚点：词级/行级 diff 用 `similar`；行为参考 pi coding-agent 的 `renderDiff`（源在 `../pi`，勿再维护仓内长篇对标文）。首段变更的行首空白 **SHOULD NOT** 纳入词级高亮。

## 布局 MUST

1. **Edit 工具 / 默认**：**unified compact**（pi `±N content`）。产品 Edit 结果块 MUST 用此形态，**MUST NOT** 默认 side-by-side。
2. Side-by-side：仅当调用方显式设置 `side_by_side_min_width` 且宽度达标时启用；列宽按**内容打包**（cap 半宽）；多行 replace hunk（`similar` 的 DD…II…）MUST **按行 zip** 成 L|R 同行，**MUST NOT** 先堆全部删除再堆全部添加。
3. **MUST NOT** 使用 Unicode 表线 / 树连接符装饰；对齐用空格。
4. 行宽按 `visible_width`（CJK/emoji）；超宽行 MUST 按 ANSI 感知宽度折行或截断策略与 Text/Markdown 一致，**MUST NOT** 按字节硬切。
5. **Edit / demo 一体块**：`tool-*-bg` 洗底包住 **header +（展开时）正文**，上下各一空行 padding（pi `Box(1,1)`）；正文 **不再**叠 `diff-*-bg` 行底。SBS 半栏仍 **MUST NOT** 套行底（c464）。

## 复制友好 MUST

1. 框选复制后仍可读：保留 `+`/`-`/` ` 语义或等价 gutter，不要只剩色块。
2. **MUST NOT** 在 Diff 正文重复 git `--- a/` / `+++ b/` 路径头（路径放在 expandable / Edit 摘要行；工作区内用相对路径，否则绝对路径）。
3. 空变更：一行 `(no changes)`（或等价 dim）。

## 可展开

产品面可将 Diff 包在 expandable 壳内（默认折叠为一行摘要，如 `edited path (+n -m)`）。展开策略见 [`expandable.md`](./expandable.md)；包组件本身可不处理展开键。

## Out of scope（本设计）

- 旧 `diff-review` 审批流
- 把 syntect 打进 Diff 默认路径（可选 `highlight_line` 钩子见 c459）
- 改 `infra` `generate_display_diff` 输出格式
