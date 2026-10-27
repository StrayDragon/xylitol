# Tasks: c2953

> 完成判据中的计数/基线均在 change 分支（相对 merge-base）测量。

## 1. journal 事件分级（垂直切片：事件 → 写入 → 观测）

- [x] 1.1 `xylitol-serve` 新增 `is_tape_event` 穷举判定（`TextDelta` / `ThinkingDelta` / `MessageUpdate` / `ToolExecutionUpdate`）。
- [x] 1.2 `SessionSlot::append_and_push` 分流：tape 走 `push_tape`（广播 + 更新 `latest_tape`），事实事件走 journal。
- [x] 1.3 单测：tape 不增 `seq`、不写 journal；事实事件正常写。
- [x] 1.4 单测：既有 `append_and_push` 测试（`append_and_push_wraps_the_domain_event_as_session_event` 等）改用事实事件或显式断言 tape 行为。

## 2. `latest_tape` 与 `subscribe.partial`

- [x] 2.1 `SessionSlot.latest_tape: Mutex<Option<AgentMessage>>`，`MessageUpdate.message` 覆盖写入；`AgentEnd` 清空。
- [x] 2.2 `SessionSubscribedPayload` 增 `partial: Option<Value>`（`skip_serializing_if`）。
- [x] 2.3 `subscribed_payload` 填充条件：`inflight && last_seq > 0`。
- [x] 2.4 单测：`latest_tape` 覆盖式（不累积）；`AgentEnd` 清空。
- [x] 2.5 BDD：中段重连应答带 `partial` 且含当前文本；冷订阅不带。

## 3. `MessageUpdate` 字段收敛

- [x] 3.1 `xylitol-plugin-sdk::XyEvent::MessageUpdate` 删 `text` / `thinking`，只留 `message`。
- [x] 3.2 `xylitol-protocol::Event::MessageUpdate` 同步删除。
- [x] 3.3 `xylitol-wire` 映射同步。
- [x] 3.4 `react/assistant.rs::streaming_message_update` 只产 `message`。
- [x] 3.5 修所有构造点（`app-print` 测试、`app-tui` bridge 测试、`preview`、`harness`）。
- [x] 3.6 `PROTOCOL_VERSION` 4 → 5。
- [x] 3.7 单测：序列化结果只含 `message`；握手版本断言更新。

## 4. journal 容量

- [x] 4.1 `DEFAULT_JOURNAL_CAPACITY` 10000 → 1024。
- [x] 4.2 单测：环绕边界（1024 条后 `min_seq` 正确）；`c2952` 水位测试仍绿。

## 5. 落库先于 `MessageEnd`

- [x] 5.1 user pending 路径改序（`react/mod.rs`）。
- [x] 5.2 assistant 正常完成路径改序。
- [x] 5.3 assistant 中断路径改序。
- [x] 5.4 assistant middleware stop 路径改序。
- [x] 5.5 单测：`MessageEnd` 发出时 store 已可读到该条。

## 6. `get_messages` 分页

- [x] 6.1 `Command::GetMessages` 增 `limit: Option<usize>` / `before_entry_id: Option<String>`（`serde(default)`）。
- [x] 6.2 `registry` 解析同步。
- [x] 6.3 `SessionManager` 分页读取：从尾部向前，按 `manifest.sealed_segments` 定位，跳过整段。
- [x] 6.4 `xylitol-kernel` `command_exec` 透传参数。
- [x] 6.5 `xylitol-serve` `get_messages` unary 透传。
- [x] 6.6 单测：`limit=200` 取最新 200；`before_entry_id` 翻页连续无重叠无遗漏；末尾边界。
- [x] 6.7 BDD：首屏 200 → 翻页 → 提示行出现。

## 7. TUI 窗口与提示行

- [x] 7.1 `UiModel.entries` 窗口上限 800：超出时最旧部分替换为提示行。
- [x] 7.2 `UiEntry::ScrollNotice` 顶部加载提示行（居中、可点击）。
- [x] 7.3 `FoldTarget::LoadEarlier` + 点击处理触发翻页。
- [x] 7.4 首屏 200 / 单页 200 接线（resume / switch / import 路径）。
- [x] 7.5 单测：窗口 800 边界；短会话无提示行；提示行计数正确。
- [x] 7.6 BDD：长会话打开 → 只显示 200 条 → 滚顶加载 → 提示行出现/消失。

## 8. 端侧契约钉住

- [x] 8.1 `app-tui-bridge` 新增规则：折叠/展开为端侧纯呈现，不得往返 Host。
- [x] 8.2 `app-tui-bridge` 新增规则：产品 TUI（attach）不得直读 Host 存储。
- [x] 8.3 单测/契约：`XyRemoteDriver::session_store()` 为 `None` 断言。

## 9. spec 落地

- [x] 9.1 `server-core`：tape 分级、`subscribe.partial`、journal 容量 1024、`get_messages` 分页、落库先于 `MessageEnd`；改 `r1798` / `r1807` / `r1810` / `r2895` / `r2896`。
- [x] 9.2 `protocol-app`：`PROTOCOL_VERSION` 5、`MessageUpdate` 字段删除、`GetMessages` 分页参数。
- [x] 9.3 `agent-runtime`：落库时序。
- [x] 9.4 `app-tui-bridge`：折叠端侧、禁止直读、`r1174` 重连由 `partial` 恢复。
- [x] 9.5 `app-tui-fixed-zone`：窗口 800 / 页 200 / 提示行。
- [x] 9.6 `cli-print`：`r52` 回归确认（不改条款）。

## 10. 文档

- [x] 10.1 `src/AGENTS.md` 占用段补 tape 分级与 `seq` 不变量。
- [x] 10.2 `crates/xylitol-app-tui/src/AGENTS.md` 补端侧折叠与禁止直读。

## 11. 验证

- [x] 11.1 `cargo test --all-features --test bdd` 全绿。
- [x] 11.2 `llman-sdd validate c2953-... --strict` 绿。
- [x] 11.3 `just qa` 全绿。
