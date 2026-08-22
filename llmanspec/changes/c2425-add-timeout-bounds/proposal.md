---
depends_on: []
---

# 外部等待全链路 timeout 有界化

任何 agent 发起的外部等待（工具、模型网关、MCP server）都必须有界：出错时表现为可观察的超时错误并释放控制权，而非无限挂起。用户当前只能靠 Esc abort 兜底，且部分通道连 abort 之外的默认界都没有。

## Why（现状缺口，2026-08-22 实测）

| 通道 | 现状 | 缺口 |
|---|---|---|
| bash 工具 | `ToolTimeout` 可选，graduated SIGTERM→5s→SIGKILL | **omit 即无限**；无默认上限 |
| grep / find | 同 bash，omit → `pending()` 永等（仅 cancel token 可救） | 无默认上限 |
| read / write / edit / ls | 完全无 timeout 概念（本地 fs 通常快；NFS/慢盘/FUSE 可无限挂） | 无界 |
| Provider HTTP（bridge） | `reqwest::Client::builder()` 仅设 user_agent，**无 connect/total/idle 超时**——reqwest 默认无总超时 | 网络挂起 = 整轮挂起，最高风险项 |
| MCP client | 握手有 8s 超时（MCP_SERVER_CONNECT_TIMEOUT） | 连接建立后的工具调用/请求期未见超时 |

设计张力：LLM 流式响应不能设总超时（长回答合法跑数分钟），需要区分 **connect timeout** 与 **chunk-gap idle timeout**；工具类则适合「默认上限 + 单次调用可覆盖」，与现行 bash max-120s 体系对齐。

## What Changes

1. 内置工具统一默认上限：bash/grep/find 的 omit 语义从「无限」改为「默认上限（值待定）」；read/write/edit/ls 引入同体系超时（fs 操作通常毫秒级，默认上限防 fs 悬挂）。
2. bridge provider 客户端补 connect timeout + 流式 chunk-gap idle timeout（不设 total，避免杀合法长回答）；非流式请求可加总超时。
3. MCP client 连接后的请求/工具调用补调用期超时。
4. 超时错误的可观察性：统一走既有 `XyToolError::Timeout` / 稳定 kind 语义，产品面可见「哪一步、等多久、怎么放宽」。

## 开放决策（propose 时裁决）

- 各类默认值与 config 形状：内置常量 vs `models.*` / `[tools]` / provider 字段级覆盖。
- SSE idle 判定口径：chunk 间隔阈值、首 byte 前后是否分级。
- MCP 调用期超时归属：client 统一常量还是 per-server 配置。
- omit=unlimited 是否保留为显式 opt-out（文档化为「明确要求不限时」），还是彻底移除。

## 非目标

改变 abort/Esc 产品语义（`插话续跑与中止.md`）；重试/退避策略；沙盒与权限。

## Impact

消除「某步出错即永久卡死」类故障面；行为合约变化涉及 agent-tools / infra-bash / package-ai-bridge / infra-mcp 相关 specs，正式化时按 SDD 全路径落地。
