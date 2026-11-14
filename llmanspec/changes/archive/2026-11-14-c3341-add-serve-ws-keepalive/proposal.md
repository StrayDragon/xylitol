---
depends_on: []
title: serve WS 心跳活性判定（防半开连接占槽）
branch: sdd/c3341-add-serve-ws-keepalive
base_branch: main
base_sha: 614079a56078586da0c446fbbc505f1dbbf2c4e9
---

## Why

审计中风险 L5：serve 对 WS 连接只回 pong 不主动 ping，半开的外部客户端（断电 / NAT 静默丢弃 / 不发 RST）订阅后永不消失，linger 不触发、会话写锁被无限期持有，其他端 subscribe 收 WriteHeld。产品 TUI 自带心跳不受影响；风险面是第三方客户端。用户确认按推荐方案修复。

## What Changes

- serve 新增 WS 心跳：每连接周期发 ping（缺省 20s，`serve.ws_keepalive` 配置，false/0 关闭），连续两个周期无任何入站帧 → 按非干净断开进入既有占用释放路径
- 配置面：`ServeConfig.ws_keepalive`、`xylitol serve --ws-keepalive-ms`、attach autostart/ensure 链路透传
- server-core spec 新增规则「WS 连接活性心跳」（r3355）
- listen 单测：静默判死、响应客户端存活、关闭时无 ping（paused time 确定性）

## Impact

- 正常客户端零行为变化（标准 WS 库自动回 pong）；静默连接从「永占」变为「40s 判死 → 60s 宽限 → 释放」
- `configs/config.schema.json` 同步再生
