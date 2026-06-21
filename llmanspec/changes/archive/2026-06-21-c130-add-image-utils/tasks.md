# c130-add-image-utils: Tasks

## Implementation

- [x] 创建 `src/infra/image/mod.rs` — 模块入口
- [x] 创建 `src/infra/image/resize.rs` — 图片缩放 + base64 编码 + JPEG 回退
- [x] 创建 `src/infra/image/orientation.rs` — EXIF 方向枚举与校正（`#[allow(dead_code)]`）
- [x] 创建 `src/infra/image/format.rs` — 格式检测（魔术字节）
- [x] 添加 feature flag `infra-image` 到 `Cargo.toml`
- [x] 添加 `image` 依赖（可选，仅 PNG/JPEG/GIF/WebP）

## Testing

- [x] 单元测试 — 图片缩放比例正确（10x10 → 保持不变）
- [x] 单元测试 — base64 编码
- [x] 单元测试 — 格式检测（PNG/JPEG/未知）
- [x] 单元测试 — Default options 正确

## Verification

- [x] `cargo check --features infra-image` — 0 errors
- [x] `cargo test --lib --features infra-image` — 441 passed (+7 image tests)
- [x] `cargo test --lib` (default features) — 434 passed, no regression
- [x] `llman sdd validate c130-add-image-utils`
