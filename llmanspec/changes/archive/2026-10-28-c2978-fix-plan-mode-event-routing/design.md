# Design

## Evidence

- `crates/xylitol-serve/src/occupancy/host.rs:1164`：`let slot = host.slot(&session_id).await;` 在 dispatch 之前取槽。
- `crates/xylitol-serve/src/occupancy/host.rs:188-195`：`session_id_from_payload` 在 payload 无 `session_id` 时回退 `fallback_session`。
- `crates/xylitol-protocol/src/command.rs:49-51` / `:83`：`SwitchSession { session_path }` / `NewSession {}` 序列化后没有 `session_id`。
- `crates/xylitol-serve/src/occupancy/host.rs:1384-1414`：dispatch 成功后计算 `plan_mode_event`，并 `slot.append_and_push(&XyEvent::PlanModeChanged …)`。
- `crates/xylitol-app-tui/src/host/session_ops.rs:527` / `:644`：`apply_switched_session` 用 `mode_strip_for_entries(&entries)` 初始化 ModeStrip。
- `llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature:217`：ModeStrip「状态须仅由 `planModeChanged` 驱动，不得在端侧另造模式来源」。

## Problem shape

`planModeChanged` 的发布点是 `slot`，而 `slot` 是 dispatch 前的会话。对 `switch_session` / `new_session`，dispatch 前并不知道新会话 id，于是事件必然落到 fallback 槽。这不是“事件内容错误”，而是**事件落点错误**。

同时存在第二个来源：端侧在切换/恢复时从 entries 重新推导模式。两处同时存在时，单客户端看起来正常，多客户端/多订阅者才暴露。

## Options

### Option A：dispatch 后按新会话 id 重新取槽（推荐）

- dispatch 成功后读取 `driver.session_id()`（或 dispatch 返回值）得到新 id；
- `host.slot(&new_id).await` 取/建新槽，再 `append_and_push(PlanModeChanged)`；
- 旧槽不再收到该事件。

优点：改动局部、语义直观；事件进入新槽 journal，订阅者重连/重放也能拿到。
缺点：需要确认 dispatch 返回值在 `switch_session` / `new_session` 两种路径上都带新 id（当前 `DispatchOutcome::SwitchedSession` / `NewSession` 已带）。

### Option B：把模式状态放进 unary 响应，端侧直接应用

- `switch_session` / `new_session` 的响应增加 `plan_mode` 字段；
- 端侧收到响应后设置 ModeStrip，不依赖事件。

优点：少一次事件往返；切换路径确定性强。
缺点：与 spec「仅由 `planModeChanged` 驱动」直接冲突；其他订阅者（多客户端）仍拿不到状态，除非再补事件。需要改 spec 或保留双轨。

### Option C：A + 端侧初始快照合法化

- 事件仍走 A；
- 端侧在切换/恢复时允许用 entries 做一次初始快照，但后续只由事件驱动；
- 修改 `app-tui-fixed-zone` 条款，明确“初始快照”与“另造来源”的边界。

优点：保留恢复体验；避免订阅时序导致 ModeStrip 短暂为空。
缺点：spec 需要澄清；两条来源需要测试约束，避免回归成“事件不生效也能显示”。

## Recommendation

采用 **Option A** 作为事件路由修复；ModeStrip 初始来源单独决策：

- 若 Host 能在订阅时稳定补发一次 `planModeChanged`，则端侧可移除 entries 推导，完全符合现行 spec；
- 若订阅时序无法保证，则采用 Option C，并在 spec 写明“仅允许一次初始快照”。

建议先在 A 上补 serve 测试，再决定是否收紧端侧。

## Test seam

- serve 单测：构造两个 `SessionSlot`，分别挂 journal；触发 `switch_session` / `new_session`，断言 `planModeChanged` 只出现在新槽 journal，旧槽 `max_seq` 不变。
- 若采用 C：TUI harness 断言“切换后先有初始快照，随后事件覆盖”；事件缺失时不得永久依赖 entries。

## Risks

- `host.slot(&new_id)` 会为新会话建槽；若新会话尚无订阅者，事件只进 journal，依赖订阅重放。需要确认 journal 保留策略（当前默认容量 1024）足够。
- 旧槽若仍有订阅者，修复后不再收到该事件；这是期望行为，但需要回归测试确认没有其他消费者依赖旧行为。
- `switch_session` 的 dispatch 路径可能返回 `SwitchedSession(id)`，`new_session` 返回 `NewSession(id)`；两者都要覆盖。

## Open questions

1. ModeStrip 初始来源：Host 订阅时补发事件，还是允许端侧一次快照？（影响 spec 是否改）
2. 是否需要同时修 `todoUpdated` / `workflowUpdated` 的 switch/new 路由？当前它们由工具在会话内发布，不经过 switch/new，暂不在本 change。

## Decision (implemented)

- 采用 Option A：`switch_session` / `new_session` 成功后按新会话 id 重新取槽并发布 `planModeChanged`。
- 订阅绑定时补发一次由最新 `plan_mode` 推导的 `planModeChanged`（不写 journal、不耗 seq），使端侧模式条收敛。
- 端侧保留一次 entries 初始快照作为兜底；`app-tui-fixed-zone` 的 `r2956` 已写明事件到达后以事件为准。
- serve 单测覆盖事件只落新槽与订阅快照不耗 seq。
