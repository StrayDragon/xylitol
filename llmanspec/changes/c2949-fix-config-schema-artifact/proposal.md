---
depends_on: []
title: 配置 schema 产物与代码同源生成
---

# 配置 schema 产物与代码同源生成

## Why

`configs/config.schema.json` 是**手维护产物**：最近一次提交停在 2026-10-23，此后 `AppConfig` 新增了 `tool_batch`、`tui`、`otel`、`loop_example`、`session.max_turns` 等字段，schema 里全都没有。运行时校验走 `schema_for!(AppConfig)`（config 插件的 `validate`），所以**加载不受影响**；受影响的是编辑器补全、外部工具与文档读者——他们看到的是过期契约。

仓库已有同源生成能力（`app_config_schema()`），只缺「生成 + 门禁」这一步。

## What Changes

- 新增生成入口：config 插件内一个 bin（`gen-config-schema`），输出 `schema_for!(AppConfig)` 的稳定 JSON（键序确定、两空格缩进、末尾换行）到 `configs/config.schema.json`。
- 漂移门禁：config 插件内单测直接比对「生成结果 vs 仓库文件」，随 `just test` 进 `just qa`，不新增脚本、不额外编译一轮。
- `just gen-config-schema` 维护 recipe（不进 qa，与 `gen-config-example` 同型）。
- 首次生成即修正当前漂移（补 `tool_batch` / `tui` / `otel` / `loop_example` / `session.max_turns`）。

## Out of Scope

- 改变 `AppConfig` 字段语义或默认值。
- 引入外部 schema 生成器（保持 schemars 单一来源）。
- 为 schema 增加人工注释字段（注释只进 `configs/example.yaml`）。
- 让漂移门禁依赖网络或外部工具链。

## Capabilities

- `runtime-config`：schema 产物同源 + 漂移门禁 MUST + 场景（生成结果与仓库文件一致；产物含当前全部顶层配置节）。
- `test-qa-gate`：门禁经既有 `just qa` 测试序列生效，不新增独立脚本门禁。

## Impact

- crate：`plugins/xylitol-official-plugin-config`（新增 bin + 门禁单测）。
- 产物：`configs/config.schema.json`。
- 工具：`justfile` 增 `gen-config-schema`。
- 文档：`configs/example.yaml` 顶部注释指向生成入口。

## Further Notes

来源：c2944 verify 的缺口 J。与 c2944 无行为耦合，可独立推进。
