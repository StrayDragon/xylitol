---
depends_on: []
title: 配置 include：读取外部配置文件
needs_specs_change: true
---

# 配置 include：读取外部配置文件

## Why

magpie 连接器插件要把网关模型目录写成独立文件，不污染用户主配置；xylitol 当前只加载固定三层配置，无法读取插件生成的外部文件。

## What Changes

- 顶层配置新增 `include: [path, ...]`：每个文件作为**低优先级层**合并，当前文件覆盖同名字段。
- 相对路径相对包含文件所在目录解析；`~` 展开为 home；被包含文件同样渲染 `{{ secret.* }}`。
- 缺失文件 warn 且跳过；include 成环报错。
- `configs/config.schema.json` 与 `configs/example.yaml` 同步。

## Capabilities

`runtime-config`：新增 r3026（配置 include）。

## Impact

- `plugins/xylitol-official-plugin-config`：`AppConfig` 字段、loader 合并逻辑、单测。
- 无运行时行为变化（不 include 时行为不变）。

## 非目标

- 不做目录自动扫描（如 `models.d/`）。
- 不做远程 URL include。
- 不改分层顺序（全局 → 项目 → CLI）。

## 不承诺

- 被包含文件与主文件同名字段覆盖顺序之外的合并细节（数组替换而非拼接）由 `deep_merge` 语义决定，不在 spec 固化。
