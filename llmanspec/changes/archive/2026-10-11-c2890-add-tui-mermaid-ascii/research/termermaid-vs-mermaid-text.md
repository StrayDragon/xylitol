# termermaid vs mermaid-text（及 grok-build 是否在用前者）

调研问题：两 crate 谁更适合 `xylitol-tui` 的 mermaid 结构块；grok-build 是否依赖 `termermaid`。

结论：**最新 grok-build（`2bdd1d6a`，2026-09-29 origin/main）仍然不用 termermaid**。它有**两套互不替换的引擎**；crates.io `termermaid` / `mermaid-text` 都不在 `Cargo.lock`。对本票 **仍锁 mermaid-text**；termermaid 不是合格 plan B。

## grok-build 用的是谁（2026-10-11 现树复读）

HEAD：`../grok-build` `2bdd1d6a` Synced from monorepo。`Cargo.lock` 无 `termermaid`、无 `mermaid-text`。全树 `rg termermaid` 仍零命中。

现网是 **双栈**，不是 crates.io 港：

| 栈 | crate | 干什么 | 对本票 |
|---|---|---|---|
| **TTY 盒线（inline）** | `xai-grok-markdown` 仓内 `src/mermaid.rs`（**5699 行**，`pub(crate)`） | Markdown `try_push_mermaid` → `Vec<Line>` + `plain_lines`。闭合围栏才画（`range.end < text.len()`，流式开围栏仍显示源） | 算法同源于 termermaid 声称的 port 源；xylitol **不整文件迁**（ratatui `Line`、失败 `sourceBox`） |
| **PNG（点按才渲）** | workspace `xai-grok-mermaid` + vendored Warp `third_party/mermaid-to-svg`（dagre） | pager `mermaid_worker`：子进程 SVG→PNG；**从不 inline**。`render_mermaid=off` 只留盒线；`auto`/`on` 在盒线下加 `[Open Image]` | D11 禁 Kitty/位图。这条比盒线重一个数量级（resvg/fontdb/子进程），本票不做 |

pager 模块头（`mermaid_content.rs` 1–9）：「markdown renderer draws mermaid blocks inline as Unicode box-drawing art」；「The rendered PNG is never drawn inline」。

`mermaid.rs` 自 8 周前 5237 行落地后，4 周前 `48271133` 又 +872/−396（标签 HTML、引号括号等）。测试仍断言 `flowchart LR\n  A[Start] --> B[End]` 解析出标签 `"Start"`/`"End"`，`td_render_has_boxes_labels_and_arrow` 要求输出含 `Start`/`End`。**原件能画方括号标签；crates.io termermaid 0.5 画丢边——移植滞后且回归。**

失败路径未变：`fallback` 标题 ` mermaid: {header} ` 圆角框；`max_width` + `Oversize::{Width,Cells}`。类型链仍是 graph/state/class/er/sequence（无 pie/gantt 盒线；那些走 PNG 栈或框源）。

npm `grok-mermaid` 仍是盒线算法的 TS 港（pi 用）。**三方都不是** `termermaid`。

termermaid README：「Ported from xai-org/grok-build」。血缘 **fork ← 旧 grok 盒线文件**，发布时间 2026-07-20，对不上现树 5699 行。移植还去掉了公开列预算、ratatui 样式，另加 pie/JSON/pyo3。

## 契约对照

