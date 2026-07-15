---
change_id: c1040-add-package-llm-types
title: "抽 xylitol-llm-types：消除 ai-bridge 双类型映射"
status: purpose-draft
priority: 1040
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: B
---

# c1040-add-package-llm-types

> **status: purpose-draft** — 待 c1030 归档且双类型映射成本被证实后再 promote。

## Why

c1030 选定过渡策略：包内 `Bridge*` DTO + 主仓映射到 domain。长期会导致字段漂移与样板成本。当映射维护成本高于一次性抽类型 crate 时，应把消息/块/usage 等共享契约外置。

## Purpose

1. 新建 `packages/xylitol-llm-types`（名称可微调）：承载 bridge 与主仓共用的消息/流块/usage 契约。
2. `xylitol-ai-bridge` 与主仓 `domain`（或 re-export）改依赖该包；删除过渡映射层（或缩成零成本 newtype）。
3. 保持 arch_guard：agent 仍不直接碰 vendor HTTP 类型。

## Capabilities（promote 时）

- `package-llm-types`（新）
- modify `package-ai-bridge` / 相关 domain 边界

## Out of scope

- 改 accounting 优先级语义
- 新 provider 方言

## Ethics

- risk_level: medium（大范围类型迁移）
- required_evidence: 映射层删除后编译与关键 BDD 绿；无主 crate ↔ bridge 循环依赖

## Depends

- **c1030-add-package-ai-bridge**
