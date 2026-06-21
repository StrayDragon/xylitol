# c145-add-tool-downloader: Tasks

## Implementation

- [ ] 创建 `src/infra/tools/mod.rs` — 模块入口
- [ ] 创建 `src/infra/tools/downloader.rs` — HTTP 下载 + GitHub API
- [ ] 创建 `src/infra/tools/extract.rs` — tar.gz/zip 提取
- [ ] 创建 `src/infra/tools/platform.rs` — 平台检测 + 资产名映射
- [ ] 添加 feature flag `infra-tools` 到 `Cargo.toml`
- [ ] 添加 `flate2`、`tar`、`zip` 依赖

## Testing

- [ ] 单元测试 — 平台资产名映射（macOS/Linux/Windows × x86_64/arm64）
- [ ] 单元测试 — 离线模式检测
- [ ] 单元测试 — tar.gz 提取（mock 文件）
- [ ] 单元测试 — zip 提取（mock 文件）
- [ ] 集成测试 — GitHub API 调用（wiremock）

## Verification

- [ ] `cargo check --features infra-tools`
- [ ] `cargo test --lib --features infra-tools`
- [ ] `llman sdd validate c145-add-tool-downloader`
