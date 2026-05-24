---
depends_on: [c90-update-markdown-rendering]
---

# c101-integrate-leaf-markdown-renderer

## Why

c90-update-markdown-rendering 调研了终端 Markdown 渲染方案，将 [RivoLink/leaf](https://github.com/RivoLink/leaf) 列为候选但 defer 了集成。现在 fork ([StrayDragon/leaf](https://github.com/StrayDragon/leaf) `feat/expose-lib-api` 分支) 已完成 library API 暴露，可以作为依赖直接引用。

当前 xylitol 的 `MarkdownRenderer`（`src/interface/tui/markdown.rs`，~460 行）是对 `pulldown-cmark` + `syntect` 的简单包装，缺少：

- 宽度感知的行折叠（CJK/emoji 安全）
- 表格布局（对齐、列宽计算）
- LaTeX / Mermaid 渲染
- YAML frontmatter 支持
- 主题系统（内置预设 + TOML 自定义）
- GitHub Alert callouts（Note / Tip / Warning / Caution）
- `==mark==` 高亮、task list 样式
- TOC 提取和链接检测

leaf 的渲染管线经过 227 个测试验证，覆盖上述所有场景。

此外，`leaf-core` 新增了 `StreamingRenderer`，专为 LLM token 流式输出设计：
- 防抖全量重解析（可配置间隔，默认 150ms）
- 行级增量 diff（只报告变化区域，减少 TUI 重绘）
- 双阶段渲染（已完成段落精确 Markdown + 当前行近似 ANSI）

## What Changes

1. **添加 `leaf-core` 为本地 path 依赖**（`leaf-core = { path = "../leaf/crates/leaf-core" }`）
   - 不直接依赖 `leaf` crate，通过 `leaf-core` 稳定 facade 隔离上游变更
2. **重构 `MarkdownRenderer`**：内部委托给 `leaf_core::MarkdownRenderer`，保持现有 `pub(crate)` 接口不变
3. **移除冗余实现**：删除 `src/interface/tui/markdown.rs` 中的自研渲染循环（~400 行），替换为 leaf-core 调用
4. **更新 `markdown-rendering` spec**：反映使用 leaf-core 库的新需求和场景
5. **更新快照测试**：leaf 的渲染输出与自研渲染不同（更丰富的样式），需要更新 insta 快照

## Capabilities

- `markdown-rendering`: 升级为基于 leaf 的终端 Markdown 渲染

## Impact

- **输出样式变化**：渲染结果将更加丰富（表格有边框、heading 有下划线、代码块有语言标签等），快照测试需要更新
- **新增依赖**：`leaf` 库引入 `ratatui` 0.30（与 xylitol 现有 0.29 并存）、`mmdflux`、`unicodeit` 等
- **体积影响**：新增约 5-8 个 transitive dependencies（均 MIT/Apache-2.0 许可）
- **ratatui 版本差异**：leaf-core re-exports ratatui 0.30 的 `Line`/`Span`，xylitol 自身使用 0.29；需在 `MarkdownRenderer` 边界统一版本或做转换
- **维护成本**：`leaf-core` 作为稳定 facade，上游 leaf 变更不直接影响 xylitol，仅需更新 `leaf-core` 适配层
- **兼容性**：`MarkdownRenderer` 的 `pub(crate)` 接口保持不变，对调用方透明
