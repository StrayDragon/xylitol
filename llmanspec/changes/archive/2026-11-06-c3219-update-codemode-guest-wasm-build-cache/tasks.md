# Tasks

## 1. 源码构建

- [x] 1.1 新增 `build.rs`：检查 `OUT_DIR/guest.wasm`；缺失时用 `WASI_SDK_PATH`（WASI SDK 24）构建 guest crate 到独立 target dir，并复制到 `OUT_DIR`。
- [x] 1.2 构建命令设置 `CC_wasm32_wasip1` / `AR_wasm32_wasip1` / `CFLAGS_wasm32_wasip1`；与 `just codemode-guest` 共用同一参数。
- [x] 1.3 `cargo:rerun-if-changed` 覆盖 guest crate、`Cargo.lock`、`build.rs`；构建日志输出 wasm SHA256。
- [x] 1.4 缺 WASI SDK / 构建失败时给出 `WASI_SDK_PATH`、WASI SDK 24、重建命令，不静默使用旧 wasm。

## 2. 嵌入与仓库

- [x] 2.1 host 改为 `include_bytes!(concat!(env!("OUT_DIR"), "/guest.wasm"))`；删除 `assets/guest.wasm`。
- [x] 2.2 `.gitignore` 忽略本地 wasm 产物；guest crate、`Cargo.lock`、构建 recipe 保留。
- [x] 2.3 `just codemode-guest` 更新为源码构建 SSOT；`just setup` 检查 WASI SDK 24；CI / release 使用固定 SDK。
- [x] 2.4 `module_cache` 行为不变；guest ABI、runtime 行为、`run_code` schema 不变。

## 3. 测试与门禁

- [x] 3.1 构建测试：源码构建生成 `OUT_DIR/guest.wasm`；缺 WASI SDK 明确失败。
- [x] 3.2 host 测试：嵌入路径使用 `OUT_DIR`；`module_cache` 回归不变。
- [x] 3.3 回归：`just codemode-guest` 可构建；仓库不跟踪 wasm。
- [x] 3.4 `just fmt`、相关 clippy、`just test`、`just qa`；BDD 新场景绑定并全绿。
