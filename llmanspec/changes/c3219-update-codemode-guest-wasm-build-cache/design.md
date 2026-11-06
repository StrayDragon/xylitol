# Design — 从源码构建并内置 guest wasm

## 目标

- 从仓库源码构建 guest；构建时嵌入 host；不提交 wasm，不下载外部二进制。
- 分发二进制已包含 wasm；源码构建者用固定 WASI SDK 自行构建。
- 防篡改：源码 + `Cargo.lock` + 固定工具链；无外部 artifact 供应链。

## 现状

- guest 是独立 crate，`rquickjs 0.14` + QuickJS C 代码，target `wasm32-wasip1`。
- 需要 WASI SDK 24 提供 `clang` / `wasi-sysroot`；主 workspace 的 `cargo build` 不编译 guest。
- `guest.wasm` 已提交；`just codemode-guest` 手动重建；无 hash 校验。
- host 用 `include_bytes!("../assets/guest.wasm")` 编译期嵌入；`module_cache` 控制启动编译一次或每次运行编译。

## 决策

### 1. 源码构建

- `build.rs` 在构建时检查 `OUT_DIR/guest.wasm`：
  - 已存在则直接使用；
  - 不存在则从 guest crate 构建：`cargo build --release --target wasm32-wasip1`，设置 `CC_wasm32_wasip1` / `AR_wasm32_wasip1` / `CFLAGS_wasm32_wasip1`，使用独立 target dir，避免与主 workspace 竞争；
  - 复制到 `OUT_DIR/guest.wasm`。
- `WASI_SDK_PATH` 必须指向 WASI SDK 24；缺失时构建失败并打印安装提示。
- `cargo:rerun-if-changed` 覆盖 guest crate、`Cargo.lock`、`build.rs`。
- 不下载 release artifact，不读取网络。

### 2. 本地与 CI

- `just codemode-guest` 提供同样的源码构建路径；`just setup` 检查 WASI SDK。
- CI 用固定 WASI SDK 构建；release 二进制包含 wasm。
- 源码构建者需要 WASI SDK；不提供网络回退。

### 3. 嵌入与防篡改

- host 用 `include_bytes!(concat!(env!("OUT_DIR"), "/guest.wasm"))`。
- 源码不提交 wasm；`.gitignore` 忽略本地构建产物。
- 防篡改依赖源码 + `Cargo.lock` + 固定 WASI SDK；没有外部 artifact 可替换。
- 可选：构建后输出 wasm SHA256 到日志，便于审计；不作为运行时校验。

### 4. 失败与诊断

- 缺 WASI SDK：失败信息包含 `WASI_SDK_PATH`、WASI SDK 24、重建命令。
- guest 构建失败：保留 stderr 摘要；不得静默使用旧 wasm。
- `module_cache` 行为不变。

## 数据流

guest crate + WASI SDK
  -> build.rs / just codemode-guest
  -> OUT_DIR/guest.wasm
  -> include_bytes! -> Wasmtime Module

## Seams 与测试

- 构建测试：源码构建可复现；缺 WASI SDK 明确失败。
- host 测试：`include_bytes!` 使用 `OUT_DIR` 资产；`module_cache` 行为不变。
- 回归：`just codemode-guest` 仍可构建；guest ABI 不变。

## 风险

- 源码构建需要 WASI SDK；无 SDK 的开发者无法构建 host。分发二进制已包含 wasm，缓解终端用户。
- `build.rs` 递归调用 cargo 可能增加构建时间；用独立 target dir 与缓存缓解。
- WASI SDK 版本漂移改变 wasm；必须 pin 并记录。

## 未决问题

- `build.rs` 自动构建，还是要求先跑 `just codemode-guest`；需权衡构建自然性与递归 cargo 风险。
- WASI SDK pin 方式与 CI runner。
- 构建日志是否输出 wasm SHA256；不影响运行时。

## 迁移

- 实施时删除已提交的 `guest.wasm`，改为 `OUT_DIR` 嵌入。
- 不改变 guest ABI 与 runtime 行为。
