---
depends_on: []
---

# c97-fix-archived-deferred-items

## Why

2026-05-24 批次归档的 7 个变更中，有 27 个 tasks（约 39%）被标注 `defer` 后直接归档，没有产生后续 change proposal 来追踪。这些遗留项分散在安全、竞态、测试稳定性、架构重构、代码卫生和 Markdown 渲染等多个领域，存在被长期遗忘的风险。

本变更的目标是：系统性清理这些 deferred items，将仍有价值的项纳入追踪，明确取消不再需要的项，并修复已发现的 spec 违规。

## What Changes

1. **Markdown 渲染修复**（来自 c90-update-markdown-rendering defer 项）：
   - 链接样式默认不强制 underline+color（spec r2 合规）— **已实现**
   - 表格渲染添加 `│` 分隔符 — **已实现**

2. **归档 defer 清单盘点**：逐一审查以下归档变更的 deferred tasks，分类为「纳入本变更」「创建独立变更」「取消」：
   - c92-refactor-code-hygiene：4 项 defer（args helper 提取、app.rs 拆分、可见性统一、错误类型约定）
   - c94-fix-race-conditions：5 项 defer（ApprovalHub 重构、mtime 校验、session ensure、MCP 锁、history 文件锁）
   - c95-fix-test-stability：7 项 defer（tempdir 改造、timeout helper、async timeout、sleep 缩短、unix 标记、dev-deps 清理、MockToolContext、paths fallback）
   - c96-refactor-architecture：6 项 defer（model DTO、security wrap、bootstrap 提取、feature 审查、full alias）
   - c93-fix-resource-boundary：2 项 defer（zstd 解压限制、ngram 容量上限）

3. **流程改进**：在 AGENTS.md 中补充 defer 归档规范。

## Capabilities

- `markdown-rendering`: 终端 Markdown 渲染修复（链接样式配置化、表格分隔符）

## Impact

- Markdown 渲染快照测试更新（表格输出格式变化）
- 链接文本默认不再有 LightBlue+Underline 样式（行为变更，符合 spec r2）
- 不影响其他已归档变更的 spec 合规性
