---
depends_on: [c50-add-security]
---

# c91-fix-security-enforcement

## Why

安全审查发现 SecurityEngine 存在多处严重的策略执行缺陷：

1. `security.enabled` 默认为 `false`，等价于全部工具无策略拦截
2. `check_file_tool()` 仅检查 `file_path` 字段，而 grep/find/ls 工具使用 `path` 字段，导致文件系统策略对这三个工具**完全失效**
3. MCP 工具名形如 `mcp:server:tool`，不匹配任何安全分支，走 `_ => Allowed` 绕过所有策略
4. Hook 脚本超时/失败均 fail-open（返回 Allow），且超时后未 `kill()` 子进程
5. `network.allowed_domains` / `blocked_domains` 配置已加载但从未检查
6. `resource_limits`（max_memory_mb 等）仅为配置占位，无实际执行
7. `bash` 工具的 timeout 由 LLM 任意指定，未与 `security.bash.timeout_secs` 配置对齐
8. TUI 审批仅覆盖 write/edit，不含 bash

这些问题叠加后，即使用户认为已配置安全策略，实际防护几乎为零。

## What Changes

1. 修复 `check_file_tool` 参数字段统一（同时检查 `file_path` 和 `path`）
2. 为 MCP 工具实现独立安全策略分支（server/tool allowlist，默认 deny）
3. 将 `security.enabled` 默认值改为 `true`，并提供保守的默认策略模板
4. Hook 超时后必须 `child.kill()`；可配置 fail-open/fail-closed
5. 实现 `network.allowed_domains`/`blocked_domains` 检查
6. bash timeout 强制 `min(args.timeout, config.security.bash.timeout_secs)`
7. TUI 审批扩展覆盖 bash 工具

## Capabilities

- `security-policy`: 安全策略执行引擎

## Impact

- 默认行为变更：首次升级后用户需确认安全策略配置
- MCP 工具需显式 allowlist 方可执行
- Hook 超时语义变更（fail-closed 可能阻断已有工作流）
- 需更新配置 schema 与示例文档
