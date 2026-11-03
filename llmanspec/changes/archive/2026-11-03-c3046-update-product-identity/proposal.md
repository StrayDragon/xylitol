---
depends_on: []
title: 产品对外身份改为 XYlitol
needs_specs_change: true
branch: sdd/c3046-update-product-identity
base_branch: main
base_sha: 63fafb8d90e32dc4d4d6fa31b737e8173d64c6c4
---

# 产品对外身份改为 XYlitol

## Why

给人看的名字、HTTP User-Agent 和 OTEL `service.name` 现在都是全小写 `xylitol`，和设计文档里的 `Xylitol Terminal` 也不一致。网关归因、观测看板和 TUI 头卡应当共用一个稳定的产品身份。

探索已拍板：对外拼法是 **XYlitol**（前两字母大写，突出探索坐标 XY），不是 Title Case `Xylitol`。命令、crate、路径、环境变量保持小写 `xylitol` / `XYLITOL_*`。这一次把展示面和对外指纹一起改完，避免后人再分两趟。

## 决策（探索已拍板）

- 对外 / 给人看的产品名 = `XYlitol`（XY 大写，其余小写）。
- 机器标识不变：二进制 `xylitol`、crate `xylitol-*`、`~/.config/xylitol/`、`~/.xylitol/`、`XYLITOL_*`、日志 target `xylitol::`。
- 范围一次做完：TUI 头卡、README / CLI about / 设计系统名、HTTP User-Agent、OTEL 缺省 `service.name`。

## What Changes

- TUI loaded-resources 头卡标题从 `>_ xylitol` 改为 `>_ XYlitol`（仍禁止木糖醇中文标签与 ASCII logo）。
- OpenAI 兼容与 Anthropic Messages 的 HTTP User-Agent 产品段改为 `XYlitol/<version>`，大小写与字面量一致。
- OTEL 未配置 `service_name` 时，`service.name` 缺省为 `XYlitol`；显式配置仍原样生效。
- README 标题与以产品为主语的散文、CLI `--help` about、`DESIGN.md` 系统名改为 `XYlitol`；命令示例仍写 ``xylitol``。
- 根 `AGENTS.md` 产品定调首次出现对照：展示名 XYlitol vs 命令 / crate `xylitol`。

## Capabilities

- `user-experience`：新增产品展示名 vs 机器标识的对照（给人看的必须是 XYlitol）。
- `app-tui-fixed-zone` r1223：头卡品牌字面量改为 `>_ XYlitol`。
- `infra-provider`：钉 UA 产品段 `XYlitol/<version>`（插件单测，不单独扩 BDD）。
- `infra-otel`：钉缺省 `service.name` 为 `XYlitol`（单测，不单独扩 BDD）。

## Impact

- `xylitol-app-tui` 头卡与 harness / 单测断言。
- 两个 provider 插件的 `default_user_agent` 与单测。
- obs 插件 OTEL resource 缺省、`configs/example.yaml` 示例、配置单测夹具。
- 根包 clap about、README、DESIGN.md、根 AGENTS 首次对照。
- BDD：r1223 增一条头部卡品牌场景；UA / OTEL 走既有单测缝。

## 非目标

- 不改 crate 名、仓库名、二进制名、配置/数据目录、环境变量。
- 不改 OTEL instrumentation scope / 日志 target（仍是代码身份 `xylitol`）。
- 不改 Harbor eval 类名 `xylitol_agent:XylitolAgent`。
- 不为产品名抽 `xylitol-plugin-sdk` 品牌模块；字面量在各产品面钉死。
- 不提供旧 UA `xylitol/` 双写或大小写别名。
- 不改模型 system prompt 自称（现行不自称产品名）。

## Open Questions

已解决：

- 拼法 = `XYlitol`，不是 `Xylitol` / `XYLITOL`。
- 展示面与对外指纹一次改完，不分期。

无未决项。
