# Design — codemode guest wasm 的构建与分发

## 目标

- 让 guest wasm 可重建、可校验、可追溯，避免“提交了 880 KB 但无法证明它对应哪份源码”。
- 保持 host 的普通 `cargo build` 不要求 WASI SDK。
- 明确离线、CI、发布三条路径。

## 现状

- guest 是独立 crate，`rquickjs 0.14` + QuickJS C 代码，target `wasm32-wasip1`。
- 需要 WASI SDK 24 提供 `clang` / `wasi-sysroot`；主 workspace 的 `cargo build` 不编译 guest。
- `guest.wasm` 已提交；`just codemode-guest` 手动重建；无 hash 校验。
- host 用 `include_bytes!` 嵌入 wasm；`module_cache` 控制启动编译一次或每次运行编译。

## 决策

### 1. 短期（本 change 可落地）

- 保留 `guest.wasm` 作为 vendored artifact，但增加 `guest.wasm.sha256`。
- 增加 `just codemode-guest-check`：重建 guest 并比对 hash；不一致即失败并打印重建命令。
- CI 跑该 recipe；hash 文件与 wasm 同提交。
- 记录构建元数据：rquickjs / QuickJS 版本、WASI SDK 版本、target、profile。

### 2. 中期

- 把 wasm 从源码树移到不可变 artifact 存储（release asset / 对象存储），文件名含内容 hash。
- host 构建或 `just setup` 按固定 hash 下载并校验；支持 `XYLITOL_CODEMODE_GUEST_WASM` 指向本地文件。
- 源码树只保留 guest crate、`Cargo.lock`、recipe、hash 与下载脚本。
- 离线 fallback：已缓存的 artifact 或显式本地路径；不得静默使用未校验文件。

### 3. 长期

- 发布 provenance / 签名；CI 产物可追溯到 commit 与构建环境。
- 可选 SBOM；不引入市场 / 动态插件语义。

## 产物与校验

- `guest.wasm.sha256` 是唯一校验真值；不得只比较文件大小。
- 校验失败时构建失败，错误信息包含期望 hash、实际 hash、重建命令。
- 不允许 host 运行时静默回退到旧 wasm；版本不匹配必须显式失败。

## CI / 本地 / 离线

- CI：固定 WASI SDK 版本，重建 guest，比对 hash；失败阻断合并。
- 本地：`just codemode-guest` 重建；`just codemode-guest-check` 校验；无 WASI SDK 时只能使用已校验资产。
- 发布：artifact 以 hash 命名；下载脚本校验后再放入构建缓存。

## Seams 与测试

- 脚本测试：hash 一致 / 不一致；缺失资产；无 WASI SDK。
- CI 测试：重建产物与提交产物一致。
- host 测试：`include_bytes!` 使用校验后的资产；`module_cache` 行为不变。

## 风险

- 下载引入网络依赖；必须提供离线路径与缓存。
- 二进制资产仍可能被人工替换；hash 与 CI 是主要护栏。
- WASI SDK 版本漂移会改变 wasm；必须 pin 版本并记录。

## 未决问题

- 长期 artifact 存储未定：GitHub Release / 对象存储 / 本地缓存。
- `include_bytes!` 是编译期；移除源码内资产后需 `build.rs` 或 `just setup` 把校验后的 wasm 放到固定路径；不能运行时下载。
- WASI SDK pin 方式与 CI runner 未定；需要可复现的 SDK 版本来源。

## 迁移

- 短期不改变 host 嵌入方式；中期改为 setup/build 阶段获取资产。
- 不改变 guest ABI 与 runtime 行为。
