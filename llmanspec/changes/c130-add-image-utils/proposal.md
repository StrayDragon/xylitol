---
change_id: c130-add-image-utils
depends_on: []
---

# c130-add-image-utils: 添加图片处理模块

## Why

pi 提供了完整的图片处理工具链（`image-resize-core.ts` + `image-resize.ts` + `photon.ts` + `exif-orientation.ts` + `image-convert.ts`，共约 630 行），支持：
- 图片缩放（限制最大宽度/高度/字节数）
- JPEG 质量压缩
- EXIF 方向校正
- 图片格式转换
- 适合多模态 LLM 输入的 payload 优化

xylitol 当前完全没有图片处理能力，无法支持多模态 LLM（如 Claude 3 视觉模型）的图片输入。

## What Changes

在 `src/infra/image/` 下创建新模块，可选的 `infra-image` feature：

1. `mod.rs` — 公共 API（`resize_image`、`apply_orientation`、`convert_format`）
2. `resize.rs` — 图片缩放逻辑（`image` crate 的 `resize_exact`/`thumbnail`）
3. `orientation.rs` — EXIF 方向读取与校正（`kamadak-exif` crate）
4. `format.rs` — 格式检测与转换

## Capabilities

- 新增 capability: `image-utils`

## Impact

- 新增可选 feature `infra-image`
- 新增依赖 `image`、`kamadak-exif`
- 约 400-500 行新 Rust 代码
