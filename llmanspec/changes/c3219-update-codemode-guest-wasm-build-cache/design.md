# Design — 发布资产与构建脚本分发 guest wasm

## 目标

- 源码树不保留 880 KB 生成物；wasm 由 CI 构建并作为不可变 release artifact 分发。
- host 构建按固定 hash 获取并校验；离线可缓存或本地路径。
- guest 可复现：固定 WASI SDK、`Cargo.lock`、构建 recipe。

## 现状

- guest 是独立 crate，`rquickjs 0.14` + QuickJS C 代码，target `wasm32-wasip1`。
- 需要 WASI SDK 24 提供 `clang` / `wasi-sysroot`；主 workspace 的 `cargo build` 不编译 guest。
- `guest.wasm` 已提交；`just codemode-guest` 手动重建；无 hash 校验。
- host 用 `include_bytes!("../assets/guest.wasm")` 编译期嵌入；`module_cache` 控制启动编译一次或每次运行编译。

## 决策

### 1. 资产生命周期

- 源码不提交 `guest.wasm`；提交 guest crate、`Cargo.lock`、recipe、获取脚本、hash 清单。
- CI 构建 guest，生成 `guest.wasm` + SHA256；发布为 release asset（文件名含版本 / hash）。
- 发布附 provenance / SBOM（工具待调研）；至少 SHA256SUMS。

### 2. 构建期获取

- `build.rs` 解析来源优先级：
  1. `XYLITOL_CODEMODE_GUEST_WASM` 本地路径（校验 hash）；
  2. 本地缓存（按 hash 命名）；
  3. 从固定 release URL 下载（仅构建期，不在运行时）。
- 校验 SHA256 后复制到 `OUT_DIR/guest.wasm`；host 用 `include_bytes!(concat!(env!("OUT_DIR"), "/guest.wasm"))`。
- 失败时给出期望 hash、实际 hash、缓存路径与 `just codemode-guest` 重建提示；不得静默使用旧 wasm。
- `cargo:rerun-if-changed` 覆盖 hash 清单与 build.rs；不因网络失败无限重试。

### 3. 本地与离线

- `just codemode-guest` 本地重建；`just setup` 预取 release artifact。
- 离线：使用缓存或 `XYLITOL_CODEMODE_GUEST_WASM`；不要求网络。
- 不在 `run_code` 运行时下载或替换 wasm。

### 4. 可复现

- pin WASI SDK 版本；记录 rquickjs / QuickJS 版本、target、profile。
- CI 重建并与发布 hash 比对；不一致失败。
- 不在源码中保留未校验二进制。

## 数据流

CI (WASI SDK + guest crate)
  -> guest.wasm + SHA256 + provenance
  -> release asset
  -> build.rs / just setup
       -> cache / local env
       -> verify SHA256
       -> OUT_DIR/guest.wasm
  -> include_bytes! -> Wasmtime Module

## Seams 与测试

- 脚本测试：hash 一致 / 不一致；缺失资产；无网络。
- CI：重建产物 hash 一致；发布资产校验。
- host：`include_bytes!` 使用 `OUT_DIR` 资产；`module_cache` 行为不变。
- 回归：本地 `just codemode-guest` 仍可构建。

## 风险

- 下载引入网络依赖；必须缓存 / 离线路径。
- `build.rs` 网络策略可能影响企业/离线构建；需明确可关闭。
- WASI SDK 版本漂移改变 wasm；必须 pin 并记录。
- release artifact 被替换；SHA256 + provenance 是护栏。

## 未决问题

- artifact 托管：GitHub Release / 对象存储 / 本地缓存；需调研后定。
- `build.rs` 是否直接下载，还是只校验本地文件、由 `just setup` 预取；需权衡离线与构建可重复性。
- provenance / SBOM 工具与 CI runner；WASI SDK pin 方式。
- 版本命名：artifact 文件名与 hash 清单格式。

## 迁移

- 实施时删除已提交的 `guest.wasm`，改为 `OUT_DIR` 嵌入。
- 不改变 guest ABI 与 runtime 行为。
