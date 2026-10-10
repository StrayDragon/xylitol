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

推荐：Markdown 主题钩子 + 子集 ASCII；失败回源。算法可看分层+盒线，不要搬源码。pan 另票 + c2858。
