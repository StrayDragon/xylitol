---
change_id: c07-fix-bdd-scenarios
title: "修复 36 个 BDD 场景失败并补齐所有 gap"
depends_on: [c06-update-bdd-framework]
status: active
priority: 7
---

# 变更提案：修复 BDD 场景

## Why

`c06-update-bdd-framework` 完成了 cucumber-rs → rstest-bdd 迁移。当前 **41/77** 通过，**36 个失败**。cucumber-rs 从未真正运行过（`harness=false` 未配置），这些失败都是预存 gap——首次被 rstest-bdd 暴露。

## What Changes

### Root Causes (6 categories)

1. **SessionEntry `type` 字段冲突**: `EntryBase` 的 `#[serde(rename = "type")] entry_type` 与 enum `#[serde(tag = "type")]` 产生重复 key，导致反序列化失败 `"missing field 'type'"`。
2. **`{text}` 捕获带引号**: rstest-bdd 的 `{text}` 捕获包含步骤文字中的引号（如 `"hello world"`），导致断言字符串不匹配。需要改用 `{text:string}`。
3. **Session manager 未自动初始化**: session 场景缺少 `Background` 步骤，`sess.mgr` 为 `None`。
4. **步骤缺失/DataTable 解析**: `ls_with_files` 中 `"结果列出 {entry}"` 步骤未注册；edit multi-replace DataTable 未被正确消费。
5. **OR 子句未拆分**: `"not found" 或 "No such file" 或 "不存在"` 被当作整体字符串而非三个备选匹配。
6. **工具返回格式与 then 不匹配**: bash combined 返回整个 JSON 字符串，但 `_t_combined_has` 只用 `contains` 匹配。

### Fix Strategy

| # | Category | Strategy |
|---|----------|----------|
| 1 | `EntryBase.type` 冲突 | 添加 `#[serde(skip_serializing)]`，由 enum tag 提供 type |
| 2 | `{text}` 引号 | 所有 then 步骤 `{text}` → `{text:string}` |
| 3 | Session auto-init | 每个 session when/given 步骤自动检查并初始化 mgr |
| 4 | 缺失步骤 | 注册 `"结果列出 {entry}"` 步骤，修复 edit DataTable 解析 |
| 5 | OR clause | 在 `_t_call_fail_msg` / `_t_edit_failed` 中按 `或` 拆分匹配 |
| 6 | Bash combined | `_t_stdout_has` / `_t_combined_has` 解析 JSON 取对应字段 |

## Capabilities

- **bdd-tests**: BDD 测试步骤定义和断言修复

## Impact

- `tests/bdd.rs`: 修复步骤定义（主要变更）
- `src/infra/session/types.rs`: `EntryBase.entry_type` 序列化修复
- `tests/features/*.feature`: 无需修改
