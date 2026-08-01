---
depends_on: []
---

# Footer used 计数对齐紧凑 k/M 表达

> 窗口侧已有 `format_compact_tokens`（`128k`）；used 侧仍刷完整整数（`used 42000 tokens`），大会话 footer 臃肿。
> 本变更让 **used 计数与 window 共用同一套紧凑算法**（方案 A），provenance `~` / `?` 规则不变。

## Why

Footer 一行预算紧。`used 215686 tokens · 164.6%/131k` 一类文案挤占 cwd/model；窗口已紧凑、used 却全量展开，观感不一致。需要对齐表达，而不是另发明「五位保留」第二套阈值。

## What Changes

- `footer_token_label`：used 计数走既有 `format_compact_tokens`（`<1k` 精确；`1k–9.9k`→`x.yk`；`≥10k`→整 `Nk` / `NM`）
- 文案仍为 `used {compact} tokens` / `used ~{compact} tokens` / `used ? tokens`
- 更新 `design/footer.md` + live `app-tui-chrome`（atc2 / atc13 / atc21 示例）
- 单测 + harness 期望串同步（如 `42000` → `42k`）

## Capabilities

- `app-tui-chrome`（atc2 / atc13 / atc21）

## 测试缝（apply 用）

| 缝 | 断言 |
|---|---|
| `format_compact_tokens` / `footer_token_label` 单测 | Api `42000` → `used 42k tokens · 32.8%/128k`；`42` → `used 42 tokens`；Heuristic 保留 `~` |
| harness c1035 / c1680 类 | footer 含紧凑 used，不含未紧凑的大会话全量数字（边界用例） |

## Impact

- Footer 更短、与 `%/Wk` 同构
- 风险：精确位数读者需看 `~`/provenance；`<1k` 仍精确

## Out of scope

- 去掉 `tokens` 词
- 列宽感知动态切换
- Compaction scrollback `186,842` 千分位（另一套 `format_token_count`）
