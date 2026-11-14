---
depends_on: []
title: 删除孤儿活合约 agent-automation.feature
branch: sdd/c3337-remove-agent-automation
base_branch: main
base_sha: 5732fd0cad722f4a457f0bc5688c4339d80a2123
---

## Why

agent-automation.feature 落在 specs/ 根、无目录布局（违反 llmanspec/AGENTS.md 单轨布局）、无 BDD binding、无任何文档指针，但 llman-sdd validate 仍加载它——活着却无人测试、无人引用的行为合约。其 schedule/trigger 提交语义已被 agent-runtime 与 agent-goal 的现行 requirement 覆盖。

## What Changes

- 删除 llmanspec/specs/agent-automation.feature（孤儿根级散文件）
- 不新增替代合约：自动化提交语义由 agent-runtime / agent-goal 现行 requirement 承担

## Impact

- specs 数量 59 → 58；validate 仍全绿
- 无 BDD / 文档指针需要更新（已核为零引用）
