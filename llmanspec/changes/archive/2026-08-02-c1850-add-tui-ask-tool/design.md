# Design — c1850 TUI-only `ask`

## 等答缝

TUI 进程内：`ask` 工具 execute 挂起 → 发事件/回调打开 Choice 槽 → oneshot 收回 `ChoiceResult` → 工具返回 JSON。

- **选进程内 oneshot**：本工具仅 TUI 注册；无需本轮接 WS `AnswerQuestion`。
- Wire `AnswerQuestion` 留给多客户端；本 change MUST NOT 假装已对齐。

## 装配

| 面 | `ask` |
|---|---|
| 产品 TUI composition | 注册进 ToolSet |
| Print / 默认非 TUI | 不注册 |
| Trust bootstrap | 不走 `ask` |

## 双受众

| 受众 | 载体 |
|---|---|
| LLM | tool result JSON（`status` + `answers`） |
| 人 | editor 槽问卷 + scrollback 人话摘要（轨） |

桥接：Ask 结束写入专用 transcript 条目（或等价），MUST NOT 当普通 Tool 绿条/JSON 块。

## ChoicePrompt 合约要点

- Esc / Skip chrome → `ChoiceStatus::Skipped`（成功语义给 ask；Trust 仍把 cancelled/skipped 当 deny）
- Review 未答：Enter 武装 → 再 Enter Skip；← 清武装
- `description` 全缺 → 无右侧空「说明」栏

## 产品视觉

见 `src/app/tui/design/ask.md`：Ask 标题 accent；固定 rail；prompt↔选项空行。
