# c155-add-platform-utils: Tasks

## Implementation

- [x] 创建 `src/infra/update/mod.rs` — 版本检查（HTTP API + semver 比较）
- [x] 创建 `src/infra/changelog/mod.rs` — CHANGELOG.md 解析
- [x] 创建 `src/infra/fs_watch/mod.rs` — 文件系统监控（notify crate）
- [x] 创建 `src/infra/browser/mod.rs` — 浏览器打开（跨平台）
- [x] 添加 feature flag `infra-platform` 到 `Cargo.toml`
- [x] 添加 `notify` 依赖

## Testing

- [x] 单元测试 — CHANGELOG.md 解析（2 个版本）
- [x] `cargo check --features infra-platform` — 0 errors
- [x] `cargo test --lib --features infra-platform` — 437 passed

## Verification

- [x] `llman sdd validate c155-add-platform-utils`
