---
depends_on: [c10-add-config, c20-add-tools]
---

# c40-add-hooks

## Why

Hook 事件系统允许用户在工具调用、模型查询、步骤完成等关键节点注册异步钩子（pre/post），实现自定义逻辑。这是声明式扩展的核心机制（§8）。

## What Changes

1. 在 `src/infra/hooks/` 实现 Hook 调度器（`HookDispatcher`）
2. 10+ 事件类型定义
3. 三级配置覆盖（全局/项目/用户会话）
4. 钩子脚本执行（stdin JSON 上下文 → stdout 控制指令）
5. 超时与阻断机制

> **⏸️ DAP PAUSED** — `tool_call.dap_command` 事件已暂停（2026-05-17）。DAP 开发恢复前不实现此事件。

### 支持的事件

| 事件 | 触发时机 | pre/post |
|------|---------|----------|
| `tool_call` | 任意工具执行 | ✓ |
| `tool_call.lsp_query` | LSP 查询 | ✓ |
| `tool_call.dap_command` | 调试器命令 | ✓ |
| `tool_call.file_write` | 文件写入 | ✓ |
| `model_query` | 调用模型前 | pre only |
| `step_complete` | 单步完成 | ✓ |
| `plan_generated` | 规划器生成计划后 | ✓ |
| `review_start/end` | 审查阶段 | ✓ |
| `repeat_detected` | 重复循环检测 | ✓ |
| `step_retry` | 步骤重试 | pre |
| `tool_call_blocked` | 安全拦截 | post |
| `session_snapshot` | 快照创建后 | post |
| `session_spawn` | 快照派生前 | pre |
| `session_merge` | 快照合并后 | post |

### 钩子执行协议

```bash
# stdin: JSON 上下文
{"event": "pre.tool_call", "tool": "bash", "args": {"command": "rm -rf /"}}

# stdout: 控制指令（可选）
{"action": "block", "reason": "Dangerous command"}
```

## Capabilities

- `hooks`: Hook 调度器 + 13+ 事件类型 + 三级配置 + 脚本执行

## Impact

- 新增 `tokio::process`（执行钩子脚本）
- `src/infra/hooks/` 从占位变为实际实现
- agent loop 和工具执行需要集成 hook 调度
- **始终编译**：无 feature flag，空 hooks 列表即 no-op dispatcher
