# c395 — Tasks

## 阶段 1：搬出复用文件 + 依赖

- [ ] 1.1 把 vendor/ratatui_markdown/highlight/syntect_bridge.rs 搬到 components/syntect_bridge.rs，删 trait 抽象（inherent 方法）。
- [ ] 1.2 把 vendor/ratatui_markdown/highlight/segment.rs 搬到 components/highlight_segment.rs。
- [ ] 1.3 Cargo.toml 加 pulldown-cmark = "0.10"（tui-gated）。

## 阶段 2：自研 markdown_render.rs

- [ ] 2.1 新增 components/markdown_render.rs：Writer 状态机（pulldown-cmark Event → Vec<Line>）。覆盖标题/粗体/斜体/code/列表/引用/链接/表格。
- [ ] 2.2 代码块经 SyntectHighlighter + segments_to_lines 高亮（无框无标签）。

## 阶段 3：换 adapter + 删 vendor

- [ ] 3.1 components/markdown.rs：render_markdown 改调自研 Writer（签名不变）。MarkdownStyle 简化（3-4 个 Color）。
- [ ] 3.2 删 src/app/tui/vendor/ 整个目录。
- [ ] 3.3 改 2 个测试断言（blockquote 无 │→ italic；nested 缩进）。

## 阶段 4：校验 + 归档

- [ ] 4.1 just qa（all-features）+ arch_guard + strict validate。
- [ ] 4.2 归档 c395。
