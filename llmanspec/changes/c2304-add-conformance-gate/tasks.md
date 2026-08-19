# Tasks

测试边界：见 `design.md`。顺序 2 → 3 → 1。方案 A 不在本票。

## 1. 合约

- [x] 1.1 `llman sdd change start` 或手动 `sdd/c2304-add-conformance-gate` + `attach`；`base_sha` 钉本地默认分支 HEAD（勿用落后的 origin merge-base）
- [x] 1.2 live specs：`protocol-app`（ToolStart args；新 unary；reload/loaded_resources）；`server-core`（改 `sr-st1` + Host 接线）；`app-tui-bridge`（wire 往返后 path chrome）
- [ ] 1.3 commit Specs landing；`llman sdd validate c2304-add-conformance-gate --strict --no-interactive`；`readyToImplement=true`

## 2. 工具 chrome（先红后绿）

- [ ] 2.1 [blocked-by: 1.3] 失败测：`ToolExecutionStart`（read + path）经 `to_wire_event` → serde → `try_from` → `apply_xy_event`；`args_preview` 须含路径且不得停在 `Read ...`；对照直接 apply
- [ ] 2.2 `Event::ToolStart` 带 `args`（serde default）；双向映射；specta 重生
- [ ] 2.3 若流式 path 仍饿死：`MessageUpdate` 携带 toolCall；禁止新 Event 变体
- [ ] 2.4 可选：真 Host 推 Start，HttpWs 订阅后投影与 InProcess 相等

## 3. 方法表扩行

- [ ] 3.1 [blocked-by: 1.3] `UNARY_METHODS` + `Command` + `dispatch` + `command_from_method`：session 树 / travel / label / list / load_entries / new / 名 / delete
- [ ] 3.2 进程级 `reload`、`loaded_resources`（HostState 基线；只读不占写者）；`abort` 可取消进行中 reload
- [ ] 3.3 `get_state` 快照含 `leaf_entry_id`（或等价）
- [ ] 3.4 `XyRemoteDriver` 接线；去掉 unsupported / 空快照 / reload no-op
- [ ] 3.5 双 carrier 场景：新 unary 成功路径 + 未知方法失败 + 写者冲突；只读不占写者

## 4. Echo 换成真 Host

- [ ] 4.1 [blocked-by: 3.5] `InProcessClient` 对真 `HostState` + `writerToken`；Echo 不得过闸
- [ ] 4.2 旧 c2290 表与 3.x 新行同一套 InProcess vs HttpWs 双跑

## 5. 闸

- [ ] 5.1 `llman sdd validate c2304-add-conformance-gate --strict --no-interactive`
- [ ] 5.2 `just qa`
