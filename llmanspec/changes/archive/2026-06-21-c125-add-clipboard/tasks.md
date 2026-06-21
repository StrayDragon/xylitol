# c125-add-clipboard: Tasks

## Implementation

- [x] 创建 `src/infra/clipboard/mod.rs` — 模块入口，导出 `copy_to_clipboard`、`read_clipboard_image` 和 `ClipboardImage`
- [x] 创建 `src/infra/clipboard/native.rs` — 平台原生工具封装（pbcopy, clip, wl-copy, xclip, xsel, termux-clipboard-set）
- [x] 创建 `src/infra/clipboard/osc52.rs` — OSC 52 终端转义序列支持（含内联 base64 编码）
- [x] 创建 `src/infra/clipboard/image.rs` — 图片剪贴板读取（wl-paste, xclip, macOS, PowerShell）
- [x] 添加 feature flag `infra-clipboard` 到 `Cargo.toml` 和 `infra/mod.rs`
- [x] 添加 `arboard` 依赖（defer: 使用平台原生工具代替，避免 Linux Wayland/X11 所有权问题）
- [x] 实现平台选择逻辑：platform tool → OSC 52 回退

## Testing

- [x] 单元测试 — OSC 52 编码/解码（纯函数，可模拟 stdout）
- [x] 单元测试 — 远程会话检测逻辑（env var 设置/清除）
- [x] 单元测试 — 回退链顺序（工具存在性检测）
- [x] 集成测试 — 14 个 clipboard 测试通过

## Verification

- [x] `cargo check --features infra-clipboard` — 0 errors
- [x] `cargo test --lib --features infra-clipboard` — 14 passed
- [x] `llman sdd validate c125-add-clipboard --strict`
