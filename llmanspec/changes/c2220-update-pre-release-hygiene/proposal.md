---
depends_on: []
---

# 0.0.1 前卫生：无未发布兼容债

> **一句话**：发布 0.0.1 之前禁止为未发布面留 shim/别名/双写；及时删死码；写法对 LSP 友好。
> **目的地**：迭代中的仓库不像已发布库那样背兼容税。

---

## Why

产品未发布，却已出现兼容层惯性（弃用别名、双路径、`allow(dead_code)` 压住的预留）。未发布版本没有外部调用方需要保护；留桥只会让下一轮改名不敢动。巨文件与字符串 magics 同时抬高人类和 rust-analyzer 的成本。

根 `AGENTS.md` 尚未把「0.0.1 前禁止 unpublished 兼容」写成硬规则。

## What Changes

- 根 `AGENTS.md` Pre-0.0.1 卫生段（**本切片已写**）：禁止未发布 API/配置键/文案的兼容层；改名一次性改调用点；死码三类分诊；预留必须有落地条件。`src/AGENTS.md` 仅指针。
- 清扫本轮能删的真死 / 无消费预留（用既有 `audit-dead-code`）。
- 与 c2200 交叉：`UiEntry` 穷举优于 `_ => {}` 与工具名字符串表。
- **不**为行数而拆；harness 过大仅在明确编辑痛点时拆。

本票默认 **无产品行为合约** → 正式化时可 `skip_specs_landing: true`。

## 非目标

- 发布 0.0.1 本身
- 对外承诺稳定 Xy* 面（那是 1.0 的事）
- 删除仍有真实入口的能力（须分诊，不是一律删）

## Further Notes

调研：[`research/pre-release-hygiene.md`](./research/pre-release-hygiene.md)
追踪：`_HANDOFF/03-pre-release-hygiene.md`
Skill：`.claude/skills/audit-dead-code/SKILL.md`
