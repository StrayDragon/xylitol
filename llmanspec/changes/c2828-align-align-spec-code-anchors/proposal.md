---
depends_on: []
---

## Why

核对阶段（code-as-SSOT）：401 条裸规则中绝大多数是「证据在别处」的规则——(a) 规范自声明由单测覆盖、(d) 结构/流程契约（AGENTS/脚本/justfile 承载）、(b) 已被跨能力场景覆盖。这些证据目前只是口头声明，无机制核对。

## What Changes

- 配套 `scripts/check_spec_anchors.py`（已单独入 qa）：规格内 `# verified-by: <file|fn|script>` 锚点校验 + `--report` 对齐矩阵。
- 本 change 为全部 (a)/(b)/(d) 裸规则铺设 `# verified-by:` 锚点（指向承载单测的源文件、AGENTS 治理文档、justfile/scripts、或覆盖场景所在 feature）。
- 移除 cli-entry r67/r68（SlashCommandInfo/SlashCommandSource{prompt|skill} 零实现且与 pt3 冲突；产品命令面由 app-tui-commands r1203 SSOT 承载）。
- (c) 缺断言面规则（~30）保持 naked，留待批 2/3（幽灵规则清理与 harness 投入）。

## Scope

- `llmanspec/specs/**/*.feature`（锚点注释 + r67/r68 移除）

## Out of Scope

- 不改任何规则规范性正文；不改产品代码。
