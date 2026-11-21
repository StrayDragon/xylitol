---
depends_on: []
needs_specs_change: false
title: XyDriver 表面收窄：把端能力、attach 语义与 store 泄出清出驱动端口
---

## Why

XyDriver trait 现有 49 个方法（driver/proto.rs），消费者普查发现三类不属于「驱动回合执行」端口的面混入其中。表面不清就 actor 化，等于在脏表面上重构；本 change 作为 c3415 的前置，先一步到位清干净。

## What Changes

- 删剪贴板三件（copy_text_to_clipboard / stage_clipboard_image / read_clipboard_text）：runtime trait 仅有默认 unsupported 实现，真实实现与全部调用点都在 app-tui 自己的镜像 trait 与 client clipboard 模块（app-tui/src/driver/clipboard.rs、clipboard/）——端能力归端（src/AGENTS.md：剪贴板属 client 能力）。
- 删 attach / 远程语义六件（attach_session / link_health / attach_observe / take_resync_rebuild / drain_idle_events / refresh_surface_caches）：消费者全部在 app-tui 镜像（lib.rs:142,377,391,546,553；remote.rs 各实现），runtime 侧均为 no-op 默认。app-tui 镜像不动——那才是这些语义的家。
- session_store() 泄出收窄：方案在 design 定稿（候选：由 kernel 向客户端单独提供 store；driver 不再暴露 Arc<dyn XySessionStore>）。涉及 react 内部两处、kernel bootstrap 一处、app-tui 四处的改道。
- XyInProcessDriver 固有方法面（session.rs 约 30 个会话操作 + mod.rs 网关注入器）与 trait 的分层在 design 定稿：候选形状为 XyDriver（回合 / agent 面）与 XySessionOps（会话管理面）分列。
- 实施清单每项删除前重验「无真实实现 / 无真实消费者」（本次普查快照见 c3410 research/03）。

## Capabilities

无 live spec 变更（纯实现层表面重构，无可观察行为变化；实施普查若发现远端剪贴板等真实路径，升级为行为变更并转 needs_specs_change: true）。

## Impact

- app-tui 镜像 trait 与其实现不动；wire 路径不动。
- thoughtdag 走固有方法面，分层后其调用点随双库 lockstep 重排。
- 1,204 场景为回归地板，全程护绿。

## Open Questions

- 【已决 2026-11-20】session_store() 收窄：kernel 直供 store 给客户端，driver 不再暴露。
- 【已决 2026-11-20】固有会话操作面升格为独立 XySessionOps trait，回合面 / 会话面分列。
- 远程语义六件在 app-tui 镜像的保留确认（预期：保留，不动）。
- 剪贴板三件删除前的二次普查确认（防误删远端真实路径）。
