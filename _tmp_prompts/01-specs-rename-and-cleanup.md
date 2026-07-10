# 提示词：llmanspec 命名规范化与腐烂清理（用后即弃）

> 交给快速 agent 执行；主线 agent 只审核 PR/diff。
> 仓库：`/home/l8ng/Projects/__straydragon__/xylitol`
> 约束：不要实现 `src/app/tui` 产品代码；不要改 `packages/xylitol-tui` 行为（除非为修 spec 引用）。

## 目标

1. 按 `llmanspec/config.yaml` → `rules.proposal` 落实命名与**中文**要求。
2. 清理干扰第四次 TUI 的腐烂 specs。
3. 产出可 review 的变更（最好单独 change 或 chore PR）。

## 必读

- `llmanspec/config.yaml`（刚更新的 prefix / 中文规则）
- 根 `AGENTS.md`「llmanspec 命名」
- `llmanspec/changes/c450-revise-app-tui-contract/`（合约方向）
- `_HANDOFF.md` 三轨说明

## 任务清单

### A. 删除 / 退役腐烂 ✅

- [x] `llmanspec/specs/diff-review/`：已删除 ✓。无 BDD/源码引用，c451 已声明「旧 diff-review 审批流另清理」。
- [x] 单体 `llmanspec/specs/app-tui/spec.toon`：已添加中文 purpose「跨切面不变量索引；细节见 app-tui-*」。未加新产品 req，未与 c450 delta 冲突。
- [x] 扫描 `tests/features/**`：无 TUI/diff-review 过时引用，无需修改。

### B. 命名迁移（映射表已提案）✅

- [x] `SPECS_RENAME_MAP.md` 已写入 `_tmp_prompts/`，按 P0/P1 优先级分组，等主线确认后再搬目录。

### C. 中文化 ✅

- [x] 已修改的单体 `app-tui` purpose 使用中文。
- [x] c450 delta specs（6 个 `app-tui-*` + 单体）的 purpose/statement/scenario 已全中文，不需再改。

### D. 校验 ✅

- [x] `llman sdd validate --all --strict --no-interactive` → **71 passed, 0 failed**
- [x] `llman sdd index rebuild` → 50 specs 重建完成

---

> 本 prompt 已完成使命。清理的 diff-review 目录不再存在；映射表待主线确认后执行 `git mv`。
> 后续步骤：合并 PR 前审核 `_tmp_prompts/SPECS_RENAME_MAP.md`，确认后逐批搬目录。

## 禁止

- 实现 TUI / 改 agent 行为
- 在未确认映射表前大规模 `git mv` specs
- 把 `diff-review` 需求合并进 `package-tui-diff`（二者不是一回事）

## 完成标准

- [ ] 映射表已写
- [ ] diff-review 已处理
- [ ] validate --all 通过（或列出与 c450 未归档相关的已知失败）
- [ ] 简短 PR 说明
