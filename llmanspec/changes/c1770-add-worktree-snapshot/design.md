# Git 工作树快照 / 差分（MVP 设计）

## 1. 设计边界

本 change 只交付 `infra-worktree-snapshot`：一个位于
`protocol::ports` 的可替换工作树快照契约，以及位于 `infra::git` 的 Git
实现。MVP 的调用方可以钉一个基线、重新采集当前工作树并得到 changeset，
但不会把它接入产品循环。

明确不在本 change：

- `AgentRuntime` 的 TurnStart / TurnEnd 自动接线、静默 capture 编排或配置开关；
- `XyEvent`、wire、session 记录、`/review`、review gate、评论回灌；
- TUI / Print / Server 产品入口；
- restore 的实际执行 API、工作树回滚 UI；
- jj/sl、文件 watcher、overlay/sandbox，以及新的 git2/gix 依赖。

因此「turn 级」在本 MVP 中是调用语义：未来消费者在 TurnStart 保存
`WorktreeSnapshot`，在 TurnEnd 调用 changeset；本 change 不拥有 turn
生命周期。

## 2. 代码事实与落点

| 代码事实 | 设计影响 |
|---|---|
| `protocol::ports` 是 agent ↔ infra 的替换边界，现有异步 port 使用 `async_trait`，并在 `ports/mod.rs` 与 protocol 根做精选 re-export | 新契约放在 ports；不让 protocol 依赖 `infra`，不向 wire 增加字段 |
| `infra::git::repo::find_git_repo` 已向上发现普通仓和 `.git` worktree 文件，并返回 worktree root、common git dir、HEAD path | 快照后端复用该发现逻辑，不再复制 `.git` 解析 |
| 当前 Git 模块只有发现、分支读取、URL 解析；Cargo 没有 git2/gix | MVP 通过 `tokio::process::Command` 调 Git CLI；参数逐项传入，禁止 shell 拼接 |
| `infra` 的外部命令实现已有异步进程模式；测试普遍使用 `tempfile` | Git 后端沿用异步命令与临时仓测试 seam |
| `infra-git` live spec 当前只覆盖发现、分支 watcher、URL 解析 | 本次不改 live specs；start 后新增独立 `infra-worktree-snapshot` capability，避免把快照语义堆进旧 capability |

## 3. 已锁定的契约形状

### 3.1 Port

新增异步、`Send + Sync` 的 `XyWorktreeSnapshot` port，最小操作为：

1. `capture`：依据后端配置从当前 worktree 生成一个不可变快照；
2. `changes_since`：以同一后端生成当前快照，计算给定基线到当前状态的
   changeset。

后端构造采用显式发现：`GitWorktreeSnapshot::try_new(cwd)` 在没有 Git
worktree、没有可解析 `HEAD` 或 Git 可执行文件不可用时返回
`Unavailable`，而不是注入一个会把「不可用」伪装成空 diff 的 no-op。后端
可通过 options 关闭 untracked 纳入；默认打开。

快照只携带后续计算所需的 opaque tree oid、仓库/worktree 身份和
`include_untracked` 策略。changeset 至少包含 baseline/current tree oid、
相对仓库根的结构化文件状态，以及 binary-safe 的 unified patch bytes。
文件状态保留 rename 的 old/new path 与 Git status code，不把路径压成
可能丢失空格、Unicode 或 NUL 的单行文本。

错误至少区分：

- `Unavailable`：非 Git、unborn HEAD、Git 不可执行或工作树已不可用；
- `ForeignBaseline`：基线来自另一个 worktree/repository 或策略不兼容；
- `StaleBaseline`：tree oid 已不能被当前仓库解析；
- `GitCommand` / `Io`：命令退出失败、输出无法解析或临时 index 失败。

错误不转换成空 changeset；空 changeset 只表示「可用 Git 仓中确实没有
差异」。

### 3.2 Git capture 原语

每次 capture 使用独立临时目录中的**不存在的** index 路径，并通过单个
`Command` 的环境变量设置 `GIT_INDEX_FILE`，不改用户 index、环境变量或
当前目录：

```text
git rev-parse --verify HEAD
git read-tree HEAD
git add -u                 # 仅把 tracked worktree 状态写入临时 index
git add -A                 # include_untracked=true 时再纳入非 ignored untracked
git write-tree             # 输出 tree oid
```

`git add -A` 的默认含义是纳入 Git 认为的非 ignored untracked；ignored
文件不进入快照，避免把构建产物或秘密文件静默送入 changeset。实现必须
使用临时 index，而不是 `stash`、全树复制或用户 index；Git 产生的临时
blob/tree 对象是 Git 原语的预期副作用，不挂持久 ref。

### 3.3 Git diff 原语

baseline 与 current 都是本后端产生并校验过的 tree oid。结构化摘要使用
NUL 分隔的 name-status 输出，patch 使用禁用外部 diff/textconv 的 binary
safe 输出：

