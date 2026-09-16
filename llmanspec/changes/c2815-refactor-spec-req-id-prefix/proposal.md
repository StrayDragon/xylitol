---
depends_on: []
needs_specs_change: false
---

# 全仓 spec @req 前缀迁移为 r<数字>（llman-sdd 0.1.3 validate gate 收紧）

## Why

`llman-sdd` 0.1.3 的 spec 解析器将 `@req` tag 识别正则收紧为
`REQ_TAG_RE = /^@?req:(r\d+)$/u`：**只有 `r<数字>` 前缀被认定是合法 req id**。
本项目 64 个 capability `.feature` 中大量使用语义前缀（`s1`、`ex1`、`sc1`、
`a1`、`p1`、`t1`…），导致全仓 845 条 `@human constraint scenario must carry
an @req:<req_id> tag` ERROR。该基线使任何 change 的 validate gate 恒红，
`readyToImplement` 恒 false，阻塞全部 SDD 生命周期。

本 change 是对全仓 spec 的一次**纯机械、零行为变更**迁移：仅改写 `@req` tag
前缀，不触碰任何 MUST/SHALL statement、验收场景步骤或 BDD 绑定（绑定按
`path + name`，与 req id 无关）。迁移后 validate gate 全绿，解锁后续全部
change。

## What Changes

- 对 `llmanspec/specs/**/*.feature` 全部 64 个 capability，将非 `r<数字>`
  前缀的 `@req:<旧id>` tag 稳定迁移为 `@req:r<新号>`；已合规的
  `r<数字>`（102 个，最大 76）原样保留。
- 新号由**全局计数器从 1000 起**分配，按语法序逐个映射，保证与既有
  `r<数字>` 及彼此都全局唯一（validate 存在全局查重 gate）。
- 同一 capability 内同一旧 id（规则与其验收场景引用）映射到同一新号，
  保证 file 内引用一致；新旧映射记录在
  `llmanspec/changes/<id>/research/req-id-migration-map.md` 供审计。
- 同步更新 `src/app/mod.rs` 中散引用 `@req:tt08` 的注释为新号。
- 不改变任何 statement / 步骤 / 场景名 / 标签分类；不启停 BDD runner。

## Capabilities

- `agent-session-store`：本文件内 `s1..s28`/`ex*`/`sc*`/`sp*` 等旧前缀迁移。
- 其余 63 个 capability 的 `.feature`：同样机械迁移（见 Impact 统计）。

## Impact

- validate：`just qa` 前置与 `llman-sdd validate --specs` 全绿（当前
  845 ERROR → 0）。
- 已绑定 change（c2810 等）：其 specs landing 分支需 rebase 到迁移后的
  main，并把分支内新写的 `@req:`（如 c2810 新增 s27/s28）改用 r 前缀。
- 无产品行为影响；git 历史可经映射表追溯旧 id ↔ 新号。

## Further Notes

- 根因与控制点：`llman-sdd/packages/core/src/spec/parser.ts` 的
  `REQ_TAG_RE`；本项目 `llmanspec/AGENTS.md` 的 req 规则用自然语言
  `@req:<id>` 描述、实例全为 `r<数字>`（r131/r135/r142），与新规范一致，
  无需改 AGENTS。
- 未来新 spec / 既有 `r<数字>` 后续新增，直接从现有最大号附近继续分配。
