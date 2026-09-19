# Tasks: c1770-add-worktree-snapshot

> 本清单是 start 之后的 apply 计划。当前仍是 pre-start：所有任务保持未勾选，
> 不执行 `change start`，不编辑 live specs，不改代码。

## 1. Branch binding 与合约落地

- [ ] 1.1 清理并确认默认分支工作区；在用户允许进入实现阶段后运行
  `llman-sdd change start c1770-add-worktree-snapshot`，不得在默认分支落 live spec。
- [ ] 1.2 [blocked-by: 1.1] 新建 `infra-worktree-snapshot` live capability，写明
  Git-only availability、baseline/capture、untracked 策略、tree-to-tree
  changeset 与 stale/foreign error；按 BDD-on Partitioned SSOT 放置场景。
- [ ] 1.3 [blocked-by: 1.2] 运行该 capability 与 change 的
  `llman-sdd validate --strict --no-interactive`，确认没有 dual-write、
  空 `valid_scope` 或未解析依赖。

## 2. Port seam

- [ ] 2.1 [blocked-by: 1.2] 在 `protocol::ports` 定义
  `XyWorktreeSnapshot`、快照/changeset DTO、options 与 typed errors；保证
  port 只依赖 protocol 根类型/标准库，不依赖 `infra`、`agent` 或 wire。
- [ ] 2.2 [blocked-by: 2.1] 按现有 ports 习惯补齐模块声明和精选
  re-export；不新增 `XyEvent`、wire command/event、session entry 或产品
  settings。
- [ ] 2.3 [blocked-by: 2.1] 为 fake backend 保留可替换 seam，锁定
  `Unavailable` 不等于空 diff、foreign/stale baseline 必须显式失败，以及
  binary-safe patch / rename path 的数据契约。

## 3. Git adapter：capture 与 changeset

- [ ] 3.1 [blocked-by: 2.2] 在 `infra::git` 复用 `find_git_repo`，实现
  `GitWorktreeSnapshot::try_new(cwd)`；普通仓与 `.git` worktree 文件均以
  worktree root 执行，非 Git、unborn HEAD、Git 不可执行统一进入
  `Unavailable` 分类。
- [ ] 3.2 [blocked-by: 3.1] 实现每次 capture 使用独立临时目录和不存在的
  `GIT_INDEX_FILE`；通过 `read-tree`、`add -u`/`add -A`、`write-tree`
  生成 tree oid，保持用户 index、环境变量和 cwd 不变。
- [ ] 3.3 [blocked-by: 3.2] 实现 tree-to-tree changeset：NUL
  name-status 解析 rename/old path，生成
  `--binary --full-index --no-ext-diff --no-textconv` patch bytes，并对
  baseline 身份、策略、tree oid 做校验。
- [ ] 3.4 [blocked-by: 3.3] 处理 Git 非零退出、坏 stdout、临时资源失败和
  tree 被回收；错误必须保持 typed error，不得返回伪造空 changeset。

## 4. Rust seam 验证

- [ ] 4.1 [blocked-by: 2.3] 以临时真实 Git 仓覆盖干净仓、tracked 修改/
  新增/删除、rename、binary、baseline 相对变化，以及 staged + unstaged
  混合；断言用户 index 内容与 index 文件未被修改。
- [ ] 4.2 [blocked-by: 3.4] 覆盖默认纳入非 ignored untracked、关闭选项、
  ignored 排除、普通仓 / worktree / detached HEAD，以及非 Git、unborn
  HEAD、缺失 Git、foreign/stale baseline 和 NUL 路径。
- [ ] 4.3 [blocked-by: 4.1, 4.2] 覆盖 fake port 可替换性和 protocol →
  ports 的编译边界；不添加 agent/app 产品测试，不为本 MVP 新建 `/review`
  或回滚入口。

## 5. 闸门与范围复核

- [ ] 5.1 [blocked-by: 4.3] 运行触及面的格式化、lint 与 Rust 测试；按仓库
  规则使用独立 worktree target，避免复用其它 worktree 的 Cargo target。
- [ ] 5.2 [blocked-by: 5.1] 运行相关 BDD / change strict validation，并
  核对 live spec 场景与实现一致；若出现协议、TurnStart/TurnEnd、配置或
  产品入口改动，先拆出后续 change。
- [ ] 5.3 [blocked-by: 5.2] 运行 verify 前复核本设计的 Open Questions：
  评论回灌、restore、tree ref 存活和 agent/app 拆分仍不得被本 MVP 偷渡；
  restore 只在另一个 change 锁定破坏性语义后实现。
