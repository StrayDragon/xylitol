---
depends_on:
  - c3106-add-codemode-tool
title: 用源码构建与发布资产分发 codemode guest wasm
---

## Why

c3106 把约 880 KB 的 `guest.wasm` 提交进 git。它是 rquickjs / QuickJS 的 WASI 生成物，普通 `cargo build` 不能重建；当前只有手动 `just codemode-guest`，没有 hash 校验，guest 源码改动后可能继续嵌入旧 wasm。本地开发应能从仓库源码自然构建；分发应不可篡改；最简单方案可以接受“随仓库 build wasm”。

## What Changes

- 源码树不提交 `guest.wasm`；`.gitignore` 忽略本地产物；源码保留 guest crate、`Cargo.lock`、构建 recipe、校验与获取脚本。
- 本地开发优先从源码构建：`just codemode-guest` / `just setup` 用固定 WASI SDK 构建 guest；不依赖网络。
- 分发用不可变 release artifact + SHA256 / provenance；CI 重建并比对 hash。
- `build.rs` 只解析已有 wasm（本地 env / 缓存 / 分发资产）并复制到 `OUT_DIR`；源码构建由 recipe 负责，避免 build.rs 递归 cargo。
- 不在运行时下载；离线使用缓存或 `XYLITOL_CODEMODE_GUEST_WASM`。
- 先调研 artifact 托管、checksum / provenance 工具，再定最终实现。
- 测试：源码构建 hash 一致；分发资产 hash 校验；无网络时提示缓存/本地路径；guest ABI 不变。

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
