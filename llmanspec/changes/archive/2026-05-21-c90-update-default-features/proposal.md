---
id: c90-update-default-features
depends_on: []
priority: 90
---

# Update Default Features

## Why

当前 `default` feature 仅包含 4 项（`agent-planning`, `ui-tui`, `infra-session`, `ui-review`），
其余 7 个非 dev feature 需要手动 `--features` 才能启用。对于日常开发和最终用户而言，
`cargo build` 应产出全功能二进制，而非需要记忆每个 feature flag。
关闭部分 feature 的场景（如最小化编译、CI 矩阵）可通过 `--no-default-features` 实现。

## What Changes

- 将 `Cargo.toml` 的 `default` 列表扩展为所有非 `dev-*` feature。
- `dev-vt100`、`dev-e2e`、`dev-fake-provider` 保持不在 `default` 中（仅用于开发/测试）。

### 新 default 列表

```
default = [
  "agent-planning",
  "agent-model-lock",
  "infra-lsp",
  "infra-dap",
  "infra-acp",
  "infra-skills",
  "infra-session",
  "infra-sandbox",
  "infra-rtk",
  "ui-tui",
  "ui-review",
]
```

## Capabilities

- `build-config`: Cargo.toml feature flags 配置

## Impact

- 编译时间增加（引入更多可选依赖如 `agent-client-protocol`）。
- CI 中如需最小化构建，应使用 `--no-default-features --features ...` 显式选择。
- 下游用户如需精简构建，同样用 `--no-default-features` 关闭全部后按需开启。
