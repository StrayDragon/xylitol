# Tasks

测试边界：见 `design.md`。顺序 2 → 3 → 1。方案 A 不在本票。

## 1. 合约

- [x] 1.1 `llman sdd change start` 或手动 `sdd/c2304-add-conformance-gate` + `attach`；`base_sha` 钉本地默认分支 HEAD（勿用落后的 origin merge-base）
- [x] 1.2 live specs：`protocol-app`（ToolStart args；新 unary；reload/loaded_resources）；`server-core`（改 `sr-st1` + Host 接线）；`app-tui-bridge`（wire 往返后 path chrome）
- [x] 1.3 commit Specs landing；`llman sdd validate c2304-add-conformance-gate --strict --no-interactive`；`readyToImplement=true`

## 2. 工具 chrome（先红后绿）

- [x] 2.1 [blocked-by: 1.3] 失败测：`ToolExecutionStart`（read + path）经 `to_wire_event` → serde → `try_from` → `apply_xy_event`；`args_preview` 须含路径且不得停在 `Read ...`；对照直接 apply
- [x] 2.2 `Event::ToolStart` 带 `args`（serde default）；双向映射；specta 重生
- [x] 2.3 `MessageUpdate` 携带可选 toolCall message，恢复流式意图 path chrome；未新增 Event 变体
- [x] 2.4 可选项暂不扩 Host 事件投影：wire 往返对拍已覆盖公共边界，Host unary 双 carrier 对拍覆盖载体

## 3. 方法表扩行

- [x] 3.1 [blocked-by: 1.3] `UNARY_METHODS` + `Command` + `dispatch` + `command_from_method`：session 树 / travel / label / list / load_entries / new / 名 / delete
- [x] 3.2 进程级 `reload`、`loaded_resources`（HostState 基线；只读不占写者）；`abort` 可取消进行中 reload
- [x] 3.3 `get_state` 快照含 `leaf_entry_id`（或等价）
- [x] 3.4 `XyRemoteDriver` 接线；去掉 unsupported / 空快照 / reload no-op
- [x] 3.5 双 carrier 场景：新 unary 成功路径 + 未知方法失败 + 写者冲突；只读不占写者

## 4. Echo 换成真 Host

- [x] 4.1 [blocked-by: 3.5] `InProcessClient` 对真 `HostState` + `writerToken`；Echo 不得过闸
- [x] 4.2 旧 c2290 表与 3.x 新行同一套 InProcess vs HttpWs 双跑

## 5. 闸

- [x] 5.1 `llman sdd validate c2304-add-conformance-gate --strict --no-interactive`
- [x] 5.2 `just qa`
