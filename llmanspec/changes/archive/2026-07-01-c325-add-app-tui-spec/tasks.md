# c325-add-app-tui-spec — Tasks

> 本变更是**纯规范沉淀**：把本次（2026-07）经充分对标调研确定的 TUI 架构决策写入 `llmanspec/specs/app-tui/`，为 c340/c350 等后续变更提供不可绕过的依据。无代码改动。

## 1. 撰写 delta spec

- [x] 在 `specs/app-tui/spec.toon` 声明 5 条 requirement（tui1 inline / tui2 in-process / tui3 三面并存 / tui4 复用契约 / tui5 不铺骨架），每条 statement 含 MUST/SHALL + 调研证据来源
- [x] 为每条 requirement 配至少一个 op_scenario（共 7 条）

## 2. 校验

- [x] `llman sdd validate c325-add-app-tui-spec --strict` 通过（spec deltas valid，仅余 tasks 勾选提示）
- [x] `llman sdd validate c325-add-app-tui-spec --strict --no-interactive` spec 部分输出 valid

## 3. 反降级护栏自查（proposal 末尾清单）

- [x] 5 条 MUST/SHALL 均带证据来源（ratatui 版本 / codex / pi 的 file 事实）
- [x] 后续 TUI 变更若违反上述 MUST，在 proposal 阶段即被识别为潜在冲突

## 完成定义

纯规范变更，validate 通过即完成。无 `just qa` / 无代码 / 无回归测试。归档时 specs/app-tui 合并到 main spec。
