---
depends_on: [c05-init-skeleton, c10-add-config]
blocks: [c25-add-agent-loop, c40-add-hooks, c65-add-skills-mcp, c75-add-diff-review, c80-add-tui]
---

# c20-add-tools

## Why

工具是 agent 与外界交互的唯一途径。7 个内置工具（read, bash, edit, write, grep, find, ls）是 agent 的基本能力，agent loop 依赖它们执行任务。

## What Changes

1. 在 `src/agent/tools/` 定义 `Tool` trait（基于 adk-core 的 `FunctionTool`）
2. 实现 7 个内置工具
3. 实现 AI patch 应用策略（fudiff 模糊匹配 + patch 精确兜底）
4. 工具注册表（`ToolRegistry`）

### 内置工具列表

| 工具 | 说明 | 关键依赖 |
|------|------|---------|
| read | 读取文件内容 | tokio::fs |
| bash | 执行 shell 命令 | tokio::process |
| edit | 搜索替换编辑文件 | 类似 codex old/new 模式 |
| write | 写入/创建文件 | tokio::fs |
| grep | 文本搜索 | grep crate |
| find | 文件查找 | glob/ignore |
| ls | 目录列表 | tokio::fs |

### Patch Apply 策略（§7.5）

```
fudiff 模糊匹配 → 成功 → 写入
                 → 失败 → patch 精确匹配 → 成功 → 写入
                                       → 失败 → 返回错误，代理重新生成
```

## Capabilities

- `tool-system`: Tool trait + 7 个内置工具 + ToolRegistry + patch apply 策略

## Impact

- 新增 `tokio`, `similar`, `fudiff`, `patch`, `glob`, `ignore`, `grep` 等依赖
- `src/agent/tools/` 从占位变为实际实现
