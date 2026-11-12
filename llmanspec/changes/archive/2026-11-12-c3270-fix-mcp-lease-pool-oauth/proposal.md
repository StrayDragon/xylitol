---
depends_on: []
title: "Fix MCP lease pool and OAuth terminal states"
branch: sdd/c3270-fix-mcp-lease-pool-oauth
base_branch: main
base_sha: 96d817748ab73ad52f1395237fdb1f0b1fa8e638
---

# MCP 租约池与 OAuth 终态修复

## Why

MCP 连接生命周期由计数器组 + 两张并行表拼出，扫描命中（证据为静态阅读，实施前复核）：

- **租约三元组**（hub.rs:28-73、sdk resource_lease.rs:99-128）：claims 计数 + idle_generation 代际 + reconnect_count，空闲过期也计入 reconnects（语义污染）；schedule_idle_close spawn 的任务无 JoinHandle，插件卸载后仍可触发；release 与 expire 之间无锁外不变量。
- **快照缓存五份独立更新**（hub.rs:100-111、603-654）：current/last_view/last_result/last_tools/notice 各自更新，try_lock 失败时合成「假就绪」视图，UI 与真实连接状态可长期不一致。
- **OAuth 双表无终态收敛**（client.rs:405-426、1078-1165）：flows 与 status 两张表并行，回调失败只删 flow 不写 status（永停 Pending）；无 gateway 时终态永不落地；expires_at_ms 已生成但无人检查。
- **连接进度无代际**（client.rs:375-392）：两次并发 connect 的 progress 互相覆盖。
- **跨会话门禁残留**（runtime in_process/mod.rs:77）：prepare_acked 在 switch_session 只清 freeze 不重置，新会话跳过 MCP 首轮门禁。

## What Changes

- 租约状态显式建模（Leased/IdleGrace/Closed + 单一转移入口）；空闲过期与重连计数分离；延迟关闭任务可取消（JoinHandle 或 abort token）。
- 快照缓存收敛单一更新点；合成视图标记「degraded」而非假就绪。
- OAuth 终态收敛：回调失败 / 超时 / 无 gateway 均落 Failed（或 Cancelled）终态；flows 与 status 合一或以 status 为唯一真值；expires_at 生效。
- 连接进度加代际；prepare_acked 随会话切换复位。

## Capabilities

- infra-mcp（准备门 / 重载 / 租约）；关联 agent-runtime（prepare 门禁复位）。

## Impact

- 范围：plugins/xylitol-official-plugin-mcp（hub、client）、crates/xylitol-plugin-sdk/src/resource_lease.rs、crates/xylitol-runtime/src/driver/in_process。
- 验证：租约并发获取/释放/过期单测；OAuth 失败矩阵（回调失败 / 超时 / 无 gateway）；切会话后 MCP 门禁再次生效 BDD。

## Open Questions

1. OAuth flows/status 合一还是 status 单真值 + flow 仅承载 channel？
2. 空闲过期是否保留 reconnect 计数语义分离后的观测指标？
