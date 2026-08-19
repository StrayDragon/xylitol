# attach TUI 工具 chrome 与进程内对拍缺口

> 2026-08-20。用户截图：折叠栈 `Explored 1 file` 展开后工具行是 `Read ...`，正文已是 README。对照旧 TUI（同进程 InProcess）会把 path 补进 `...`。

## 产品症状

折叠组件没坏：无 path 时 `PATH_PLACEHOLDER` 本就是 `"..."`（`src/app/tui/bridge/preview.rs`，对齐 pi `renderToolPath`）。cluster 头 `Explored 1 file` 对 pathless read 也成立。缺的是 **path 从未到达 attach 路径上的 `XyEvent::ToolExecutionStart.args`**。

`read` 的 `ToolEnd.result` 是文件正文，不是带 `path` 的 JSON，所以 `extract_result_path` 无法在 End 回填。直播 Start 丢 args 后，done 态仍是 `Read ...`。

## 线协议丢字段（可复现）

`XyEvent::ToolExecutionStart { id, name, args }` → `Event::ToolStart { id, name }`（`to_wire_event` 用 `..` 丢掉 args）。

`Event::ToolStart` → `XyEvent::ToolExecutionStart { args: Value::Null }`。

第二刀：`MessageUpdate` 只映射 `text`/`thinking`，丢掉 `message`（含 `AgentPart::ToolCall`）。流式意图 chrome（atb10 / atb13）在 attach 上同样饿死；完成态 `Read` 只靠 Start.args 即可。

Host `append_and_push` 推的是 `to_wire_event()` 之后的 `Event`。产品 TUI attach 经 `XyRemoteDriver` `try_from` 还原。Print / 旧同进程 TUI 仍拿完整 `XyEvent`。

## 已有「对拍」是什么、不是什么

| 落点 | 实际覆盖 | 能否抓住本 bug |
|---|---|---|
| `activity_fold/scene.rs` `tool_start(id, name, path)` | 直接构造带 `args.path` 的 `XyEvent`，喂 `apply_xy_event` | 否：从不走 wire |
| `src/app/tui/tests.rs` `activity_fold_live_write_placeholder_is_editing_not_dots` | 渲染层：禁止 inflight write 显示 `Editing ...` | 否：手写 `UiEntry`，不经 Host |
| `src/protocol/wire/event.rs` 单测 | ThinkingDelta / QueueUpdate / Error / ContextToken 往返 | 否：**没有** ToolStart.args |
| `packages/xylitol-tui` snapshot / harness | 通用 TUI 引擎 | 否：零产品工具 chrome |
| c2304 原稿 | unary 方法表 InProcess vs HttpWs | 否：未写、且原非目标写「协议本身」 |
| `docs/architecture/远程体验与线协议.md` | 产品意图「远程与本地体验 parity」 | 文档；未闸 Event 字段 |
| `docs/roadmaps/Web与TUI同源.md` 「同源契约测试意图」 | Web↔TUI 词汇 | 未实现；也不是 InProcess↔attach |

**结论**：没有「旧 TUI vs attach」的跨进程对拍。现有绿测全部绕开 `to_wire_event`。

## 建议闸（本 change 第二切片）

先红后绿，不先改协议形状：

1. 同一条 `ToolExecutionStart`（read + `path`）直接 `apply_xy_event` vs `to_wire_event` → serde → `try_from` → `apply_xy_event`。后者 `args_preview` MUST 含路径，MUST NOT 停在 `Read ...`。
2. 双 carrier：真 Host 槽推同一 Start；InProcess 订阅与 HttpWs 订阅各自还原后喂 UiModel，preview 相等。
3. 然后才给 `Event::ToolStart` 补 `args`（serde default，Pre-0.0.1 一次性，无双解析）。MessageUpdate 的 toolCall 若同切片需要流式 path，一并过线；禁止新开 Event 变体。
