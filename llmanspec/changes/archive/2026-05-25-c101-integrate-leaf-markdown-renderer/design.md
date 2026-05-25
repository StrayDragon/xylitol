# c101-integrate-leaf-markdown-renderer Design

## Decision 1: 依赖引入方式

**选项：**
- A) `path = "../leaf/crates/leaf-core"`（本地 path 依赖）
- B) `git = "https://github.com/StrayDragon/leaf"` + branch/rev

**决定：** A — 本地 path 依赖

**理由：**
- 开发期快速迭代，无需每次 push 上游
- 发布前可切换到 git 或 crates.io 引用
- `leaf-core` 作为稳定 facade，隔离上游 leaf 变动

## Decision 2: ratatui 版本差异处理

**背景：** xylitol 使用 ratatui 0.29，leaf-core re-exports ratatui 0.30 的 `Line`/`Span` 类型。

**选项：**
- A) 统一升级 xylitol 到 ratatui 0.30
- B) 在 MarkdownRenderer 边界做类型转换（0.30 → 0.29）
- C) 让 leaf-core 支持 ratatui 0.29 feature flag

**决定：** A — 统一升级到 ratatui 0.30

**理由：**
- ratatui 0.29 → 0.30 的 breaking changes 有限（主要是 API 重命名）
- 避免运行时类型转换开销和维护成本
- leaf-core 和 xylitol 共享同一 ratatui 版本，减少 compile 时间和二进制大小
- 升级范围可控：xylitol 的 ratatui 用量集中在 `src/interface/tui/`

## Decision 3: 接口保持策略

**决定：** `MarkdownRenderer` 保持现有 `pub(crate)` 接口签名不变

**细节：**
- `render(&self, markdown: &str, width: u16) -> Vec<Line<'static>>` — 签名不变
- `set_link_styled(&mut self, enabled: bool)` — 映射到 leaf-core 主题配置
- `set_theme(&mut self, theme_name: &str)` — 委托给 leaf-core 主题系统
- 内部实现完全替换为 `leaf_core::MarkdownRenderer` 委托调用

## Decision 4: 移除的自研代码

将删除以下辅助结构/函数（约 400 行）：
- `ListKind` / `ListState` enums
- `push_line` / `start_line` / `current_style` helper methods
- `pulldown-cmark` 事件循环遍历逻辑
- 手动 syntect 高亮集成

保留：
- `MarkdownRenderer` struct 定义（字段更新为 leaf-core 内部状态）
- 公开接口方法签名

## Migration Risk

- **低风险**：接口不变，调用方无需修改
- **中风险**：渲染输出样式变化会导致 insta 快照失败，需要批量更新
- **低风险**：ratatui 0.30 升级，xylitol TUI 代码量有限
