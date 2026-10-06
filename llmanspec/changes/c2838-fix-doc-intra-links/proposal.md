---
depends_on: []
---

## Why

c2837 公开化 `agent/infra` 后 rustdoc 扫描面扩大，28 处既有 intra-doc 链接缺陷暴露并以
crate-level `#![allow]` 临时兜底。其中含 **1 处真 doc 笔误**（`BootstrapOutput::warnings`
——该类型不存在，意图为 `ResolvedAssembly::warnings`）。本 change 逐处修复并移除 allow，
恢复 doc 链接质量与 rustdoc lint 完整性。

## What Changes

- 修复 broken-intra-doc-links ×18（补全正确路径；`id`/`scene` 转义字面量；`BootstrapOutput`→`ResolvedAssembly`）
- PRIVATE 类 ×10 改为代码字体文本（不扩公开面，见 research/findings.md 逐表）
- 移除 `src/lib.rs` 的两个 `#![allow(rustdoc::…)]`
- 验证：`cargo doc` 零 warning、`just qa` 全绿、validate 绿

## Out of Scope

- 不做「私有项语义公开化」升 pub（独立 API 决策）
- 不改 specs / BDD features
