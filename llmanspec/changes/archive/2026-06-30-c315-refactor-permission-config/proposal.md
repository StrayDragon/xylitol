---
depends_on: []
---

# c315-refactor-permission-config

## Why

c265 renamed the runtime side to `XyPermission` / `AllowAllPermission` / `GlobPolicy`
and renamed the `infra/sandbox` module to `infra/permission`. But the **config schema**
was left speaking `Sandbox`, so the noun flips at the config↔runtime boundary:

- `options.permission` is built from `security.sandbox` (`SandboxConfig`).
- `SandboxBackend::Fallback` constructs a `GlobPolicy` — the variant name and the
  type it builds disagree.
- The config types (`SandboxConfig`, `SandboxFilesystemConfig`, `SandboxNetworkConfig`,
  `SandboxProcessConfig`, `SandboxBackend`) still say "Sandbox".

Worse, `SecurityConfig` carries a **second, fully-dead parallel rule set** that nothing
reads: top-level `bash` / `filesystem` / `network` (`BashSecurityConfig` /
`FilesystemSecurityConfig` / `NetworkSecurityConfig`), `tool_allowlist`,
`mcp_allowlist`, and `resource_limits`. Grep confirms zero Rust readers outside
`infra/config/types.rs`. The only live path is `security.sandbox.*`
(→ `build_permission` → `GlobPolicy`). The dead fields are never consulted by the
ReAct loop, the permission port, or any tool.

And `configs/example.yaml` documents a `security:` block whose field names
(`bash.blocked_commands`, `filesystem.allowed_globs`, …) **do not exist in the Rust
types at all** — it actively misleads users. The generated `configs/config.schema.json`
emits the dead fields too.

This change restores a single honest noun end-to-end and deletes the dead parallel
config surface.

## What Changes

### P0 — Rename `Sandbox*` config → `Permission*` (mechanical)
1. `infra/config/types.rs`: `SandboxConfig → PermissionConfig`;
   `SandboxBackend → PermissionBackend` with `Fallback → Glob` (the variant that
   builds `GlobPolicy`); `SandboxFilesystemConfig → PermissionFilesystemConfig`;
   `SandboxNetworkConfig → PermissionNetworkConfig`;
   `SandboxProcessConfig → PermissionProcessConfig`.
2. Config key `security.sandbox` → `security.permission` (serde field rename,
   hard cut, no alias — pre-1.0.0 per AGENTS.md).
3. Update all readers: `infra/permission/mod.rs` (`build_permission`,
   `GlobPolicy::new`), `app/cli/mod.rs`, tests.

### P1 — Remove dead parallel `SecurityConfig` fields
4. Delete `SecurityConfig.{bash, filesystem, network, tool_allowlist, mcp_allowlist,
   resource_limits}` and their orphan types `BashSecurityConfig`,
   `FilesystemSecurityConfig`, `NetworkSecurityConfig`, `ResourceLimits` (and their
   default fns). `SecurityConfig` keeps only `enabled` and `permission`.
5. Update `example.yaml` + regenerate `config.schema.json` so the documented schema
   matches reality (no phantom fields, `security.permission` block).

### P2 — Validate
6. `just qa` green; `rg -n "SandboxConfig|SandboxBackend|security\.sandbox" src/ configs/` = 0.

## Capabilities

- `security-policy`: `s10` (config struct) renamed to `PermissionConfig` + key
  `security.permission` + backend `Glob`; `s14` path updated; dead `r8`
  (`bash-timeout-cap`, referenced the removed `config.security.bash.timeout_secs`
  and was never implemented) removed.

## Impact

- **Code:** `infra/config/types.rs`, `infra/permission/mod.rs`, `app/cli/mod.rs`,
  tests, `configs/example.yaml`, `configs/config.schema.json`.
- **Behavior:** unchanged (deleted fields were never read; the rename is
  source/config-key only). The config key `security.sandbox` → `security.permission`
  is a **breaking YAML change** (pre-1.0.0; no compat shim per AGENTS.md).
- **Data:** none (config is load-time; nothing persisted).
- **HC:** none — config-layer only; no new agent↔infra coupling.
- **Out of scope:** the remaining never-implemented `security-policy` requirements
  (r1–r4 `SecurityToolWrapper`/`SecurityEngine`, r6 hook-timeout, r7 network-engine)
  describe a dead engine, not config fields, and are reconciled in a future change.
