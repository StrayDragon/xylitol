---
change_id: c457-demo-bash-mode-external-editor
title: "agent_demo：! bash 模式边框 + Ctrl+G 外部编辑器原型"
status: purpose-draft
priority: 457
depends_on: []
author: agent
track: A
---

# c457-demo-bash-mode-external-editor

> **status: purpose-draft**

## Why

产品后置 bash/外部编辑器，但库与交互需提前验证（边框色、键位不与 Editor 冲突）。

## Purpose

demo：`!` 前缀切 bash 边框色；Ctrl+G 触发外部编辑器钩子（可 stub `$EDITOR`）；DESIGN 草稿同步。

## Out of scope

- 产品 `Driver::execute_bash` 接线（c492）
