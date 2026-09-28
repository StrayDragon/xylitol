---
depends_on: []
branch: sdd/c2826-refactor-specs-compact
base_branch: main
base_sha: 7797b10c77e3847d55e12990d8649c3be1672570
---

# specs-compact：压降裸规则并补可执行场景

> **一句话**：对重点 11 capability 把裸规则转为嵌套可执行场景（含 bindings），合并 10 条语义等价规则，修复 1 处文件内重复 req id；规范行为不变。

## Why

specs 迁移原生分层格式后 pending（无嵌套场景的裸规则）共 596，重点 11 能力占 251。裸规则缺可执行示例，review pending 信号持续高位。本次按已批准的压缩计划（见 `research/compaction-plan.md`）压降，不改任何 MUST/SHALL 行为语义。

## What Changes

- **转场景 155 条**：为重点 11 capability 的裸规则补嵌套 `场景:`（英文 id，zh-CN 关键字假如/当/那么），并在 `tests/bdd/bindings_*.rs` 落地 `#[scenario]` 绑定（遵循仓库 451/452 场景有绑定的既有约定）；必要的少量新 step 落在对应 `steps_*.rs`。
- **合并 10 条**（移除 0 条，全部有承载）：
  - 能力内 6：r1242→r1280（ath）、r1218→r1224（atc）、r1380→r70（cli）、r1320→r1286（ati）、r1200→r1188（atm）、r1113→r1112（store）。
  - 跨能力 4：r1225→ath r1251、r1515→cli r1391、r1033→store r1094、r1034→protocol-app r1698。
  - 承载 req 文本补句 5 处：r1280（bang Esc 无 suppress）、r1251（+theme）、r70（+不复制第二套语义）、r1094（+创建/导入校验）、r1698（+XyEvent↔Event 映射句）。
- **换号修复 1 条**：package-ai-bridge 内 `@req:r1555` 重复挂载（thinking-level-resolve-exact 与 deepseek-prompt-cache-read-mapped）；后者换 **r1902**（原全局最大 1901）。
- **保留 84 条**裸规则：理由分四类（自带「由单测覆盖，MUST NOT 扩 BDD step」平版禁令 / 结构 seam 约束 / 真 PTY 依赖 / 负面闭集），逐条理由见计划表。
- 预期 pending：重点 11 能力 251 → ~85；全仓 596 → ~430（非重点 53 能力不动，留第二波）。

## Capabilities

app-tui-host、layer-architecture、app-tui-fixed-zone、app-tui-input、package-ai-bridge、cli-entry、infra-otel、agent-runtime、protocol-app、app-tui-commands、agent-session-store

## Further Notes

- 完整逐条决策表与映射表：`research/compaction-plan.md`（用户已批准）。
- 执行机制依据：嵌套场景按名经 `#[scenario(path, name)]` 绑定执行；`背景:` 必须紧跟 `功能:` 行。
- 归档 freeze 决策：archive 16 项 / 2.7M 均为近一月条目，不冻结。