```text
git diff --name-status -z <baseline_tree> <current_tree>
git diff --binary --full-index --no-ext-diff --no-textconv \
  <baseline_tree> <current_tree> --
```

两条命令都以参数数组执行。只对 tree 做 diff，不读取用户 index；解析失败
或 Git 非零退出都返回 typed error。

### 3.4 生命周期与存活

MVP 的 tree oid 只在进程内由调用方持有，不写
`refs/xylitol/...`，不承诺跨进程或跨长期 GC 存活。仓库被 GC、重置或
删除后再次使用基线时返回 `StaleBaseline`，不得降级为 HEAD 基线或空
changeset。未来若需要持久 review/session，再单独设计 ref 命名、清理和
并发保活。

## 4. Open Questions：现在钉死的答案

| 原问题 | 本 change 的答案 | 后续影响 |
|---|---|---|
| 评论回灌用 `steer` 还是 `ReviewContinue`？ | **不在本 change 决定，也不预留协议事件。** review consumer 另开 change；届时按 review 是否新 turn 决定 `steer` 或专用命令 | 本 change 只返回数据，不触碰 agent queue / wire |
| restore 用什么命令、如何清理 untracked？ | **MVP 不实现 restore 方法。** 这是有破坏性的工作树写操作；在未来入口前单独锁定 temp-index、路径冲突、删除范围和确认语义，禁止现在放一个未定义的 `restore` 占位 API，禁止用 `git clean -fdx` 旁路 | proposal 中的 restore「能力位」保留为后续扩展方向，不等于本 MVP 暴露入口 |
| 是否拆成 infra 与 agent/app 两个 change？ | **现在就按两个 change 的边界规划。** 本 change 只做 infra port + Git adapter；Turn consumer、review gate、UI、回滚入口各自后置 | 当前 `depends_on: []`；后续消费者依赖本 change 归档后的契约 |
| tree oid 是否挂 ref 防 GC？ | **MVP 不挂 ref。** 只在内存持有，跨进程不保证；失效明确报 `StaleBaseline` | 持久化快照需另开存活/清理设计 |
| `git` CLI 还是 gix/git2？ | **MVP 选 Git CLI。** 仓库已有 CLI 进程模式且无现成库依赖；参数数组和固定 diff flags 限制 shell/config 旁路 | 后续若性能证据足够，再以独立 adapter 评估 gix |
| 「TurnEnd 静默 capture 默认开」由谁控制？ | **不是 infra port 的职责。** MVP 不读配置、不改 settings；未来 `agent-turn-changeset` 消费者拥有开关和失败观测语义 | `XyEvent` / TUI chrome 不在本 change |

## 5. 测试 seam 与验收

由于本 MVP 没有 CLI 或产品入口，主 seam 是公开
`XyWorktreeSnapshot` port 和真实 Git adapter，不另造脱离产品的命令行
harness。start 后按 BDD-on 规则落 live spec；本次 pre-start 不编辑
`llmanspec/specs/**`。

实现阶段至少覆盖：

- 干净仓、tracked 修改/新增/删除、rename、binary patch；
- baseline 已有修改时，后续 diff 只报告相对 baseline 的变化；
- staged 与 unstaged 混合时，用户 index 内容与 index 文件均不被改写；
- 默认纳入非 ignored untracked，关闭选项后不纳入，ignored 文件始终不纳入；
- 普通仓、Git worktree、detached HEAD、非 Git 目录、unborn HEAD、缺失
  `git` 命令；
- foreign/stale baseline、NUL 路径解析、Git 命令失败；
- `XyWorktreeSnapshot` 可由 fake 实现替换，且 protocol 不引用 infra。

## 6. Start readiness（pre-start）

### 规划就绪

- `proposal.md`、本 `design.md`、`tasks.md` 三件规划工件齐全后，
  change 可标记为 Designed；
- `depends_on: []`，没有待绑定的前置 change；
- 范围、port seam、Git 原语、错误和 Open Questions 已锁定；
- 当前不改 live specs、不创建 change 分支、不改代码。

### 分支启动前置条件

仍需满足 llman 的 Branch binding 门禁后才能执行后续任务：

1. 工作区回到干净状态且位于默认分支；
2. 运行 `llman sdd change start c1770-add-worktree-snapshot`；
3. 仅在新分支上落 `infra-worktree-snapshot` live spec（BDD-on 下按
   Partitioned SSOT 放置可执行场景），再进入 apply。

本次检查发现工作区已有与本 change 无关的
`llmanspec/changes/c2045-add-tui-fold-target-remaining/proposal.md`
修改；不触碰、不覆盖它。因此**规划就绪，但当前不能直接 start**。

## 7. 非目标

- 不改变 Trust / 工具 allow-all；
- 不把快照变成 `XyEvent`、session entry 或产品状态；
- 不读取或写入用户 index 之外的持久 snapshot ref；
- 不为未来 review、回滚、评论、插件或多 VCS 提前造抽象。
