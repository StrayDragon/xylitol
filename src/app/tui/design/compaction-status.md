---
version: "alpha"
name: "compaction-status"
description: "Context compaction / auto-retry status (c493)."
tokens_from: "../DESIGN.md"
components:
  compaction-line:
    textColor: "{colors.muted}"
---

# Compaction / AutoRetry status

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 忙碌短词与 spinner 通则见 [`status.md`](./status.md)。

## MUST

1. **CompactionStart**：busy status = 单行 `Compacting`；live scrollback 追加一条 muted System（含 `reason`）。
2. **CompactionEnd**：scrollback 追加 `compaction complete` 或 `compaction aborted`；若仍 Busy → status 恢复 `Working`。
3. **AutoRetryStart**：busy status = 单行 `Retry {attempt}/{max_retries}`。
4. **AutoRetryEnd**：`success=false` 时 scrollback 追加失败说明；无论成败，若仍 Busy → status 恢复 `Working`。
5. **MUST NOT**：多行 status、百分比墙、把压缩/重试细节堆进 footer。

颜色：scrollback System 用 `{colors.muted}`；status 短词走既有 busy spinner（`{colors.accent}` + muted 文案）。
