---
depends_on: [ "c3410-update-actor-execution-contract" ]
needs_specs_change: false
title: 插件单一脊柱：发现委派保留、清单 MCP 声明、私有工具桥退役
branch: sdd/c3430-refactor-plugin-discovery-mcp-convergence
base_branch: main
base_sha: 20f2e308cf554b81734388ea31fc4def92b21239
---

## Why

外部工具扩展现存双轨：config 声明的 MCP 与自研 stdio JSON-RPC 工具桥并存，编解码、命名、trust 语义长期双份维护；已知唯一外部插件 thoughtdag 实际只消费发现与委派，不消费工具桥。一步到位收敛为单一脊柱。

## What Changes

- plugins/bin 发现、describe 清单、CLI 委派保留；清单新增可选 mcp 声明（stdio command 与 args），自动注册为 MCP server，工具进入既有 mcp__ 管线。
- 私有 stdio JSON-RPC 工具桥（external_plugins）、plugin 前缀双下划线命名、桥级 trust 语义删除。
- 装配条款：config 来源的未配置零装配不变；插件来源采用安装即装配、config 可显式停用（信任门在 design 定稿，备选首用批准）。
- thoughtdag 由外部插件转为嵌入端：其 describe 清单与 install.sh 退役，改为独立二进制运行。

## Capabilities

- infra-plugin（实现）
- cli-plugin-command（实现）
- infra-mcp（实现）

## Impact

- 外部工具作者改实现 MCP server（各语言官方 SDK）；thoughtdag 双库同批适配。
