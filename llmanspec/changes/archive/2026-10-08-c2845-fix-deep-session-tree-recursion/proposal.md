---
depends_on: []
needs_specs_change: true
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# 修复 v3 session_tree 深度递归崩溃（治本：RAW 透明 + 回退栈补丁）

## Why

深树会话 + `/session-tree` 让 serve 崩溃：`fory` 生成的 binary codec 对 `SessionTreeNode` 按树深**逐层递归**，debug 帧巨大（~44KB/层），~185 层即可让默认 tokio worker 栈（~2MiB）溢出 abort（core 栈实锤：1186 帧 `write_data → write_collection_data<Vec<SessionTreeNode>> → write_with_mode<SessionTreeNode>` 递归 ×169）。

上一版用 `thread_stack_size(64MB)` 加栈是**治标**：地雷没拆，只会延后（debug ~1400 层仍爆）。

## What Changes（治本）

1. `wire_v3::typed_payload` **把 `session_tree` 移出强 schema 白名单** → 应答走 `RawOk`，载荷与 JSON 轨**逐字面同构**（`{"tree":[...]}`），与 c2842 命令透明化同向。深度的 fory 递归从传输路径**整体移除**；客户端经既有 RawOk 解码路径拿到与 JSON 轨同一 JSON（serde 小帧，深度安全）。
2. **回退** `src/main.rs` 的 64MB worker 栈补丁 → 默认栈即可（根因不依赖栈大小）。
3. 强 schema `TreeResult` / `v3_to_tree_nodes` **保留**作兼容解码与旧端对拍（同 c2842 保 Command 的手法）。
4. spec `server-core` r1921（session_tree 应答透明与深度安全）+ 双轨对拍场景。

## Capabilities

- `app/server`（v3 应答塑形）、`wire v3`（方法形状）、`test-infra`（wire 对拍）

## Impact / 风险

- 仅 session_tree 应答形状变化（get_messages/travel 等 flat 方法保持强 schema）；客户端消费路径不变（serde 消费 `tree`）。
- 与 c2842 同为「形状透明化」方向的既有扩展，无协议外追加。
