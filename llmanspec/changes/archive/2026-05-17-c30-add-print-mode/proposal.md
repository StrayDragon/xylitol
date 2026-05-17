---
depends_on: [c15-add-cli, c25-add-agent-loop]
---

# c30-add-print-mode

## Why

Print 模式是 MVP 最先可验证的完整模式——非交互、stdout 流式输出 + 工具执行结果打印。不需要 TUI，用户可以立即验证 agent 是否正常工作。

## What Changes

1. 在 `src/interface/print.rs` 实现 Print 模式
2. 订阅 agent 事件流，将文本输出写入 stdout
3. 工具调用结果显示（工具名 + 简要结果）
4. 流式输出（逐字符/逐行刷新）
5. 与 CLI 分派集成（`--mode print` 或默认模式）

### 输出格式

```
> 用户输入的 prompt

Agent 文本输出（流式显示）...

[Tool: read src/main.rs] ✓
[Tool: bash cargo check] ✓ (2 warnings)

Agent 继续输出...

完成。
```

## Capabilities

- `print-mode`: 非交互 stdout 流式输出模式

## Impact

- `src/interface/print.rs` 从占位变为实际实现
- 仅新增 `console`/`owo-colors` 样式依赖（可能已在骨架中）
- 这是第一个端到端可验证的功能
- **始终编译**：无 feature flag，作为默认交互模式
