# llman-sdd attach 语义与 base_branch 记录（c2841 人工改动的复盘）

## 结论

不是工具缺陷。`change attach` 与 `change start` 记录 `base_branch = 默认分支(本仓 main)` + `base_sha = merge-base(当前分支, 默认分支)`，
语义是**分叉源审计**，不是「收口目标」。收口目标由 finalize 的 `--into` 覆盖（默认回落 `base_branch`）。

## 源码依据（本机 llman-sdd 仓）

- `packages/core/src/change/lifecycle.ts::attachChange`：
  `const configuredBase = opts.base ?? defaultBranch(git);`
  `const baseSha = mergeBase(git, branch, configuredBase);`
  `writeBinding(... { branch, baseBranch: configuredBase, baseSha })`
  - `--base` 仅当与绑定分支不同才允许（`opts.base === branch` 抛错）。
- `packages/core/src/change/lifecycle.ts::finalizeChange`：
  `const target = opts.into ?? binding.baseBranch ?? defaultBranch(git);`
  - `--into <branch>` 是覆盖 merge target 的**正解**；finalize 还要求当前位于绑定分支。

## 对 PR 分支工作流的正确用法

| 步骤 | 命令 |
|---|---|
| 建 change | `llman-sdd change new <id>`（frontmatter 无需手写 branch/base_branch） |
| 绑定 | 在 PR 分支上 `llman-sdd change attach <id>`（自动记录 base_branch=main、base_sha=merge-base）——**不要**手改 base_branch |
| 落地/实现 | 在绑定分支编辑 specs/代码 |
| 收口 | `llman-sdd change finalize <id> --into <当前 PR 分支>`（target 覆盖为 PR 分支；默认 target=main 会误合并） |

## c2841 复盘

我在 c2841 收口前把 frontmatter 的 `base_branch` 手改为 `pr/2026-10-bdd-infra-and-contracts`。
- **必要性**：无（`--into` 已能达到相同效果）；这不是工具缺陷而是「没用对命令」——最终也是靠 `--into` 落地的。
- **副作用**：手改使归档审计元数据显示「分叉源=PR 分支」，与事实（分叉自 main）不符；validate 不报错，但审计时会有偏差。建议归档副本改回 `main`。
- 仓库历史（c2837/38/39）归档前 frontmatter 均为 `base_branch: main`，且仅存在于本 PR 分支 → 它们也是 `--into PR 分支` 落地的（或等价目标覆写），印证正确姿势。

## 工具层可改进点（llman-sdd 仓，非 xylitol 交付）

「直接在共享 PR 分支上开发 change 再就地收口」不是工具一等公民流程：attach 要求绑定分支 ≠ 默认分支（在默认分支工作会抛错），
base_branch 只能记默认分支，收口必须显式 `--into`。可作为 llman-sdd 上游 UX 建议：
- `attach` 支持 `--into-current`（收口目标=当前分支）并把 base_sha 仍记 merge-base(默认分支)；
- 或 finalize 在 `--into <当前分支>` ≡ 绑定分支时，允许 `base_branch` 保持默认分支记录（当前已如此，无需改）。
