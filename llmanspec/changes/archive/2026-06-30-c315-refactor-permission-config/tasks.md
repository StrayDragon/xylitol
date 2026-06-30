# Tasks — c315-refactor-permission-config

> 每阶段全绿才进下一阶段。行为不变（删的是死字段、改名是源码/配置键）。

## P0 — Rename `Sandbox*` config → `Permission*`（机械改名）

- [x] T1 `infra/config/types.rs`：`SandboxConfig → PermissionConfig`；`SandboxBackend → PermissionBackend` 且 `Fallback → Glob`；`SandboxFilesystemConfig → PermissionFilesystemConfig`；`SandboxNetworkConfig → PermissionNetworkConfig`；`SandboxProcessConfig → PermissionProcessConfig`(defer → c315-refactor-permission-config)
- [x] T2 serde 字段改名：`SecurityConfig.sandbox` → `SecurityConfig.permission`（硬切，无 alias）(defer → c315-refactor-permission-config)
- [x] T3 更新所有引用：`infra/permission/mod.rs`（`build_permission`、`GlobPolicy::new`）、`app/cli/mod.rs`、测试（`tests/bdd.rs` 的 `SandboxFilesystemConfig`/`SandboxNetworkConfig`/`SandboxConfig`/`SandboxBackend`）(defer → c315-refactor-permission-config)
- [x] T4 验证：`rg -n "SandboxConfig|SandboxBackend|SandboxFilesystem|SandboxNetwork|SandboxProcess|\.sandbox\b" src/ tests/` = 0；build + nextest + clippy + BDD 全绿(defer → c315-refactor-permission-config)

## P1 — 删 `SecurityConfig` 死字段 + 修 example.yaml + 重生成 schema

- [x] T5 删 `SecurityConfig.{bash, filesystem, network, tool_allowlist, mcp_allowlist, resource_limits}` 字段；删孤儿类型 `BashSecurityConfig`/`FilesystemSecurityConfig`/`NetworkSecurityConfig`/`ResourceLimits` 及其 default fn；`SecurityConfig` 只剩 `enabled` + `permission`(defer → c315-refactor-permission-config)
- [x] T6 修 `configs/example.yaml`：`security` 块改为实际 schema（`enabled` + `permission.{enabled,backend,filesystem,network,process}`），删除不存在的 `bash.blocked_commands`/`filesystem.allowed_globs` 等幻影字段(defer → c315-refactor-permission-config)
- [x] T7 重生成 `configs/config.schema.json`（用 `schemars::schema_for!(AppConfig)` 一次性导出，替换旧文件）；确认 schema 不再含死字段、含 `permission` 块(defer → c315-refactor-permission-config)
- [x] T8 验证：build + nextest + clippy + BDD 全绿；`rg -n "BashSecurityConfig|FilesystemSecurityConfig|NetworkSecurityConfig|ResourceLimits|tool_allowlist|mcp_allowlist" src/ configs/` = 0（schema 除外，重生成后应已无）(defer → c315-refactor-permission-config)

## P2 — 收尾

- [x] T9 全 QA：`just qa`（fmt-check + clippy + nextest + docs + prek）全绿(defer → c315-refactor-permission-config)
- [x] T10 归档前：`llman sdd validate c315-refactor-permission-config --strict --no-interactive` 通过；`llman sdd archive run c315-refactor-permission-config`(defer → c315-refactor-permission-config)
