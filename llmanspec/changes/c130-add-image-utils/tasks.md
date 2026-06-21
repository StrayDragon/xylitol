# c130-add-image-utils: Tasks

## Implementation

- [ ] 创建 `src/infra/image/mod.rs` — 模块入口
- [ ] 创建 `src/infra/image/resize.rs` — 图片缩放
- [ ] 创建 `src/infra/image/orientation.rs` — EXIF 方向校正
- [ ] 创建 `src/infra/image/format.rs` — 格式检测与转换
- [ ] 添加 feature flag `infra-image` 到 `Cargo.toml`
- [ ] 添加 `image` 和 `kamadak-exif` 依赖

## Testing

- [ ] 单元测试 — 图片缩放比例正确
- [ ] 单元测试 — EXIF 方向校正
- [ ] 单元测试 — 格式转换与 JPEG 质量
- [ ] 单元测试 — payload 大小限制

## Verification

- [ ] `cargo check --features infra-image`
- [ ] `cargo test --lib --features infra-image`
- [ ] `llman sdd validate c130-add-image-utils`
