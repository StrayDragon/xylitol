---
depends_on:
  - c2705-refactor-runtime-actor-api
---

# 塌 XyDriver 方法表：Command 为会话操作 SSOT

## Why

`XyDriver`（`src/app/core/driver/proto.rs`）约 50 个方法，与 `protocol::Command` 及 `app/core/dispatch.rs` 1:1 平行。dispatch 自称无状态 mapper。`XyRemoteDriver` 再把每个方法编一次 unary。加一个会话操作要改 trait + 两端实现 + Command + dispatch + host `dispatch_session_unary`（cognitive ~115）。Command 上仍有 `serde(alias)`（`path`/`session_id`/`queue_stats`）——Pre-0.0.1 禁止兼容别名。必须一次把「会话 RPC」收成 Command → 单一执行器，Driver 只留流/链路面。

## What Changes

- 会话级 unary：**只** `Command` → 共享执行（今日 dispatch + host 分叉合并）。
- `XyDriver` 缩到：attach/link、`run`/abort/steer 流、idle drain、固定区只读缓存、clipboard 等**面**能力。禁止再为每个 slash 加 trait 方法。
- in-process 与 remote 共用同一 Command 执行路径；remote 不再为每个方法手写 `unary_cmd`。
- 删除 Command 上全部 `serde(alias)`；wire 只留现行主字段名。一次性改调用点与测试夹具。
- `dispatch_session_unary` 从巨型 match 改为「解析 Command + 调执行器」，复杂度降到闸可维护。

## 非目标

- 不改 WebSocket 只下行、unary HTTP POST 信封。
- 不把 Prompt 的 EventStream 塞进普通 DispatchOutcome（仍由 run loop 持有流）。
- 不在本 change 删 `/debug` 或 `cycle_thinking_level`（`c2740` / `c2730`）。

## Capabilities

start 后 **必须**改 live spec：凡钉 `XyDriver::select_model` 等方法表、或钉 alias 字段的 RPC/slash 场景。能力域主要是跨面 Driver / host unary / 产品信封。**先不改** `.feature`（本目录仅规划壳）。

## Impact

- 库嵌入若直接调 `driver.select_model` 改为 `dispatch(Command::SetModel {…})` 或保留极薄 intrinsic。
- 远程客户端与 TUI 行为应保持；破坏的是 **Rust trait 形状** 与 **JSON 别名**。

## 本批依赖

`c2705`：Driver 不再直穿 Capabilities，塌方法时只有 Runtime + 本层。阻塞 `c2730`、`c2740`。
