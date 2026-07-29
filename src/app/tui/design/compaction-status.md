---
version: "alpha"
name: "compaction-status"
description: "Context compaction status — busy Compacting + in-transcript compaction block (c493 / c1730)."
tokens_from: "../DESIGN.md"
components:
  compaction-line:
    textColor: "{colors.muted}"
  compaction-label:
    textColor: "{colors.muted}"
  compaction-body:
    textColor: "{colors.muted}"
---

# Compaction / AutoRetry status

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 忙碌短词与 spinner 通则见 [`status.md`](./status.md)。
> 可展开块通则见 [`expandable.md`](./expandable.md)（本块 **默认折叠**，与 tool 默认展开正交）。
> 静图：[`playground/`](./playground/) 槽 `compaction`。

对齐 pi：`CompactionStatusIndicator`（busy）+ `CompactionSummaryMessageComponent`（transcript 块）。

## MUST

### Busy status

1. **CompactionStart**：busy status = 单行 `Compacting`（既有 spinner）；**MUST NOT** 把压缩细节堆进 footer。
2. **CompactionEnd**：若仍 Busy → status 恢复 `Working`；若 idle → 清 busy。
3. **AutoRetryStart**：busy status = 单行 `Retry {attempt}/{max_retries}`。
4. **AutoRetryEnd**：`success=false` 时 scrollback 可追加失败说明；无论成败，若仍 Busy → status 恢复 `Working`。
5. **MUST NOT**：多行 status、百分比墙、footer 临时「正在压缩」文案。

### Transcript compaction block（c1730）

6. **CompactionStart**：在 transcript 插入一条 **占位块**（与最终块同槽位），折叠摘要示意进行中，例如：
   - 标签行：`[compaction]`（muted / 可加粗标签）
   - 折叠行：`Compacting…`（muted）
7. **CompactionEnd（成功）**：同一占位 **就地变成** 完成块（勿再叠一条「compaction complete」System 行）：
   - 标签行：`[compaction]`
   - **默认折叠**：`Compacted from {N} tokens ({expand-chord} to expand)`；`N` = `tokens_before`（千分位可选）
   - **展开**：`Compacted from {N} tokens` + 空行 + summary markdown（muted 正文）
8. **CompactionEnd（失败 / aborted）**：占位变成失败块或短 muted 一行（`compaction aborted` / 错误摘要）；**MUST NOT** 假装成成功 summary。
9. **默认折叠**；展开键与 tool 块共用产品面 expand 和弦（旁注括号提示，见 expandable）。重建 / resume 时已落盘的 CompactionEntry **MUST** 渲染为完成块（默认折叠），**MUST NOT** 退回旧的整段 System `[compaction] {full summary}`。
10. **MUST NOT** 用 footer 表达压缩进度或摘要；footer 只刷 token 数字（见 [`footer.md`](./footer.md)）。

### Footer tokens（同 change 范围）

11. Footer `used … tokens` 除既有时机外，**MUST** 在：turn 进行中有可用 Api usage 更新时（节流，**MUST NOT** 每 TextDelta encode）、以及 **CompactionEnd / TurnEnd** 后刷新。细则仍服从 footer.md 异步 / 不阻塞。

颜色：块标签与折叠/展开正文用 `{colors.muted}`；busy 短词走既有 spinner（`{colors.accent}` + muted 文案）。
