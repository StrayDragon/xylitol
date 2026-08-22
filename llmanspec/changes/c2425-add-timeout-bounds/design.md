# Design

## 裁决（propose 定案；apply 按此实现）

**权限模型（用户定案 2026-08-22）**：所有外部等待由**程序最高权限管理**——计时器必然武装、必然以 timeout 失败收场；模型侧输入至多是「请求」，被程序校验并钳制，不存在可表达的无限语义。全局兜底上界 **600 秒**（`MAX_EXTERNAL_WAIT_SECS`）。

| 通道 | 默认（差异化） | 模型/配置影响 |
|---|---|---|
| bash | **120s**（graduated kill） | 可请求更短，程序钳制 ≤120 |
| grep | **60s** | 同上 ≤60 |
| find(fd) | **60s** | 同上 ≤60 |
| read / write / edit | **30s**，无模型参数 | 仅系统级 |
| ls | **30s**，无模型参数 | 仅系统级 |
| Provider HTTP 流式 | chunk-gap idle **90s** | 无模型面；常量内置 |
| Provider HTTP 非流式 | total **300s** + connect **10s** | 同上 |
| MCP 调用期 | **120s**/request | per-server 覆盖后置，钳 ≤600 |
| Hooks | 未配置默认 **30s** | 显式配置优先，钳 ≤600 |
| attach unary POST | 常规 **30s**；reload 类分级放宽 | 分级表实现定 |
| mux WS 下行 | Ping 每 **20s** ×2 静默判死→resync | — |

补充：read/write/edit/ls 经 TypedTool `wait_bound` 统一包裹（`FS_TOOL_TIMEOUT_SECS=30`）；config 字段化后置（届时任何配置值钳 ≤600）。

## 合约影响面（landing 清单）

- agent-tools t24/t25 改写（omit 反转）+ doc 场景同步；新增 fs 四工具有界 req。
- infra-bash be8 改写（执行器层 omit 同步反转）。
- package-ai-bridge 新增 req：provider HTTP 三口径有界。
- infra-mcp 新增 req：调用期超时。
- app-tui-host 新增两 req：unary 有界分级、mux keepalive 半开检测。

## 兼容性核查（2026-08-22）

可执行 BDD 无冲突：`bash-timeout`（显式 1s）不变；`bash-omit-timeout-completes`（sleep 2s + omit）在默认 120s 下仍绿。src 内 `test_bash_omit_timeout_unlimited` 单测属 apply 阶段代码改动。

## 明确不做

模型侧保留 unlimited 表达；total timeout 用于流式请求；本票实现。
