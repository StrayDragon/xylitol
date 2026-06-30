# c315-refactor-permission-config — Design

> 配置层的诚实命名 + 死代码清理。c265 已把运行时改名为 `XyPermission`；本变更把
> 配置 schema 对齐，并删除从未通电的平行 `SecurityConfig` 规则集。

## 1. 单一名词链（目标）

```
config.yaml:  security.permission   (PermissionConfig)
                          ↓ build_permission
infra:        GlobPolicy / AllowAllPermission
                          ↓
runtime:      XyPermission / XyPermissionVerdict
```

改名后配置↔运行时不再换名词。`PermissionBackend::Glob` 与它构建的 `GlobPolicy`
名字一致（旧的 `SandboxBackend::Fallback` 构建的是 `GlobPolicy`，自相矛盾）。

## 2. 删除的平行死字段

`SecurityConfig` 当前有两套规则：

| 字段 | 类型 | 有无读取方 |
|------|------|-----------|
| `security.permission`（原 `sandbox`） | `PermissionConfig` | ✅ `build_permission` → `GlobPolicy` |
| `security.bash` | `BashSecurityConfig` | ❌ 零 Rust 读取方 |
| `security.filesystem` | `FilesystemSecurityConfig` | ❌ 零 |
| `security.network` | `NetworkSecurityConfig` | ❌ 零 |
| `security.tool_allowlist` | `Vec<String>` | ❌ 零 |
| `security.mcp_allowlist` | `Vec<String>` | ❌ 零 |
| `security.resource_limits` | `ResourceLimits` | ❌ 零 |

注意：`infra/permission/GlobPolicy` 读的是 `PermissionConfig.{filesystem,network}`
（原 `SandboxFilesystemConfig`/`SandboxNetworkConfig`），**不是** 顶层那套
`FilesystemSecurityConfig`/`NetworkSecurityConfig`。两套互不相干，顶层那套从未被
任何 `check_*` 消费。删它无行为影响。

bash 超时：`config.security.bash.timeout_secs`（`BashSecurityConfig`）零读取方；
真实 bash 超时来自 `BashTool::MAX_TIMEOUT_SECS` 与 `bash_exec::DEFAULT_TIMEOUT_SECS`。
故 `security-policy` 的 r8（`bash-timeout-cap`，引用该字段且从未实现）一并移除。

## 3. example.yaml 与 schema 的幻影

`configs/example.yaml` 的 `security:` 块写了 `bash.blocked_commands`、
`filesystem.allowed_globs` 等字段名——Rust 类型里根本不存在。生成出来的
`config.schema.json` 一并含这些幻影 + 死字段。本变更把 example 改成真实 schema，
并用 `schemars::schema_for!(AppConfig)` 重生成 schema（一次性导出覆盖）。

## 4. 破坏性

`security.sandbox` → `security.permission` 是 **YAML 键破坏性变更**。pre-1.0.0，
按 AGENTS.md 不留 compat shim，一步到位。现有用户（如有）需改 config.yaml 的键名。

## 5. 不在本变更范围

`security-policy` 的 r1–r4（`SecurityToolWrapper`/`SecurityEngine` 三层覆盖/MCP）、
r6（hook-timeout，属 `hook-system`）、r7（network-engine 措辞）描述的是一个从未
实现的 `SecurityEngine`，**不是** 本轮删除的配置字段。它们与 `s15`（permission 非
安全边界）的诚实命名立场有张力，留作独立 spec-cleanup 变更（记入 future.md）。
