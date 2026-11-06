---
depends_on:
  - c3106-add-codemode-tool
title: 用发布资产与构建脚本分发 codemode guest wasm
---

## Why

c3106 把约 880 KB 的 `guest.wasm` 提交进 git。它是 rquickjs / QuickJS 的 WASI 生成物，普通 `cargo build` 不能重建；当前只有手动 `just codemode-guest`，没有 hash 校验，guest 源码改动后可能继续嵌入旧 wasm。生成物不应作为源码长期维护。

## What Changes

- 从源码树移除 `guest.wasm`；`.gitignore` 忽略本地构建产物；源码只保留 guest crate、`Cargo.lock`、构建 recipe、校验与获取脚本。
- CI 用固定 WASI SDK 构建 guest，发布不可变 release artifact，附 SHA256 与 provenance。
- `build.rs` / `just setup` 按固定版本与 hash 获取 artifact，校验后放入构建缓存 / `OUT_DIR`；host 用 `include_bytes!(concat!(env!("OUT_DIR"), "/guest.wasm"))`。
- 不在运行时下载；离线使用缓存或 `XYLITOL_CODEMODE_GUEST_WASM` 本地路径。
- 先调研 artifact 托管、`build.rs` 网络策略、checksum / provenance 工具，再定最终实现。
- 测试：CI 重建 hash 一致；hash 不匹配失败；无网络时给出缓存/本地路径提示；guest ABI 不变。

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