| | **mermaid-text 0.57.0** | **termermaid 0.5.0** |
|---|---|---|
| 许可 | MIT（[crates.io](https://crates.io/crates/mermaid-text)） | Apache-2.0 |
| 作者仓 | [leboiko/markdown-reader](https://github.com/leboiko/markdown-reader) `crates/mermaid-text` | [alitrack/termermaid](https://github.com/alitrack/termermaid) |
| 发布 | 2026-04 起，0.57 = 2026-07-21；~69k dl / 14 reverse-deps | 2026-07-20 同日 0.1–0.5；~90 dl / 0 reverse-deps |
| 根 API | `render` / `render_with_options` → `Result<String, Error>` | README 写 `termermaid::render`；**根 `lib.rs` 未 re-export**。真实入口 `termermaid::mermaid::render` → `Option<String>` |
| 硬宽度 | `max_width_strict` → `Error::TooWide { requested, actual }` | 无。内部 `MAX_NODES=128` 等是 grok 切片上限，不是列预算 |
| 失败 | `EmptyInput` / `UnsupportedDiagram` / `ParseError` / `TooWide` | 空输入 `None`；未知类型 **`fallback` 源码框**（`src/mermaid.rs` 107–126，最多 20 行） |
| 依赖 | `ascii-dag` + `chrono` + `unicode-width`（约 7 crate） | 仅 `unicode-width`（可选 `pyo3`） |
| 源码量 | tarball `src/` ~42k 行（含 parser/layout/render） | `src/` ~3.9k 行；`#![allow(dead_code, unused_variables, unused_mut)]` |
| 图类型 | `detect.rs` `DiagramKind`：flowchart / sequence / state / class / er / pie / journey / gantt / timeline / gitGraph / mindmap / quadrant / requirement / sankey / xychart / block / architecture / packet | README：flowchart / sequence / state / class / er（另实现 pie） |

xylitol 已锁的路径（D5–D7）：`Ok` → 盒线结构块；`Err`（含 `TooWide`）→ 普通代码块；**禁止** sourceBox。只有 mermaid-text 的 `Result` 能直接接上。termermaid 的 `Some(框住的垃圾源)` 会被当成「画成功」。

## 同图实测（2026-10-11 `/tmp/mermaid-cmp`，宽 80，`max_width_strict`）

1. **`A[Start] --> B[End]`**：mermaid-text 画出 Start/End 盒。termermaid **只剩孤立 `A` 盒，边和 B 丢掉**。`A[Start Label] --> B[End Label]` 同样。无括号的 `Start --> End` 两边都能画——说明移植在 **方括号标签** 上回归。
2. **带决策的 TD**：mermaid-text 保留 `Start` / `Decision?` / `Do thing` / `Skip`。termermaid 画成节点 id `A B C D`，且出现 `╭─┼─╮` 接头穿盒。
3. **sequence**：两边都可用；termermaid 更窄（19 vs 31 列）。
4. **stateDiagram-v2**：mermaid-text 一行状态机。termermaid 盒边漏 `│`（`┌─┼─┐│`）。
5. **classDiagram**：mermaid-text UML 盒完整。termermaid 框线与文字错位，Dog 几乎空框。
6. **gantt**：mermaid-text 画条。termermaid **sourceBox 原文**。
7. **garbage**：mermaid-text `UnsupportedDiagram('this')`（本票 → 代码块）。termermaid 仍 `Some(┌…┐)`。
8. **宽 12 的 LR 双标签**：mermaid-text `TooWide`（20>12）。termermaid 仍吐半截图（且标签解析已坏）。

grok-build 原件测试断言含标签字符串、超宽走 `mermaid: …` 框。termermaid **没有**把这条契约搬对。

mermaid-text 的代价：Sugiyama TD 竖直空隙偏多（简单 `A-->B` 约 11 行 vs termermaid 7 行）。这是滚动高度，不是正确性。

## 对本票

| 维度 | 胜者 |
|---|---|
| 接 r1949 硬宽度 / 失败回代码 | mermaid-text |
| 标签 flowchart（agent 最常吐的形态） | mermaid-text |
| 图种类 | mermaid-text |
| 许可贴 MIT 仓 | mermaid-text |
| 成熟度 | mermaid-text |
| 依赖重量 / 编译切片 | termermaid（但画错） |
| 圆角 ╭ 观感 / 贴近 grok | grok-build **原件**，不是 termermaid 现网 |

**保持 `mermaid-text = "0.57"`。** 不要换成 termermaid。若盒线观感以后不可接受，plan B 是对照 grok-build `mermaid.rs` **自研/裁剪**（先剥 ratatui、把超宽改成 `None` 而不是 sourceBox），或评估把 grok 算法 vendoring——**不是**再试 crates.io termermaid。npm grok-mermaid 仍不能进本 crate。
