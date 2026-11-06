---
depends_on:
  - c3106-add-codemode-tool
title: 选择 codemode guest wasm 的可复现构建与分发策略
---

## Why

c3106 把约 880 KB 的 `guest.wasm` 提交进 git。它是 rquickjs / QuickJS 的 WASI 产物，普通 `cargo build` 不能重建；需要 WASI SDK 24、`wasm32-wasip1` 和 C 工具链。当前只有手动 `just codemode-guest`，没有 hash 校验，guest 源码改动后可能继续嵌入旧 wasm。

## What Changes

- 明确生成产物的分发策略，比较三条路径：
  1. **源码内 vendored 资产**：继续提交 `guest.wasm`，同时提交 `guest.wasm.sha256` 与构建元数据；CI 重建并比对 hash。优点是离线、构建简单；缺点是二进制 diff、仓库体积、来源审计弱。
  2. **CI 构建 + 不可变发布资产**：CI 从固定 `guest/Cargo.lock` 构建，发布带 SHA256 / provenance 的 release artifact；host 构建或 `just setup` 下载固定版本并校验。优点是不把生成物放源码；缺点是需要网络/缓存与离线 fallback。
  3. **build.rs 本地构建**：有 WASI SDK 时构建，否则回退预编译资产；灵活但构建环境复杂，CI 与本地行为易分叉。
- 推荐分阶段：短期保留源码内资产并补 `sha256` + CI 重建校验；中期把 wasm 移到不可变 artifact 存储，host 构建用 `XYLITOL_CODEMODE_GUEST_WASM` 或 setup 下载固定 hash；长期加 provenance / 签名。
- 源码只保留 guest crate、`Cargo.lock`、构建 recipe 与校验脚本；生成物不参与人工 diff 评审。
- 测试覆盖：CI 重建 hash 一致；hash 不匹配时构建失败并给出重建命令；无 WASI SDK 的 host 构建仍可使用已校验资产；离线 fallback 明确。

## Capabilities

- `agent-codemode`
- `workspace-assembly`
- `test-qa-gate`

## Impact

- `just codemode-guest` / `just setup` / CI workflow / release artifact。
- 仓库体积与二进制审查策略。
- 不在本 change 改 guest ABI 或 runtime 行为。

## Further Notes

行业常见做法是“源码可复现、产物可追溯”：生成物要么不提交，要么以 vendored artifact + hash + provenance 提交。800 KB 本身不是问题，问题是缺少可重建性与校验。
