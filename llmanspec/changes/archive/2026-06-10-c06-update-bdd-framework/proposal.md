---
change_id: c06-update-bdd-framework
title: "将 BDD 框架从 cucumber-rs 迁移到 rstest-bdd"
depends_on: []
status: active
priority: 6
---

# 变更提案：BDD 框架迁移

## Why

当前的 `cucumber = "0.23"` 存在以下痛点：

1. **regex 步骤匹配难以阅读** — `(\d+)` `([^"]+)` `(.+)` 等正则表达式散落在步骤定义中，可读性差，且无类型标注。
2. **脱离 `cargo test` 体系** — 必须通过自定义 `async fn main() { ToolWorld::run().await }` runner 运行，无法 `cargo test test_login_success` 精确执行单个场景。
3. **World 单体 struct** — 所有场景共享一个巨大 struct（约 20 个字段），字段之间互相污染。
4. **运行时才发现缺失步骤** — 编译时无法检测到未实现的 Gherkin 步骤。
5. **无法混合单元测试** — BDD 测试是独立二进制（`tests/bdd.rs`），和 `cargo test` 的单元测试割裂。

**rstest-bdd 解决了这些问题（类似 pytest-bdd 的 Rust 版）：**

- **Typed placeholder 替代 regex** — `{count:u32}` 可读、自文档、编译时校验。
- **`cargo test` 原生** — 每个场景是一个普通 `#[test]` 函数，支持精确过滤、IDE 点击运行。
- **Fixture 模式** — 按场景按需组合独立 fixture，取代 World 单体，无状态污染。
- **编译时步骤验证** — 通过 `strict-compile-time-validation` feature。
- **保留 .feature 文件** — 不改 feature 文件，只改 step 定义 + runner。

## What Changes

| 工件 | 操作 |
|------|------|
| `Cargo.toml` | 移除 `cucumber = "0.23"`，添加 `rstest` + `rstest-bdd` + `rstest-bdd-macros` |
| `tests/bdd.rs` | 整个文件重写：World → fixture，regex → placeholder，删除 `main()` |
| `tests/features/*.feature` | 无需修改（仅需确认 Examples 列名为英文） |
| `llmanspec/specs/test-infra/spec.md` | 更新 BDD 相关 requirement（若含 cucumber 特定约束） |

## Capabilities

- `test-infra` — BDD 测试基础设施

## Impact

- **测试代码量：** 1191 行 → ~1200 行（差异不大，因为步骤数量不变，但可读性提升）
- **运行方式：** `cargo test --test bdd` → `cargo test bdd`（统一为 `cargo test`）
- **精确调试：** 可以 `cargo test test_login_success` 跑单个场景
- **编译时间：** 无显著变化（rstest-bdd 编译量小）
