# codemode guest wasm 分发调研

## 结论

社区常见做法是：生成物不长期放源码；CI 构建并发布不可变 artifact；构建期按固定 hash 获取并校验；离线走缓存或本地路径。`build.rs` 应把文件写入 `OUT_DIR`，源码用 `include_bytes!(concat!(env!("OUT_DIR"), ...))`。最终 artifact 托管与 `build.rs` 网络策略仍需项目决策。

## 一手来源

- Cargo Build Scripts：`build.rs` 在编译前执行；写入 `OUT_DIR`；用 `cargo::rerun-if-changed` 控制重跑。  
  https://doc.rust-lang.org/cargo/reference/build-scripts.html
- GitHub Releases：release 可附带二进制资产，适合不可变发布物。  
  https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases
- Rust `include_bytes!` + `OUT_DIR`：构建脚本应把生成文件放到 `OUT_DIR`，不要修改源码目录。  
  https://stackoverflow.com/questions/56014764/falling-back-to-alternative-value-if-include-bytes-target-is-missing
- Release integrity 实践：release assets + SHA256SUMS + provenance / SBOM；下载后校验。  
  https://github.com/oaslananka/boardreadyops/blob/469086c5f54ceae0307ae241d51405ccd252f289/docs/security/release-integrity.md
- Rust release workflow 示例：多平台构建、checksum、release asset。  
  https://github.com/xctions/xctions-rust-release

## 候选方案

1. **源码构建优先 + 分发资产回退**：本地 `just codemode-guest` 从 guest crate 构建；CI 发布 release artifact + SHA256；`build.rs` 解析 env / 缓存 / 预取资产。本地自然、可审计；无 WASI SDK 时用分发资产。
2. **GitHub Release + `build.rs` 下载**：实现简单；构建需要网络；离线需缓存 / env。
3. **GitHub Release + `just setup` 预取**：`build.rs` 只校验本地文件；`cargo build` 不联网；多一步 setup。
4. **对象存储 + 内部缓存**：适合企业；需要额外基础设施。

## 待决策

- 选 1 还是 2；是否需要企业离线模式。
- artifact 命名与 hash 清单格式。
- provenance / SBOM 工具与 CI runner。
