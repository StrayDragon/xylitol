# Design: Slash Commands 全集 + 派发

对齐 pi `core/slash-commands.ts` + AgentSession 派发。

## 决策

1. **破坏性替换，不做 BC shim**（遵循 AGENTS.md）：移除旧 `SlashCommandInfo`（7 字段常量）+ `BUILTIN_COMMANDS`，一次性更新所有调用方与测试。
2. **`SlashCommandSource` 三值枚举**：`Extension | Prompt | Skill`，对齐 pi。当前 extensions 不支持（用户要求），Extension 来源命令本期为空集，但枚举值预留以匹配 pi 语义。
3. **派发优先级**：`process_prompt` → 若以 `/` 开头先查命令表派发；未命中再走 LLM。`!`/`!!` 由 c95 的前缀路由优先处理（不进 slash 派发）。
4. **TUI 专属命令降级**：settings/scoped-models/changelog/hotkeys/trust UI 等，在非 TUI 模式返回 `NotAvailable { reason: "需要 TUI 模式" }`，不 panic、不静默。
5. **handler 接线**：`/export`→c105、`/compact`→session.compact、`/fork`→fork_session、`/tree`→navigate_tree、`/new`/`/resume`→session 切换、`/model`→select_model、`/quit`→退出。其余暂为 stub 占位。
6. **SourceInfo 统一**：复用 skills/resource 的 SourceInfo，使 `/get_commands`（RPC）能返回完整来源。

## 不做

- 不实现 TUI 交互式选择器（model/settings 选择 UI 属 TUI，用户已排除）。
