# Design — Serve 网关相位编码收敛

## 0. 复核结论（2026-11-13，HEAD ce54131 逐条开文件核对）

4 点位全部成立、无已修复项；c3300 未触碰 `crates/xylitol-serve/` 任何文件；listen.rs 自 2026-11-03（ea03a75）零改动。

- A8：Phase 枚举（listen.rs:228-233）与 AtomicU8 + 三常量（:236/:247-249）双份，phase() 手工 match 含 `_ => Starting` 兜底（:361-367）；stopping 无枚举值，仅 healthz 内 Ready+is_shutting_down 组合产出（:523-527）；状态字符串 render_unavailable（:378-389）与 healthz（:504-534）两处独立产出——当前值一致，属漂移风险。
- A9：ServeLifecycle 枚举（lifecycle.rs:9-13）与 LIFE_* 常量（listen.rs:250-251）两套，`_ => Persistent`（:283）吞未来合法值；idle_gen（AtomicU64）与 lifecycle（AtomicU8）为两字段，点位所指即后者。
- A18：ReverseRpcResult::Timeout（occupancy/ws.rs:111）全仓无构造点；实际超时走 `timeout(...)` 的 `Err(_)` 臂（host.rs:872/876），`Ok(Ok(Timeout))` 不可达；引用仅 host.rs:876/:900 pattern 与 drop 模拟单测。
- A19：shutdown()（:65-70）与 Drop（:116-126）各自 mark_shutting_down + stop_graceful(30s 硬编码两处)；真实重复路径 = `xylitol serve` 子命令 shutdown 后 handle 再 drop（kernel/occupancy/subcommand.rs:57-58）；语义差：remove_registration 只在 Drop（spec server-core r94 要求优雅退出删注册文件）。

补充事实：server-core r64 已把 starting/ready/stopping/failed 四态钉为产品合约——本收敛是代码向已落地 spec 靠拢；`"stopping"` 字符串当前无任何测试钉住，须补单测；shutdown_signal（:200-226）纯信号路径不 mark_shutting_down，Stopping 转移点收口后自然覆盖。

## 1. 目标与边界

- 相位单一编码、状态字符串单点、死变体删除、收尾合一（needs_specs_change: false；对外字符串值 / 503 / retry-after 不变）。
- 边界：GatewayHoop 对 Ready 一律放行的现行行为**不变**（stopping 期间是否拒绝属产品决策，超出本单，如有需要另立 fix change）。

## 2. 决策摘要

| 项 | 方向 |
|---|---|
| A8 相位 | Phase 增 `Stopping`；`repr(u8)` + 单一 as_u8/from_u8 映射，删常量与手工 match；mark_shutting_down 上移为 Stopping 转移点（覆盖 shutdown_signal 路径）；from_u8 未知值显式错误（与 A9 同标准） |
| A8 字符串 | `status_str()` 单点，render_unavailable 与 healthz 共用；新增 stopping 字符串单测 |
| A9 lifecycle | ServeLifecycle 显式 u8 映射承载合一；删 `_ => Persistent` 兜底，未知值显式错误 |
| A18 | 删 Timeout 变体；host.rs:876/:900 pattern 改 `Ok(Err(_)) | Err(_)`；仓内无消费者已确认，本仓 xylitol-serve 为 workspace 内部 crate，仓外消费者不存在（无人依赖二进制仓内部 RPC 类型） |
| A19 | Drop 委托 shutdown() 或共用私有 finalize；remove_registration 保留在收尾路径（守住 r94）；30s 宽限提常量 |

## 3. 实施顺序

见 tasks.md T1–T5（死变体最小切口先行，listen.rs 同文件 T2→T3→T4→T5 接力）。

## 4. 测试边界

单测（相位转移、字符串产出、stopping 钉住、生命周期未知值）；server-core BDD healthz 值不变回归；停机后注册文件删除断言。
