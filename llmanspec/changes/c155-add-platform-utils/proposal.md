---
change_id: c155-add-platform-utils
depends_on: []
---

# c155-add-platform-utils: 添加平台工具模块

## Why

pi 提供了多个小型工具模块（共约 400 行），各自提供单一职责：

- `version-check.ts`（80 行）：检查最新 pi 版本
- `changelog.ts`（196 行）：解析并展示 CHANGELOG.md
- `fs-watch.ts`（30 行）：健壮的文件系统监控
- `frontmatter.ts`（39 行）：YAML 前端内容解析
- `sleep.ts`（18 行）：异步 sleep 工具
- `open-browser.ts`（24 行）：打开浏览器 URL
- `deprecation.ts`（14 行）：弃用警告

xylitol 当前缺少这些小型工具。它们对于 TUI 模式、启动体验和通知至关重要。

## What Changes

分组创建多个小型模块：

1. `src/infra/update/mod.rs` — 版本检查（`check_for_new_version()`）
2. `src/infra/changelog/mod.rs` — CHANGELOG.md 解析
3. `src/infra/fs-watch/mod.rs` — 文件系统监控
4. `src/infra/frontmatter/` — 增强现有 YAML 前端内容解析
5. `src/infra/browser/mod.rs` — 浏览器打开

## Capabilities

- 新增 capability: `platform-utils`

## Impact

- 混合 feature 门控：update、changelog、fs-watch 为 `infra-platform` feature
- frontmatter 增强为内置（已有部分代码）
- 约 300-400 行新 Rust 代码
