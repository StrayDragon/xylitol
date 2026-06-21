# c150-add-syntax-highlight: Tasks

## Implementation

- [x] 创建 `src/infra/syntax/mod.rs` — 模块入口（highlight, supports_language, html_to_ansi）
- [x] 添加 feature flag `infra-syntax` 到 `Cargo.toml`
- [x] 添加 `syntect` 依赖（default-syntaxes, default-themes, regex-onig, html）

## Testing

- [x] 单元测试 — Rust 代码高亮
- [x] 单元测试 — 语言自动检测
- [x] 单元测试 — HTML 标签剥离
- [x] 单元测试 — HTML 实体解码
- [x] `cargo check --features infra-syntax` — 0 errors
- [x] `cargo test --lib --features infra-syntax` — 7 passed

## Verification

- [x] `llman sdd validate c150-add-syntax-highlight`
