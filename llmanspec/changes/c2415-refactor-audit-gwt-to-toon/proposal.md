---
depends_on: []
branch: sdd/c2415-refactor-audit-gwt-to-toon
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: false
---

# 未绑定审计 GWT 迁回 toon（batch 1）

specs-compact 渐进票第一批：把「无 `#[scenario]` 绑定、从不执行」的 `.feature` 场景迁为 `spec.toon` 的 `feature:false` 文档行，然后删除对应 `.feature`。Partitioned SSOT 从此在批内恢复双轨纯净：`.feature` 只剩真执行场景。

## Why

全仓 823 行场景中 399 行（~48%）无绑定、从不进 harness——它们以可执行语法承载文档语义，误导「以为在跑」。本票按家族分批收敛；首批选 5 个小户验证迁移模式。

## What Changes

- 迁移并删除：`app-tui`（4）、`infra-diagnostics`（4）、`infra-git`（4）、`infra-process`（4）、`infra-observability`（4）共 **20 个场景**。
- 每场景转为 toon `scenarios[]` 一行（`feature:false`），scenario id 保留英文名。
- `infra-observability.feature` 的 `功能:` 名与目录名不一致（infra-provider-trace）问题随删除消解。
- 零行为变化：这些场景本就不执行。

## 非目标

bound 文件不动；其余 21 个 unbound 文件留待后续批次；场景内容改写。

## Impact

toon scenarios 计数：app-tui 1→5、diagnostics 0→4、git 0→4、process 0→4、observability 9→13；5 个 `.feature` 删除；harness 口径与实际执行一致化。
