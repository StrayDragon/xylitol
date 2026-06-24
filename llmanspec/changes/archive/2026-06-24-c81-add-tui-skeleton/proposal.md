---
depends_on: [c80-add-tui]
---

# c81-add-tui-skeleton

## Why

c80-add-tui 定义了完整的 25 组件 TUI 架构，但 scope 过大且架构（全 retained Component 树）已过时。
本 change 重新设计 TUI 架构为 **Codex 风格单列流式 + select! 事件循环**（参考 PRD §2），
分阶段实现：本 change 只做 **P0 骨架**（select! 循环 + terminal setup + 纯文本 transcript/composer），
后续 change 再叠加 markdown 渲染、工具卡片、选中复制等能力。

## What Changes

1. 在 `src/interface/tui/` 创建 TUI 模块骨架
   - state/   — 纯函数状态机（Transcript + Composer），不依赖 ratatui
   - render/  — ratatui 投影层（含 Selection 选中复制 widget）
   - input/   — 三层键盘模型（decode → keymap → action），纯函数可单测
   - mod.rs   — select! 事件循环 + terminal setup/restore
2. 新增 CLI 子命令 `xylitol tui` 入口
3. 新增 feature flag `ui-tui`（依赖 ratatui / crossterm 等已在 Cargo.toml 中）
4. 已有的 `src/interface/diff_review/cli.rs` 保持原样，不迁移

### 技术栈

| 组件 | Crate |
|------|-------|
| TUI 框架 | ratatui 0.30.2 |
| 终端后端 | crossterm 0.29 |
| 多行编辑器 | ratatui-textarea 0.9.2（composer） |
| 选中复制 | 自研 Selection + OSC 52（base64） |
| CJK 宽度 | unicode-width |

### 非 P0 范围（后续 change 覆盖）

- Markdown 渲染（pulldown-cmark + syntect）
- 工具调用卡片（折叠/展开/spinner）
- overlay 系统（帮助/审批/选择器）
- 底部 status bar（模型/状态/队列计数）
- 图片内联显示
- session/模型/主题选择器
- 历史搜索

## Capabilities

- `tui-interface`: TUI 交互模式

## Impact

- 新增 ~5 crate 依赖（ratatui-textarea, unicode-width, unicode-segmentation, base64）已在 Cargo.toml 中
- 新增 `ui-tui` feature flag（默认关闭，后续可默认开启）
- `src/interface/` 新增 `tui/` 子模块
- 与已有 `print` / `rpc` / `diff_review` 模式平行共存
- 零影响现有 CLI、agent loop、session 等核心代码
