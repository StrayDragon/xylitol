# c130-add-image-utils: Design

## Architecture

```
src/infra/image/
├── mod.rs         # Public API: resize_image(), apply_orientation(), convert_format()
├── resize.rs      # Image scaling using `image` crate (thumbnail, resize_exact)
├── orientation.rs # EXIF orientation using `kamadak-exif` crate
└── format.rs      # Format detection, conversion, JPEG quality selection
```

## Key Decisions

1. **Use `image` crate**: The most mature Rust image processing library. Supports PNG, JPEG, WebP, GIF.
2. **Use `kamadak-exif` for EXIF**: Lightweight, pure Rust EXIF reader.
3. **Dimension limits**: `max_width=2000`, `max_height=2000`, `max_bytes=4.5MB` (below Anthropic's 5MB limit).
4. **JPEG fallback for size**: When PNG exceeds byte limit, convert to JPEG with configurable quality (default 80).
5. **Feature gate**: `infra-image` optional feature.

## Integration

- Used by tool definitions (read tool for multimodal image support)
- Used by clipboard image paste (c125)
