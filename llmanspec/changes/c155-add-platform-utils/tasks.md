# c155-add-platform-utils: Tasks

## Implementation

- [ ] 创建 `src/infra/update/mod.rs` — 版本检查
- [ ] 创建 `src/infra/changelog/mod.rs` — 变更日志解析
- [ ] 创建 `src/infra/fs-watch/mod.rs` — 文件监控
- [ ] 创建 `src/infra/browser/mod.rs` — 浏览器打开
- [ ] 增强 `src/infra/frontmatter/` — 提取公共 `parse_frontmatter()` 到独立模块
- [ ] 添加 feature flag `infra-platform` 到 `Cargo.toml`
- [ ] 添加 `notify`、`open`、`semver` 依赖

## Testing

- [ ] 单元测试 — 版本检查请求/解析（wiremock）
- [ ] 单元测试 — CHANGELOG.md 解析
- [ ] 单元测试 — 文件监控回调触发
- [ ] 单元测试 — 前端内容解析
- [ ] 单元测试 — 浏览器 URL 打开

## Verification

- [ ] `cargo check --features infra-platform`
- [ ] `cargo test --lib --features infra-platform`
- [ ] `llman sdd validate c155-add-platform-utils`
