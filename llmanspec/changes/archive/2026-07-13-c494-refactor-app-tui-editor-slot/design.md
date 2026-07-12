# Design — c494-refactor-app-tui-editor-slot

## EditorSlot 状态机

```text
                    ┌─────────────┐
         idle Esc×2 │             │ Esc / Enter travel→id
         ──────────►│ Tree(stub)  │──────────────────────► Editor
                    └─────────────┘
  Ctrl+P (MAY)      ┌─────────────┐
  ─────────────────►│ Plate       │ Esc ─────────────────► Editor
                    └─────────────┘
  /settings (MAY)   ┌─────────────┐
  ─────────────────►│ Settings    │ Esc ─────────────────► Editor
                    └─────────────┘
  Ask (MAY)         ┌─────────────┐
  ─────────────────►│ Choice      │ Esc/选择 ────────────► Editor
                    └─────────────┘
```

### Esc 优先级（硬）

| 条件 | Esc 行为 |
|---|---|
| 非 Editor 槽打开 | 关槽 → Editor（消费按键） |
| busy 且 Editor 槽 | `Driver::abort` + clear steer（ati10） |
| idle 且 Editor 空 | 双 Esc 窗内第二次 → 开 Tree stub |
| busy | **MUST NOT** 开 Tree |

## 模块 delta

```text
src/app/tui/
  effects.rs          # drain_pending — 生产 + harness
  commands.rs         # slash / bang
  bridge/{mod,handlers/**}
  layout/{root,theme,slots.rs}
  widgets/            # 既有；plate 文案可挂这里
```

## 与 agent_demo

对齐：**槽替换 + Esc 关槽**；不对齐：`TUI::start`、假队列实现、demo `/theme`。

## 撞名 / 并行

Agent 层 rename（`AgentRuntime` / `AgentCapabilities`）见另 change `c585`；本 change 不碰。
