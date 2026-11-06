---
depends_on:
  - c3106-add-codemode-tool
title: 从源码构建并内置 codemode guest wasm
branch: sdd/c3219-update-codemode-guest-wasm-build-cache
base_branch: main
base_sha: 34c7067268cfaaa3487a62b352eb47629b0f5a9b
---

## Why

c3106 把约 880 KB 的 `guest.wasm` 提交进 git。它是 rquickjs / QuickJS 的 WASI 生成物，普通 `cargo build` 不能重建；当前只有手动 `just codemode-guest`，没有 hash 校验，guest 源码改动后可能继续嵌入旧 wasm。生成物不应作为源码长期维护；也不应让用户下载外部二进制。

## What Changes

- 源码树不提交 `guest.wasm`；`.gitignore` 忽略本地产物；源码保留 guest crate、`Cargo.lock`、构建 recipe、WASI SDK 版本。
- `build.rs` 自动从源码构建 guest 并嵌入 host 二进制；不下载 release artifact，不回退网络。
- `just codemode-guest` 作为手动构建命令 SSOT；与 build.rs 共用同一 target / env 参数。
- `build.rs` 在 `OUT_DIR` 生成 / 复制 wasm，host 用 `include_bytes!(concat!(env!("OUT_DIR"), "/guest.wasm"))`。
- `just codemode-guest` / `just setup` 提供同一套源码构建路径；固定 WASI SDK。
- 分发二进制已包含 wasm；源码构建者需要 WASI SDK，缺失时构建失败并给出安装提示。
- 测试：源码构建可复现；缺 WASI SDK 明确失败；guest ABI 不变。

## Capabilities

- `agent-codemode`
- `workspace-assembly`
- `test-qa-gate`

## Impact

- 仓库体积与二进制审查策略。
- CI / release workflow / `just setup` / `build.rs`。
- 不改变 guest ABI、runtime 行为或 `run_code` schema。

## Further Notes

调研见 `research/artifact-distribution.md`。c3106 的已提交 wasm 在本 change 实施时移除。
