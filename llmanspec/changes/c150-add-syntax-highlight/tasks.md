# c150-add-syntax-highlight: Tasks

## Implementation

- [ ] 创建 `src/infra/syntax/mod.rs` — 模块入口
- [ ] 创建 `src/infra/syntax/theme.rs` — 主题定义
- [ ] 创建 `src/infra/syntax/render.rs` — 高亮渲染输出
- [ ] 添加 feature flag `infra-syntax` 到 `Cargo.toml`
- [ ] 添加 `syntect` 依赖

## Testing

- [ ] 单元测试 — Rust 代码高亮（关键字、字符串、注释）
- [ ] 单元测试 — 语言自动检测
- [ ] 单元测试 — 主题映射正确性

## Verification

- [ ] `cargo check --features infra-syntax`
- [ ] `cargo test --lib --features infra-syntax`
- [ ] `llman sdd validate c150-add-syntax-highlight`
