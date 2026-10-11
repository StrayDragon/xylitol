# mermaid 终端渲染对照

## xylitol

- `components/markdown.rs`：围栏 → `render_code_block_lines` → syntect。无 mermaid 分支。
- D11：无 Image 组件；保留 `is_image_line` + OSC 8 hyperlink。

## 对照

| 产品 | 引擎 | 输出 |
|---|---|---|
| pi 1.1.0 | npm `grok-mermaid`，Markdown transformer | Unicode art；超宽则留源；`off\|final\|streaming` |
| opencode TUI | `@opencode/merman` | 终端网格 / box-drawing；支持 flowchart/gantt/sequence/state/timeline 子集 |
| opencode web | mermaid.js → SVG | 与 TTY 无关 |
| crush | 无 mermaid；Kitty/ANSI 只管文件图 | — |
| ratatui-markdown | pest flowchart + 手写 sequence/pie/gantt；`Vec<Line>` 盒线 | **SySL-1.0**，禁止依赖；无图内 pan |

推荐（已改锁，见 [mermaid-crates.md](./mermaid-crates.md)）：Markdown 短路 + **`mermaid-text` 0.57**；失败回源。不搬 grok / merman 源码。pan 另票 + c2858。

## 2026-10-11 一手补记（propose 锁定）

本地路径：

| 源 | 许可 | 体量 | 对本票 |
|---|---|---|---|
| `../pi-cursor-sdk/node_modules/grok-mermaid` | Apache-2.0 | parse+layout ≈ 2k 行 TS | 引擎自定宽、`null`=空白/语法/未知类型/过大。**pi 调用方**比 `art.width`，失败回 `token.raw`，**不用**库里的 `sourceBox`。 |
| `../grok-build` HEAD `2bdd1d6a` `xai-grok-markdown/src/mermaid.rs` | Apache-2.0 | **5699** 行盒线；另有 `xai-grok-mermaid` PNG（Warp dagre SVG，点按才渲、从不 inline） | **不用 termermaid**。盒线原件仍 `sourceBox`+ratatui，不整文件迁。详见 [termermaid-vs-mermaid-text.md](./termermaid-vs-mermaid-text.md) |
| `../opencode/packages/merman` | 未核进本仓 | flowchart/sequence/gantt/… 分目录 | 对照 detect 分派；有图内拖，本票不做 |
| `../ratatui-markdown/src/mermaid` | SySL-1.0 | pest + 手写 | **禁依赖 / 禁搬 grammar** |

锁定（回写 proposal Q1–Q5）：

1. **默认 `final`**。grok-mermaid 对流式半句会「丢最后一行再画」——xylitol `final` **禁止**这条，否则半图提前出、高度抖（c2865 D6）。
2. **`mermaid-text` 0.57**（见 [mermaid-crates.md](./mermaid-crates.md)）。不自研 layout，不 vendoring grok。
3. 失败 = **普通代码块**，不是 `sourceBox`（撞 r1645 / r1648）。
4. thinking 不经 Markdown，不画。
5. 产品不另开配置。

围栏是否闭合：与 c2865 同一套 opener/closer 扫描；pulldown 在 EOF 也会 `End(CodeBlock)`，不能当「已闭合」。
