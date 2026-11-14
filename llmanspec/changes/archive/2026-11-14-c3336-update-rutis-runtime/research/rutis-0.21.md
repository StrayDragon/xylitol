# rutis 0.21.0 升级调研

- 日期：2026-11-14
- 范围：本仓 `rutis` / `rutis-loader` 精确锁 `=0.20.1` → `0.21.0`
- 一手来源：crates.io API（`/api/v1/crates/rutis`、`/api/v1/crates/rutis-loader`）；crate 源码 `0.20.1` 与 `0.21.0` 的 `public-api.txt` 与逐文件 diff；rutis 仓库 `docs/migration-0.21.en.md`、`docs/releases/0.21.0.en.md`。

## 版本现状

| crate | 本仓锁 | crates.io 最新 | 发布时间 |
| --- | --- | --- | --- |
| `rutis` | `=0.20.1` | `0.21.0` | 2026-11-06T09:10:35Z |
| `rutis-loader` | `=0.20.1` | `0.21.0` | 2026-11-06T09:11:06Z |
| `rutis-macros` | `0.20.1`（传递） | `0.21.0` | 随 `rutis` |

结论：有新版，且只落后一个 minor。0.21.0 是当前最新，无 0.21.x 补丁版。

## 0.21 的不兼容面（迁移指南三节）

1. **Rust `rutis::Error::Value` 改为结构体变体**：`Value(String)` → `Value { message, reason }`，新增 `Error::value()` / `Error::why()`；`data.reason` 取值 `non-finite` / `cycle` / `too-deep` / `type` / `tag` / `limit`。
2. **收到函数默认异步**：仅影响 Node / Bun / Python 插件；Rust 形态不变。
3. **错误文案改写**：codes 与 `data` 键是契约，文案不是；新增 `bad-result` 码，`bad-value` 增 `reason`，`no-method` 增 `provider`。

## 对本仓的逐项影响

| 0.21 变更 | 本仓是否触及 | 证据 |
| --- | --- | --- |
| `Error::Value` 结构体变体 | 否 | `rg 'Error::Value'` 在 `crates` 无匹配；本仓不构造也不匹配该变体 |
| 收到函数异步 | 否 | 本仓为 Rust host，无 Node/Bun/Python 插件 |
| 错误文案改写 | 否（无文案断言） | `rg 'no row provides it\|still waiting after\|already provided by'` 仅命中 kernel 自实现的 `describe_wait`（`crates/xylitol-kernel/src/boot.rs:488`），它按 `WaitReason` 枚举自行措辞，不复制 rutis 文案 |
| loader `runtimes` 特性下的状态/错误改动 | 否 | 工作区依赖 `rutis-loader = { default-features = false }`，未启用 `runtimes`；`Cargo.lock` 中 `rutis-loader` 无 `rutis-bridge` |
| `FiberView::running(Duration)` 新 API | 否（增量） | `rutis` `public-api.txt` 两版 diff 仅此一条新增；本仓未调用，无破坏 |

`rutis` 公开 API diff（0.20.1 → 0.21.0）只新增：

```text
+pub async fn rutis::host::FiberView::running(&self, core::time::Duration) -> core::result::Result<(), rutis::Error>
```

`rutis-loader` 的公开签名集合 diff 为空（无增删改签名）；源码差异集中在错误文案与 `runtimes` 分支的状态判定。

## 仓内需同步的常量与文档

| 位置 | 现状 | 动作 |
| --- | --- | --- |
| `Cargo.toml:59-60` | `=0.20.1` | 改 `=0.21.0` |
| `crates/xylitol-plugin-sdk/src/rt.rs:15` | `RUTIS_LOCK = "0.20.1"` | 改 `"0.21.0"` |
| `crates/xylitol-plugin-sdk/src/lib.rs:154` | 契约测试断言 `"0.20.1"` | 改 `"0.21.0"` |
| `src/AGENTS.md:130` | 「精确锁 `=0.20.1`」 | 改 `=0.21.0` |

`rg '0\.20(\.1)?'`（排除 `Cargo.lock` / `target` / `vendor`）只命中以上四处。

## 升级步骤

1. `Cargo.toml` 改两处精确锁。
2. `cargo update -p rutis --precise 0.21.0`（loader、macros 同法）。
3. 同步 `RUTIS_LOCK` 与契约测试、`src/AGENTS.md`。
4. `cargo check --workspace --all-features`（需 `WASI_SDK_PATH`）。
5. `just qa` 全绿。

## 风险与回退

- 风险低：编译面无破坏，错误文案改动不影响本仓断言。
- 回退：还原 `Cargo.toml` 两行 + `cargo update -p rutis --precise 0.20.1`，`RUTIS_LOCK` 回 `0.20.1`。

## 验证结果（2026-11-14）

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 编译 | `cargo check --workspace --all-features`（`WASI_SDK_PATH` 导出） | exit 0 |
| clippy（全 target） | `just lint-all` | exit 0（先修 4 处存量告警，见 `design.md` D5） |
| 全量门禁 | `just qa` | `QA_EXIT=0`；nextest 3858 passed / 54 skipped；spec validate 60/60 |
| SDD 严格校验 | `llman-sdd validate c3336-update-rutis-runtime --strict` | `valid: true` |
| lockfile 一致性 | `cargo metadata --locked` | exit 0 |

`Cargo.lock` 用 rustup `cargo` 重新生成后 diff 只有 `rutis` / `rutis-loader` / `rutis-macros` 三包的 version 与 checksum。首次用 mbx shim 的 `cargo` 生成时额外重解析了无关的 `windows-sys` 引用（包版本未变、仅依赖引用改写），已弃用该结果。
