---
depends_on: ["c3415-refactor-session-actor-mailbox"]
needs_specs_change: false
title: ChatStateActor 与会话格式重定版：持久化 Ack 与 append-only 一等条目
---

## Why

状态与持久化独占化（参照 grok-build 的 ChatStateActor 形态），并落实 c3410 的会话格式重定版条款：append-only、不可变、注入条目一等公民，一步到位不留迁移码。

## What Changes

- ChatStateActor 独占会话状态与会话存储写路径；FlushAndAck 落盘屏障入 actor。
- 新格式落盘：append-only 维持、条目不可变、上下文注入条目一等公民；header 版本升版；LEGACY_SESSION_VERSION 迁移路径删除；未知行保留策略按 c3410 design 定稿实施。
- resume 与加载走新版本等值门；导出导入按新格式。

## Capabilities

- agent-session-store（实现）
- agent-runtime（实现）

## Impact

- 旧版本会话文件拒载（无迁移，产品未公开阶段可接受）；用户数据以新格式重新起会话。
