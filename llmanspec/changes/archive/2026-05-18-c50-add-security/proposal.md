---
depends_on: [c10-add-config, c40-add-hooks]
---

# c50-add-security

## Why

## What Changes

1. 在 `src/infra/security/` 实现安全策略引擎（`SecurityPolicy`）
2. 三级限制：bash 命令（正则匹配）、文件系统（glob 路径）、网络（域名/IP）
3. 规则合并（仅收紧不放宽）
4. 资源配额（子进程数、内存、CPU 时间）
5. 拦截事件 `tool_call_blocked`

### 安全策略结构

```yaml
security:
  tool_allowlist: [...]
  bash:
    allowed_patterns: [...]
    forbidden_patterns: [...]
    timeout_seconds: 120
    max_output_bytes: 1048576
  filesystem:
    path_allowlist: [...]
    path_blocklist: [...]
    allow_outside_workspace: false
  network:
    allowed_hosts: [...]
    default_policy: "deny"
  resource_limits:
    max_subprocesses: 16
    max_memory_mb: 2048
```

### 规则优先级

内置层先于 Hook 执行。Hook 可进一步阻止，但无法放宽内置限制。

## Capabilities

- `security`: 声明式工具调用安全策略 + 三级限制 + 规则合并 + sandbox feature flag 预留

## Impact

- 新增 `globset`（路径匹配）、`regex`（命令匹配）依赖
- **始终编译**：无 feature flag，通过 `security.enabled` config 控制运行时启用
- 工具执行前必须经过安全检查
