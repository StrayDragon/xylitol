---
depends_on: [c20-add-tools, c40-add-hooks]
---

# c75-add-diff-review

## Why

交互式 Diff 评审允许用户在 agent 完成步骤后审查变更，添加行级评论，选择性接受或拒绝。支持 CLI 终端内评审（默认）和 Web 浏览器评审（可选）（§7）。

## What Changes

1. 在 `src/interface/` 实现 Diff 评审系统
2. CLI 终端内评审（基于 ratatui）
   - Diff 渲染（similar + ratatui Paragraph）
   - 语法高亮（syntect）
   - 行级评论（ratatui-textarea）
   - 交互键位（j/k 导航, c 评论, a 接受, r 拒绝）
3. Web 浏览器评审（axum + Monaco Editor）
   - 与 CLI 模式共享评论数据结构
4. 审查流程：步骤完成 → diff 生成 → 用户评审 → Accept/Reject

### 共享数据结构

```rust
struct ReviewComment { file, line_start, line_end, content, severity }
enum ReviewVerdict { AcceptAll, RejectWithComments(Vec<ReviewComment>) }
```

### YAML 配置

```yaml
review:
  enabled: true
  mode: "on-step"       # on-step | on-error | manual
  backend: "cli"        # cli | web
```

## Capabilities

- `diff-review`: CLI + Web 双后端 Diff 评审 + 行级评论

## Impact

- 新增 `ratatui-textarea`, `syntect`, `axum` 依赖
- feature flag `ui-review` 启用此模块
- 依赖 c20 工具集（similar diff 生成）和 c40 hooks（审查事件）
