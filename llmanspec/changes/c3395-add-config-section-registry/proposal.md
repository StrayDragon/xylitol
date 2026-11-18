---
depends_on: []
title: "配置节注册表：声明归插件、聚合归核心"
needs_specs_change: false
---

# 配置节注册表：声明归插件、聚合归核心

## Why

runtime-config r3070 明文「所有顶层配置节 须 由其所属插件声明（键名、schema、默认值）；核心 须 仅承担加载、合并与注册表聚合，不得 维护手写字段清单」，r2928 要求 schema 产物「由插件 section 注册表聚合导出」。现状反向：config 插件手写 24 节静态表（sections.rs:28-167），并依赖 7 个官方插件作类型枢纽（types.rs:671-674/816-818 re-export；Cargo.toml 依赖 hooks / loop-policies / todo / workflow / mcp / lsp / dap），超出 src/AGENTS.md「官方插件只依赖 plugin-sdk」的字面。新增一个插件配置节必须同时改 config 两个文件与 kernel 接线。

## What Changes

- sdk 新增 config_section 词表：纯数据 `ConfigSection`（key + schema/default 函数指针），无新增依赖；schemars 不得进 sdk。
- hooks / loop-policies / todo / workflow / mcp / lsp / dap 七插件各自声明节（schema / default 函数随类型走）。
- kernel 组装全量节清单（core_sections + 各插件声明）注入 config；加载校验与产物渲染消费注入清单；boot row 组装改直连插件类型；gen-config-schema bin 迁至 kernel，justfile 改指向。
- config 删除 7 个插件 re-export 与 Cargo 依赖；静态 SECTIONS 收缩为 core_sections()（核心域节 + 过渡节）；ConfigDomain 死列删除。
- 行为不变式：configs/config.schema.json 逐字节不变；未知键两阶段校验行为不变；外进程插件 claim 路径不动。

## Capabilities

- runtime-config：无条款变更（needs_specs_change: false）——本 change 是 r3070 / r2928 既有方向的实现推进，产品行为不变。

## Impact

- 代码：crates/xylitol-plugin-sdk、crates/xylitol-kernel、plugins/xylitol-official-plugin-config、上述 7 插件、justfile。
- 后续批次见 design.md（context_inject 双定义收敛、otel 归属、外进程 claim 并入、「未装配节」语义评估）。
- 决策 D2 / D3 / D4 按推荐默认采用（问询两次超时未获回答，apply 前可推翻），记录于 design.md。
