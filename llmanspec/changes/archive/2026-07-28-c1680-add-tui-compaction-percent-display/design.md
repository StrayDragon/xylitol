# Design: c1680-add-tui-compaction-percent-display

## 对照 pi（一手）

| pi | xylitol 现状 | 本 change |
|---|---|---|
| `getContextUsage()` → `percent = tokens/window*100` | 仅 `used N|~N|? tokens` | ✅ 派生 % |
| footer `{p}%/{formatTokens(window)}` | 无 window/% | ✅ 追加到 token 字段 |
| Heuristic 无单独 `~` on % | provenance 在 used 侧 | ✅ Heuristic：`~p%/W` |
| `(auto)` 当 compaction enabled | 无 | ⏳ **非 MVP**（避免扩 Driver；follow-up） |
| 色阶 >70 warning / >90 error | muted 单色 | ⏳ **非 MVP**（可后加 theme） |
| compact 后无 post-usage → `?%` | estimate 已有 Unknown/omit | ✅ 沿用 omit / `?` 规则 |

## 已决（关闭 draft Open Questions）

| 问题 | 决议 |
|---|---|
| 已用 % vs 距 reserve 余量 | **已用 %** = `tokens/window`；距 reserve **不做**主读数（触发仍看 reserve，展示不混） |
| footer vs status | **仅 footer** token 字段；MUST NOT 增高 status |

## 文案公式

```text
base = footer_token_label(provenance, tokens)   # 既有 c1035
if context_window > 0 && provenance != Unknown:
  pct = tokens as f64 / window as f64 * 100.0
  pct_s = format!("{pct:.1}")                   # 对齐 pi toFixed(1)
  win_s = format_compact_tokens(window)         # <1k 原样；<10k x.xk；否则 round(k)k；百万级 M
  if Heuristic:
    append " · ~{pct_s}%/{win_s}"
  else:
    append " · {pct_s}%/{win_s}"
elif context_window > 0 && Unknown:
  append " · ?%/{win_s}"                        # 有窗但 tokens 未知
else:
  # window==0 或不展示 %
  base only
```

示例：
- Api 42000 / 128000 → `used 42000 tokens · 32.8%/128k`
- Heuristic 100 / 128000 → `used ~100 tokens · ~0.1%/128k`
- window 0 → `used 42 tokens`（不追加）
- 空会话 omit 整段（既有）

## 数据源（无协议扩字段亦可）

```text
refresh_footer_tokens / kick_footer_token_refresh:
  est = driver.estimate_context_tokens()
  window = driver.current_model().map(|m| m.context_window).unwrap_or(0)
  label = footer_token_label(est.provenance, est.tokens, window)
```

**不要求**改 `ContextTokenEstimate` 结构（可选用：日后把 window 塞进 estimate 再收口）。

`format_compact_tokens`：纯函数，单测钉边界（999 / 1000 / 128000 / 1_000_000）。

## 硬隔离

- `should_compact` / reserve 公式 / CompactionSettings **零改动**。
- 配置 schema **无** `compaction_threshold` / usage_ratio 闸。
- harness：改完后 `should_compact` 单测仍绿；搜配置无百分比闸字段。

## 非目标

| 禁止 | 说明 |
|---|---|
| cache hit / CH% 同屏 | roadmap |
| 距 reserve 余量主读数 | 易与触发混淆 |
| status 多行 % 条 | atc9 |
| `(auto)` / 色阶 | follow-up，不挡本 change |
| 改 domain 触发合约 | c2/c16 只引用「若展示则派生」 |

## 验收 seam

| 锚点 | 覆盖 |
|---|---|
| derived-only | 有窗 → footer 含 `p%/W` 且 p≈tokens/window |
| no-percent-when-no-window | window=0 → 无 `%/` |
| heuristic-tilde | Heuristic → `~` 在 used 与 % 两侧 |
| no-threshold-config | 无 compaction_threshold 配置/字段 |
| trigger-unchanged | should_compact 行为与 c1630 一致（既有单测） |
