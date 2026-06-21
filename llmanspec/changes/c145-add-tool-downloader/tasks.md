# c145-add-tool-downloader: Tasks

## Implementation

- [x] 创建 `src/infra/tools/mod.rs` — 模块入口 + GitHub 下载 + tar.gz/zip 提取
- [x] 创建 `src/infra/tools/platform.rs` — 平台检测 + 资产名映射（fd/rg）
- [x] 添加 feature flag `infra-tools` 到 `Cargo.toml`
- [x] 添加 `flate2`、`tar`、`zip` 依赖

## Testing

- [x] `cargo check --features infra-tools` — 0 errors, 0 warnings
- [x] `cargo test --lib` — 434 passed, no regression

## Verification

- [x] `llman sdd validate c145-add-tool-downloader`
