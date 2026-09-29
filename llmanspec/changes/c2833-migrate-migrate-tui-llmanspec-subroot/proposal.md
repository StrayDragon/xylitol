---
depends_on: []
---

## Why

上游 llman-sdd 0.7.0 已实现 subproject-llmanspec-discovery（issue #6）。子包 specs 就近管理：staleness 按实例根计算、验证命令按子包 runner、xylitol-tui（纯单测，无 rstest-bdd）获得独立 specs 所有权。

## What Changes

1. `packages/xylitol-tui/llmanspec/` 子根 init（无 --skills），config 配 `specs.check_command: cargo test -p xylitol-tui`。
2. 16 个 `package-tui-*` capability 目录 git mv 到子根 specs。
3. bridge 族拆分：`packages/xylitol-ai-bridge/` 相关部分迁 `packages/xylitol-ai-bridge/llmanspec/`（同法 init）；`src/infra/provider/`、`src/agent/compaction/` 根侧部分拆为新根 capability（spec next-req-id 取号）。app-tui-bridge（纯 src/app/tui/）留守。
4. 根 specs 不再 scope 进 packages/*；`--all-roots` 双根聚合验证。
5. 根 `init --update` 刷新托管块。

## Scope

- `llmanspec/**`、`packages/xylitol-tui/llmanspec/**`、`packages/xylitol-ai-bridge/llmanspec/**`

## Out of Scope

- BDD 绑定（tests/bdd）不迁移：主 crate 单一二进制注册表按绝对路径消费 live specs（rstest-bdd `path=` 改指子根文件）。
