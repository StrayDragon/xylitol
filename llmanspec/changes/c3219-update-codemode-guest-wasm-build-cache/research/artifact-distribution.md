# codemode guest wasm 构建调研

## 结论

选择“源码构建 + 构建期嵌入”：不提交 wasm，不下载 release artifact。`build.rs` 在 `OUT_DIR` 生成 / 复制 wasm；host 用 `include_bytes!(concat!(env!("OUT_DIR"), ...))`。源码构建者需要固定 WASI SDK；分发二进制已包含 wasm。

## 一手来源

- Cargo Build Scripts：`build.rs` 在编译前执行；写入 `OUT_DIR`；用 `cargo::rerun-if-changed` 控制重跑。
  https://doc.rust-lang.org/cargo/reference/build-scripts.html
- Rust `include_bytes!` + `OUT_DIR`：构建脚本应把生成文件放到 `OUT_DIR`，不要修改源码目录。
  https://stackoverflow.com/questions/56014764/falling-back-to-alternative-value-if-include-bytes-target-is-missing
- GitHub Releases：release 可附带二进制资产；本方案不采用，避免外部供应链与网络依赖。
  https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases

## 方案对比

1. **源码构建 + 构建期嵌入（采用）**：本地 / CI 从 guest crate 构建；无网络；源码 + `Cargo.lock` + 固定 WASI SDK 防篡改；源码构建者需要 WASI SDK。
2. **提交 wasm（拒绝）**：构建简单；但 880 KB 生成物不可审查、易过期、与源码不一致。
3. **下载 release artifact（拒绝）**：分发方便；但引入网络、缓存、供应链与离线问题。

## 待决策

- `build.rs` 自动构建，还是要求先跑 `just codemode-guest`；递归 cargo 风险需评估。
- WASI SDK pin 方式与 CI runner。
