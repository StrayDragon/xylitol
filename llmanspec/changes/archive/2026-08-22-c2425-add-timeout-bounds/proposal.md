---
depends_on: []
branch: sdd/c2425-add-timeout-bounds
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: true
checkpoint_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
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
| MCP client | 握手有 8s 超时（MCP_SERVER_CONNECT_TIMEOUT） | 连接建立后 `call_tool`/`list_all_tools` 直接 `.await`，**调用期无界**（rmcp 提供 `*_with_timeout` 变体未用） |
| Tokenizer HuggingFace 下载 | 裸 `reqwest::get(url)`（tokenize/mod.rs） | 默认 client 零超时；多 MB 文件慢网无限挂 |
| 配置 `!` 前缀 shell 插值 | `std::process::Command::output()`（config/value.rs）——**同步阻塞** | 命令悬挂 = 启动/配置加载永久卡死，且占死线程 |
| Hooks 脚本执行 | `timeout_secs` 可选；`entry_timeout` 返回 None 即裸等子进程 | 未配置超时的 hook 无限等（有配置则已有界） |
| TUI→Host unary POST | `http_ws` `.post(...).send()` 无请求级超时 | Host 半开连接（断网无 FIN）挂到 OS TCP 超时（~15min+） |
| WS mux 下行读循环 | `reader.next().await` 处理 Close/Err，但**无 idle 检测 / TCP keepalive** | 半开连接静默停摆：既不 Close 也不 Err，下行无声中断 |

已有界的点（无需动）：session 写盘为 async tmp+rename 原子写；reverse-RPC 应答等待有 1s 收包窗；hooks 配置了 `timeout_secs` 的路径已有界。

设计张力：LLM 流式响应不能设总超时（长回答合法跑数分钟），需要区分 **connect timeout** 与 **chunk-gap idle timeout**；工具类则适合「默认上限 + 单次调用可覆盖」，与现行 bash max-120s 体系对齐。

## What Changes

1. 内置工具统一默认上限：bash/grep/find 的 omit 语义从「无限」改为「默认上限（值待定）」；read/write/edit/ls 引入同体系超时（fs 操作通常毫秒级，默认上限防 fs 悬挂）。
2. bridge provider 客户端补 connect timeout + 流式 chunk-gap idle timeout（不设 total，避免杀合法长回答）；非流式请求可加总超时。
3. MCP client 连接后的请求/工具调用补调用期超时（rmcp `*_with_timeout` 或外层 `tokio::time::timeout`）。
4. 网络下载与配置命令补界：tokenizer 下载换带超时的 client；config `!` 前缀命令改异步执行 + 超时（解除同步阻塞占线程）。
5. hooks 未配置 `timeout_secs` 时给默认上限（保留显式覆盖与 opt-out）。
6. attach 载体补界：unary POST 请求级超时；mux WS 加 TCP keepalive / idle 检测触发既有重订路径。
7. 超时错误的可观察性：统一走既有 `XyToolError::Timeout` / 稳定 kind 语义，产品面可见「哪一步、等多久、怎么放宽」。

## 开放决策（propose 时裁决）

- 各类默认值与 config 形状：内置常量 vs `models.*` / `[tools]` / provider 字段级覆盖。
- SSE idle 判定口径：chunk 间隔阈值、首 byte 前后是否分级。
- MCP 调用期超时归属：client 统一常量还是 per-server 配置；hooks 默认值与 opt-out 形态。
- omit=unlimited 是否保留为显式 opt-out（文档化为「明确要求不限时」），还是彻底移除。
- mux idle 检测阈值与 keepalive 参数归属（client 侧 vs tokio-tungstenite 配置）。

## 非目标

改变 abort/Esc 产品语义（`插话续跑与中止.md`）；重试/退避策略；沙盒与权限。

## Impact

消除「某步出错即永久卡死」类故障面；行为合约变化涉及 agent-tools / infra-bash / package-ai-bridge / infra-mcp 相关 specs，正式化时按 SDD 全路径落地。
