# c655 — 调研与延后说明（人工核对）

> **状态**：`paused`（2026-07-14）。不 apply、不 full。
> **位置**：`llmanspec/do-not-read-me/c655-update-app-tui-footer-context/`

## 1. 为何暂停

探索结论：footer 要展示的「已用 token」在 xylitol **今天没有足够可信、且随 tree travel 稳定的单一真值**。
启发式（chars/4）可做 compaction 闸，但不适合当默认产品数字；API `XyUsage` 有缺口（fake/部分流、abort、compact 后未知）。
在未选定数据源与诚实文案策略前落地，容易「看起来准、实际漂」。

## 2. 代码事实（xylitol）

| 组件 | 现状 |
|---|---|
| Footer | `format_footer_text(cwd, model, steer, follow_up)` → `cwd · model` + 可选 `q:sN\|fM`；**无 token 字段**（`src/app/tui/widgets/mod.rs`） |
| `UiRoot` | `refresh_footer_from_queue` 不读用量 |
| Driver / `SessionState` | 有 `model.context_window`；**无** used tokens / percent API |
| `ContextUsage` / `get_context_usage` | `agent/session/stats.rs`：给定 estimate + window → percent；**未接到 Driver** |
| `estimate_context_tokens` | `agent/compaction/token_estimator.rs`：**chars/4**；可选 last `XyUsage` + trailing |
| `XyUsage` | domain 有；OpenAI/Anthropic adapter 会填；Fake 常为 `None` |
| Tree travel | `travel_session_tree` + `rebuild_scrollback_from_travel` 按 **leaf 祖先路径** 重建 UI；用量若做，必须绑同一 path |

合约：`app-tui-chrome` `atc2` 仍为 MVP `cwd · model`。
设计文：`footer.md` / DESIGN Next wave 仍写 **c655 · context%**（与探索修订不一致；恢复前改文）。

## 3. pi 对照（`../pi`）

参考：`packages/coding-agent/src/modes/interactive/components/footer.ts`、`agent-session.ts#getContextUsage`、`packages/ai/src/utils/estimate.ts`。

| 行为 | pi | 备注 |
|---|---|---|
| 累计 ↑↓ / cache / $ | 扫 **全部** session entries 的 assistant `usage` | 账单感，不全等于「当前上下文」 |
| 当前上下文 `% / window` | `getContextUsage()`：依赖 `model.contextWindow`；对 **当前 branch messages** `estimateContextTokens` | compact 后可 `tokens: null` → 显示 `?` |
| Token 估计 | **chars/4**（+ image 常数）；**无** tiktoken | 与 xylitol compaction 同级启发式 |
| 人类友好 | `formatTokens`：`999` / `1.5k` / `15k` / `1.2M` | 可复用思路，与数据源无关 |

要点：pi 也承认估计；产品上用 `?` 表达未知。xylitol 若只显示「used N」而无未知态，更容易被当成精确值。

## 4. 探索中曾达成的产品修订（未写入 full specs）

讨论方向（相对原 purpose-draft「context% + 禁假 0%」）：

1. Seam：**Driver 薄只读 API**（不 Host 私自估、不先胖化 `GetState`）。
2. 展示：`· used <friendly> tokens`；**暂不做** `%` / window（依赖 model 额外配置）。
3. 语义：随 **MessageHistory leaf 路径** 变（travel 后必须更新）。
4. 允许「假显示」估计值 — 与原 proposal ethics 冲突；也是暂停主因之一。
5. **不**在本 change 引入 tokenizer crate（与 pi 一致；真 tokenizer 另开 future）。
6. 合约倾向：新 `atc13`，并改 `footer.md` / DESIGN 措辞。

## 5. 不准从何而来（恢复前必答）

1. **Chars/4**：CJK / 代码 / tool JSON 误差大；与厂商 tokenizer 不可比。
2. **仅 API usage**：无 usage 的消息、Fake、abort、流中断 → 0 或过时；compact 后 last usage 可能指压缩前。
3. **usage + trailing 估**：pi 同款，仍混「真 usage + 假 trailing」。
4. **百分比**：还依赖 `context_window` 配置是否诚实。
5. **Travel**：必须按 path 重算；若误用全 session 累加会与 scrollback 不一致。

## 6. 恢复触发条件（建议）

满足至少一条再移回 `changes/` 并 full：

- [ ] Driver 能提供 **文档化** 的用量：`Exact(u64)` / `Estimate(u64)` / `Unknown`，footer 文案可区分；或
- [ ] 产品接受明确标注的估计（如 `~1.5k` / `used ?`），并写入 `footer.md` MUST；或
- [ ] 引入与主路径一致的 tokenizer，且 travel/compaction 有回归证明。

仍建议 **Out of scope**：费用、↑↓ 分项、cache 率（可对标 pi 另开 change）。

## 7. 建议落地切片（恢复后）

1. 改 DESIGN/`footer.md`：context% → 选定文案与未知态。
2. `Driver::…` 只读 + 当前 leaf path 消息。
3. `format_footer_text` + travel/turn/compact 刷新。
4. Delta `app-tui-chrome` + harness（含 travel）。
5. **不**默认加 tiktoken，除非触发条件 3。

## 8. 相关路径速查

```
src/app/tui/widgets/mod.rs          format_footer_text
src/app/tui/layout/root.rs          refresh_footer_from_queue
src/app/core/driver.rs              SessionState / Driver trait
src/agent/session/stats.rs          ContextUsage, get_context_usage
src/agent/compaction/token_estimator.rs
src/app/tui/bridge/session_tree.rs  rebuild_scrollback_from_travel
../pi/.../footer.ts
../pi/.../utils/estimate.ts
```
