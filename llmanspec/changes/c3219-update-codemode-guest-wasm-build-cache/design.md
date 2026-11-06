# Design — 源码构建与发布资产分发 guest wasm

## 目标

- 本地开发从仓库源码自然构建 guest；不把 880 KB 生成物长期放源码。
- 分发资产不可篡改：SHA256 / provenance；CI 重建比对。
- 离线可缓存或本地路径；不在运行时下载。
- guest 可复现：固定 WASI SDK、`Cargo.lock`、构建 recipe。

## 现状

- guest 是独立 crate，`rquickjs 0.14` + QuickJS C 代码，target `wasm32-wasip1`。
- 需要 WASI SDK 24 提供 `clang` / `wasi-sysroot`；主 workspace 的 `cargo build` 不编译 guest。
- `guest.wasm` 已提交；`just codemode-guest` 手动重建；无 hash 校验。
- host 用 `include_bytes!("../assets/guest.wasm")` 编译期嵌入；`module_cache` 控制启动编译一次或每次运行编译。

## 决策

### 1. 源码构建优先

- 本地开发：`just codemode-guest` 用固定 WASI SDK 从 guest crate 构建；`just setup` 可自动确保 wasm 存在。
- 不把 wasm 提交进源码；`.gitignore` 忽略本地构建产物。
- 源码构建是首选路径；只有无法安装 WASI SDK 时才使用分发资产。

### 2. 分发资产

- CI 构建 guest，生成 `guest.wasm` + SHA256；发布为 release asset（文件名含版本 / hash）。
- 发布附 provenance / SBOM（工具待调研）；至少 SHA256SUMS。
- 分发资产按固定 hash 获取；校验失败不得静默回退。

### 3. 构建期解析

- `build.rs` 只解析已有 wasm，不递归调用 cargo：
  1. `XYLITOL_CODEMODE_GUEST_WASM` 本地路径（校验 hash）；
  2. 本地缓存（按 hash 命名）；
  3. 分发资产缓存 / 预取结果。
- 校验 SHA256 后复制到 `OUT_DIR/guest.wasm`；host 用 `include_bytes!(concat!(env!("OUT_DIR"), "/guest.wasm"))`。
- 找不到时给出 `just codemode-guest` / `just setup` 提示；不得静默使用旧 wasm。
- `cargo:rerun-if-changed` 覆盖 hash 清单与 build.rs。

### 4. 离线与分发

- 离线：使用缓存或 `XYLITOL_CODEMODE_GUEST_WASM`；不要求网络。
- 分发：release artifact 可预取到缓存；`just setup` 负责下载与校验。
- 不在 `run_code` 运行时下载或替换 wasm。

### 5. 可复现与防篡改

- pin WASI SDK 版本；记录 rquickjs / QuickJS 版本、target、profile。
- CI 从源码重建并与发布 hash 比对；不一致失败。
- 源码构建与分发资产都校验 hash；不提交未校验二进制。

## 数据流

本地开发:
  guest crate + WASI SDK -> just codemode-guest -> guest.wasm -> hash

分发:
  CI -> guest.wasm + SHA256 + provenance -> release asset -> just setup cache

host 构建:
  build.rs -> env / cache / artifact -> verify SHA256 -> OUT_DIR/guest.wasm
  -> include_bytes! -> Wasmtime Module

## Seams 与测试

- 脚本测试：源码构建 hash 一致；分发资产 hash 校验；无网络提示。
- CI：重建产物 hash 一致；发布资产校验。
- host：`include_bytes!` 使用 `OUT_DIR` 资产；`module_cache` 行为不变。
- 回归：本地 `just codemode-guest` 仍可构建。

## 风险

- 本地开发需要 WASI SDK；无 SDK 时走分发资产。
- 下载引入网络依赖；必须缓存 / 离线路径。
- WASI SDK 版本漂移改变 wasm；必须 pin 并记录。
- release artifact 被替换；SHA256 + provenance 是护栏。

## 未决问题

- artifact 托管：GitHub Release / 对象存储 / 本地缓存；需调研后定。
- `build.rs` 只校验本地文件、由 `just setup` 预取，还是允许 build.rs 下载；需权衡离线与构建可重复性。
- provenance / SBOM 工具与 CI runner；WASI SDK pin 方式。
- 版本命名：artifact 文件名与 hash 清单格式。

## 迁移

- 实施时删除已提交的 `guest.wasm`，改为 `OUT_DIR` 嵌入。
- 不改变 guest ABI 与 runtime 行为。
