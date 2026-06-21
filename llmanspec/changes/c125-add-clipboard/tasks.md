# c125-add-clipboard: Tasks

## Implementation

- [ ] 创建 `src/infra/clipboard/mod.rs` — 模块入口，导出 `copy_to_clipboard`、`read_clipboard_image` 和 `ClipboardImage`
- [ ] 创建 `src/infra/clipboard/native.rs` — 平台原生工具封装
- [ ] 创建 `src/infra/clipboard/osc52.rs` — OSC 52 终端转义序列支持
- [ ] 创建 `src/infra/clipboard/image.rs` — 图片剪贴板读取
- [ ] 添加 feature flag `infra-clipboard` 到 `Cargo.toml`
- [ ] 添加 `arboard` 依赖（可选，仅当 feature 启用时）
- [ ] 实现平台选择逻辑：native addon → platform tool → OSC 52

## Testing

- [ ] 单元测试 — OSC 52 编码/解码（纯函数，可模拟 stdout）
- [ ] 单元测试 — 远程会话检测逻辑
- [ ] 单元测试 — 回退链顺序
- [ ] 集成测试 — `cargo test --features infra-clipboard`

## Verification

- [ ] `cargo check --features infra-clipboard`
- [ ] `cargo test --lib --features infra-clipboard`
- [ ] `llman sdd validate c125-add-clipboard --strict`
