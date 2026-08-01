# Design: c1780 busy instant lists

## 分流

```text
busy + Enter slash
  → slash_allowances.when_agent_busy
       Allow → 既有 pending 执行（可开槽）
       Reject → ScrollNotice refuse（不变）

busy + Resume 面板已开 + Enter(Select)
  → 第二闸：MUST NOT SwitchSession
  → push_scroll_notice(A)
  → 面板可保持打开（浏览继续）或保持选中态；MUST NOT 关面板冒充成功
```

文案 A（常量，单测可断言子串 `finish turn or Esc abort`）：

```text
agent busy — finish turn or Esc abort before switching session
```

## 实现落点（提示，非合约）

| 点 | 文件意向 |
|---|---|
| Allow 表 | `src/app/tui/commands.rs` `slash_allowances` |
| 去掉/改写「unavailable while busy」开槽拒 | `effects/slash.rs` 对 OpenModels / Theme / OpenSessionResume |
| switch 闸 | host drain `pending_session_resume_select` 时查 `run_active`/`phase Busy` |
| rename/delete 确认 | 同 busy 则拒（与 switch 同提示或专用短句；优先同 A 语义「等 turn / Esc」） |

## 非目标

- 不改 idle Resume 行为
- 不引入 Plate / `/hotkeys`
