---
depends_on: []
title: "Fix TUI remote driver link state"
---

# TUI 远程驱动链路状态修复

## Why

XyRemoteDriver 的下行链路把断链 / 重同步 / 致命三态压成一个 bool 派生（静态扫描，实施前复核）：

- **链路状态**（D2，remote.rs:128-136）：subscribed_ok/resync_needed/skip_cold_replay/downlink_gen/fatal 五字段；link_health 只看 subscribed_ok；fatal 仅 wait_subscribed 取用，之后永久 Down 无恢复通知；resync_needed 需外部消费否则静默丢失——致命态退化为「永远重连中」。
- **连接死亡三载体**（D16，http_ws.rs:47-54）：closed/last_err/dead/stop 四份表达同一事实，reset_mux 不清 last_err。
- **回合取消残留**（D17）：turn_cancel 无「回合结束必复位」，靠下次起手补偿。
- **观察态跳变**（D19）：AttachObserve Ended→Live 无显式复位，重订阅 seq>0 直接改 Live 跳过收敛，通知靠 prev/cur 比对，跳变即漏报。
- **历史窗口位置即状态**（D15）：has_more/has_earlier/entries[0] 类型三处表达，任何前插条目破坏判定。
- **Esc 双击窗口无单一所有者**（D14）：last_esc_at 与 suppress_idle_esc 分居两模块。

## What Changes

- 链路状态收敛显式枚举（Connecting/Live/ResyncNeeded/FatalClosed），fatal 可观测且可恢复（重连入口）。
- 连接死亡单一载体；reset_mux 全量复位；turn_cancel 回合边界复位；AttachObserve 显式转移。
- 历史窗口状态类型化；Esc 双击窗口单一所有者。

## Capabilities

- app-tui-host（attach 驱动）；关联 protocol-app（重同步语义）。

## Impact

- 范围：crates/xylitol-app-tui/src/driver/remote.rs、host_client/http_ws.rs、history_window.rs、layout/root/slot_nav.rs。
- 验证：断链 / 版本不匹配 / 重订阅场景 harness；fatal 后重连恢复测试。

## Open Questions

1. fatal（版本不匹配）是否提供自动降级重连，还是仅显式用户操作后重试？
