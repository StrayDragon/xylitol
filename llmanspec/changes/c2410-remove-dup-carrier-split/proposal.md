---
depends_on: []
branch: sdd/c2410-remove-dup-carrier-split
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: false
---

# 去重：载体切分条款收敛到 app-tui-bridge

specs-compact 维护票：「产品 TUI 经四象限 attach；同进程路径留给 print/embed」这一载体切分事实目前在三处陈述——`cli-entry ce6`、`app-tui tui-cs`（句内）、`app-tui-bridge atb4`。收敛 canonical 到 **atb4**（唯一有可执行 BDD 场景 `remote-type-kept` 支撑），其余各自保留本位轴。

## Why

同一载体语义三处双写，attach 行为演进时需同步三点。c2405 已去重入口侧（tui2→cli-entry）；本票收掉剩余的载体侧簇。`server-core sr-remote1` 是 Host 侧义务（RemoteDriver 经信封），主体不同，不动。

## What Changes

- `atb4` 吸收 ce6 唯一条款「切换 MUST 只换客户端实现」，statement 收敛为载体切分单点真源。
- 删除 `cli-entry ce6`（requirement 行 + feature:false 文档场景行，无 `.feature`/binding 牵连）。
- `app-tui tui-cs` 删去句内载体复述（「产品 TUI MUST 经四象限客户端连 Host；print 与库嵌入 MUST 仍允许同进程」），保留面本地能力切分轴；文档场景 GWT 同步改写。
- 附带：`tests/bdd/bindings_app_tui.rs` 模块注释更新（现仅绑 bridge 场景）。

## 非目标

`sr-remote1`、协议/Host 层四象限条款；任何行为变化。

## Impact

cli-entry requirements/scenarios 22→21；app-tui 条款瘦身；app-tui-bridge statement 收紧；零代码行为差异。
