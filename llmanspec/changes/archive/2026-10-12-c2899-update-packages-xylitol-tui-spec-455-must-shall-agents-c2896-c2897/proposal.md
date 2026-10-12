---
depends_on: []
needs_specs_change: false
title: packages/xylitol-tui 子树 spec 情态动词清整（对齐中文情态词表）
branch: sdd/c2899-update-packages-xylitol-tui-spec-455-must-shall-agents-c2896-c2897
base_branch: main
base_sha: 5247a4a8a4553b7d5e69000ee05164724bf1b0fe
---

## Why

> 注：合约编辑全部位于包 root `packages/xylitol-tui/llmanspec/specs/**`（上游 `specsLanded` 门为单 root 假设，只认主 `llmanspec/specs/`）；本票为纯词法清整、行为合约零变化，故 `needs_specs_change: false`，包 root 变更以本分支 commits 为准。

c2896/c2897 已全仓确立中文情态方向（约束句 必须/不得、MUST→须映射、GWT 步骤行无情态），但只清了 root `llmanspec/specs/**` 的 Top8 密度文件，「其余随触清」。实测存量：**455 处英文 MUST 全部集中于 `packages/xylitol-tui/llmanspec/specs/**`（18 个 capability）**——整个子树从未清整。同期上游 llman-sdd v0.7.5 模板已改为 free-text statement（无情态强制）+ 情态代称消歧（4c6f533），`llmanspec/AGENTS.md` 语言节已改写为 STC 对齐词表（必须/不得/禁止/宜；避免 应/应当/务必）。packages 子树与产品 BDD bindings 按步骤文本精确匹配，清整必须与 `tests/bdd/bindings_package_tui_*.rs` 同步改。

## What Changes

- TODO（propose 时锁）：packages/xylitol-tui 18 个 capability feature 的规则陈述行英文情态 → 中文情态；GWT 步骤行情态词清零；同步受影响 bindings 步骤文本。随触清原则不变——本票一次清完子树，此后回归「随触清」。
- 验收：`grep -rc MUST packages/xylitol-tui/llmanspec/specs --include=*.feature` 归零（引用原文/技术标识符除外）；`cargo test --test bdd` 全绿；`llman-sdd validate --all` 绿。

## 非目标（草案）

- 不改行为合约语义（纯词法）；不动 root llmanspec（已清）。
- 不处理 src 侧 specs（c2896/97 已覆盖，存量随触清）。
