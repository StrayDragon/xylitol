# Design

## 裁决（propose 定案；apply 按此实现）

| 通道 | 裁决 |
|---|---|
| bash / grep / find | omit 语义反转：**默认 120s**（= 现行 max 上限，不新造数字）。模型侧**移除 unlimited 表达能力**——r6「0 非无限」继续成立，且不再有任何哨兵表示无限。真正长任务的放宽走系统级配置（形状 apply 时定），模型不可自选无限 |
| read / write / edit / ls | 默认 **30s**，不暴露 schema 参数；仅系统级默认（防 fs 悬挂：NFS/FUSE） |
| Provider HTTP | connect **10s**；流式 chunk-gap idle **90s**（任何 chunk 重置，含首 byte 前窗）；非流式请求 total **300s**。常量内置，config 字段后置 |
| MCP 调用期 | per-request 默认 **120s**（rmcp `*_with_timeout` 或外层 timeout）；per-server 覆盖后置 |
| Hooks | 未配置 `timeout_secs` 时默认 **30s**；显式配置仍优先 |
| attach unary POST | 每方法有界：常规 **30s**；已知长操作（如 reload）分级放宽（值 apply 定）。spec 层钉「有界 + 分级」不钉具体数 |
| mux WS 下行 | client 每 **20s** Ping；连续两个周期无任何入站帧判定半开 → 走既有 resync/重订路径 |

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
