# Diff block

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

## 行级着色 MUST

| 行类 | Token | 视觉 |
|---|---|---|
| 删除 | `diff-removed` | 红 / 语义色 |
| 添加 | `diff-added` | 绿 / 语义色 |
| 上下文 | `diff-context` | dim / muted |
| 头信息 `---`/`+++`/`@@` | muted | dim |

主题经闭包注入（与 Markdown/SelectList 一致）；包 **MUST NOT** 硬编码产品色板。

## Word-level（行内）MUST

1. 当且仅当出现**恰好一对**相邻 `-` 行与 `+` 行时，对该对做词级（或字级）对比。
2. 变更片段用 **reverse**（或 `word_change` 闭包）高亮。
3. 多行连续增减 **MUST NOT** 做词级对比，只做行级着色。

算法锚点：pi `renderDiff` + `similar`（见 `docs/tui-research/pi.md`）。

## 布局 MUST

1. 默认 **unified**（单栏）。
2. 当终端宽度 ≥ `spacing.diff-side-by-side-min-cols`（默认 **100**）且调用方开启 side-by-side 时，可渲染左右对照；否则 MUST 回退 unified。
3. **MUST NOT** 使用 Unicode 表线 / 树连接符装饰；对齐用空格。
4. 行宽按 `visible_width`（CJK/emoji）；超宽行 MUST 按 ANSI 感知宽度折行或截断策略与 Text/Markdown 一致，**MUST NOT** 按字节硬切。

## 复制友好 MUST

1. 框选复制后仍可读：保留 `+`/`-`/` ` 语义或等价 gutter，不要只剩色块。
2. **MUST NOT** 加语言标签条、双边框墙、装饰性行号墙（gutter 数字可保留，但勿做成「行号墙」视觉）。
3. 空变更：一行 `(no changes)`（或等价 dim）。

## 可展开

产品面可将 Diff 包在 expandable 壳内（默认折叠为一行摘要，如 `edited path (+n -m)`）。展开策略见 [`expandable.md`](./expandable.md)；包组件本身可不处理展开键。

## Out of scope（本设计）

- 旧 `diff-review` 审批流
- 把 syntect 打进 Diff（高亮是 fence 代码块的事，见 [`markdown.md`](./markdown.md) / c452）
