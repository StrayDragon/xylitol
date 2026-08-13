# diff

复制友好的代码变更。产品把 Edit `display_diff` 嵌在 expandable 工具块里（**非**独立浏览面）。

## 产品行为（与代码核对）

- 默认宽阈值 **100 列**：≥100 → side-by-side；更窄 → unified compact。阈值 ≡ DESIGN `spacing.diff-side-by-side-min-cols`。
- Edit 默认 unified compact；宽屏才 SBS。**MUST NOT** 把 SBS 当 Edit 唯一形态。本稿固定态：`unified`（80 列）与 `side-by-side`（100 列）。
- rail 块内 **MUST NOT** 再叠 `diff-*-bg` 行底。
- 词级：恰好一对相邻 `-/+` 才开；多行 hunk 只做行级。
- 视口：展开后默认末 12 行，`ctrl+o to expand` 满高；过大省略。
- 空变更：`(no changes)`。

## 行级着色

| 行类 | fg | Unified（产品 rail） | Side-by-side |
|---|---|---|---|
| 删除 | `diff-removed` | fg；**不**叠行底 | 仅 fg；半栏 **MUST NOT** 行底 |
| 添加 | `diff-added` | 同上 | 同上 |
| 上下文 | `diff-context` | 无行底 | 无行底 |
| 头信息 | `muted` | 过滤 git `--- a/` 路径头 | dim |

## 行号 / SBS

- compact：`±{pad}{lineNum} {content}`。SBS 左右各自 compact；空半栏 **MUST NOT** 伪造行号、**MUST NOT** 画 `│`。
- SBS 分隔 ` │ `。多行 replace **按行 zip** 成 L|R，禁止先堆完全部删除再堆添加。
- **MUST NOT** Unicode 表线墙。宽按显示宽度（CJK）。

## 词级 / 复制

产品 Edit：surface 向极性轻量混亮，**勿 reverse**。框选后仍可读 `+`/`-` 或等价 gutter。路径放在 expandable 摘要，不在正文重复 `--- a/`。
