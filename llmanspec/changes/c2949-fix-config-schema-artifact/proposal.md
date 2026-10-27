---
depends_on: []
title: 配置 schema 产物与代码同源生成
---

# 配置 schema 产物与代码同源生成

## Why

`configs/config.schema.json` 是**手维护产物**：最近一次提交停在 2026-10-23，此后 `AppConfig` 新增了 `tool_batch`、`tui`、`otel`、`loop_example`、`session.max_turns` 等字段，schema 里全都没有。运行时校验走 `schema_for!(AppConfig)`（`plugins/xylitol-official-plugin-config/src/validate.rs`），所以**加载不受影响**；受影响的是编辑器补全、外部工具与文档读者——他们看到的是过期契约。

仓库已有同源生成能力（`app_config_schema()`），只缺「生成 + 门禁」这一步。

## What Changes

- 新增生成脚本（`scripts/gen_config_schema.py` 或等价 Rust bin）：从 `schema_for!(AppConfig)` 输出稳定排序的 `configs/config.schema.json`。
- 门禁：`scripts/check_config_schema.py --check` 比对「生成结果 vs 仓库文件」，漂移即红，接进 `just qa` 的 `check_*.py` glob。
- `just gen-config-schema` 维护 recipe（不进 qa，与 `gen-config-example` 同型）。
- 首次生成即修正当前漂移（补 `tool_batch` / `tui` / `otel` / `loop_example` / `session.max_turns`）。

## Out of Scope

- 改变 `AppConfig` 字段语义或默认值。
- 引入外部 schema 生成器（保持 schemars 单一来源）。
- 为 schema 增加人工注释字段（注释只进 `configs/example.yaml`）。

## Capabilities

- `runtime-config`：schema 产物同源 MUST + 场景（生成结果与仓库文件一致；漂移时门禁红）。
- `test-qa-gate`：新检查脚本必须接线进 `just qa`（沿用既有门禁纪律）。

## Impact

- 脚本：`scripts/gen_config_schema.py`、`scripts/check_config_schema.py`、`justfile`。
- 产物：`configs/config.schema.json`。
- 测试：脚本自检（生成幂等、漂移可检出）；BDD 或单测覆盖门禁接线。
- 文档：`configs/example.yaml` 顶部注释指向生成入口。

## Further Notes

来源：c2944 verify 的缺口 J。与 c2944 无行为耦合，可独立推进。
