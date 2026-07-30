---
change_id: c1210-update-mcp-hot-merge-ungate
title: MCP connecting 不解闸输入；tools 热合并进下一轮请求
status: designed
priority: 1210
depends_on:
  - c1200-update-infra-mcp-startup
author: agent
branch: sdd/c1210-update-mcp-hot-merge-ungate
base_sha: 07d736d503d4d6339b14151e6450b13039a128d7
checkpointed: false
---

# c1210 — MCP hot-merge ungate

> 推翻 c1200 闸 A（connecting 拒 prompt/bang）；改走方案 B：只热合并 tools，下一轮请求带上。
> Prompt 模板体系延后见 **c1220**（主目标：prompt 管理重构 / 抽出可 eval system）。

## Why

启用 MCP 后，connecting 期间拒绝普通 prompt / bang，会挡住**根本不需要 MCP** 的工作。头卡已能表达进度；provider 每轮按请求携带 `tools`，settle 后热合并即可在**下一轮**生效。

c1200 已交付非阻塞 TTI、并行连接、头卡进度、settle 热合并骨架。本变更改「输入闸」+「合并去重 / 防重复进程」+ system 散文收紧 + **`/mcp` 发现面**。

## 已拍决策（2026-07-31）

| 项 | 决定 |
|---|---|
| 输入 | connecting 期间 **允许** agent prompt **与** bang |
| Slash | connecting 期间 **放开** `/reload` 等（删除 `when_mcp_connecting` 列或整列废弃） |
| Tools | settle 后 overlay；**下一轮** `run` 的请求 `tools` 带上 |
| SYSTEM | `Available tools:` **只列 builtins**；MCP 一句 discover + 可引导 `/mcp` |
| 发现面 | **`/mcp` 面板**（任意态）看列表/连接/armed；短 cue 可选引导；**不**在输入区堆名单 |
| 假消息 | **禁止**往 transcript 插「MCP ready」 |
| `/reload` | 放开 + MUST shutdown/替换旧 MCP 进程，防无法管理残留 |

## What Changes

- 改写 `mcp7` / `ath23` / `ath27` / `atm17` / `agent-prompt`（及 `.feature`）
- Host 去掉 connecting 输入闸；保留头卡进度
- `ToolSet` overlay + 单一 rebuild 入口；禁止裸 extend 叠 mcp
- settle / reload：先 shutdown 旧 manager 再装新
- system：builtins-only Available tools + MCP discover 一句（可引导 `/mcp`）
- **`/mcp` 面板**（任意态）+ 可选短 cue（禁止长名单 dump）

## Capabilities

- `infra-mcp`（mcp7）
- `app-tui-host`（ath23、ath27）
- `app-tui-commands`（atm16/atm17）
- `agent-prompt`（pt11）
- 可能触 `agent-runtime`（仅文档对齐 ar6）

## Impact

- 用户立刻可聊；MCP 晚到则下轮多 tools
- 风险：点名未就绪 MCP → 运行时无 tool（可短错）；头卡仍可感

## Out of scope

- c1220 prompt 管理大重构 / 模板引擎
- c1205 reload spinner 整包 UX
- 假 system 行
