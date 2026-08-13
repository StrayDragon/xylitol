# 派工 prompt：ActivityAtom 难核（c2200）

独立 worktree。这是 **架构难核**：让新 `UiEntry` 变体无法被 `_ => {}` 吞掉。

工作目录必须是本 wt 根。先：

```bash
eval "$(just cargo-wt-env)"
```

读：

- `.tmp/c1762-activity-fold-labels-lessons.md`（若无则读 archive c1762 + `activity_fold/summary.rs`）
- `src/app/tui/activity_fold/summary.rs`（`count_middles` / `format_cluster_body` / `is_*_tool`）
- `src/app/tui/bridge/model.rs` `UiEntry`
- `llmanspec/changes/c2200-refactor-tui-kind-catalog-verify/proposal.md`
- `llmanspec/changes/c2200-refactor-tui-kind-catalog-verify/research/code-as-design-and-tui-verify.md` §4
- 根 `AGENTS.md` Pre-0.0.1 卫生（禁止兼容别名）

## 做

1. 引入小、穷举的贡献类型（名称可议，如 `ActivityAtom` / `ActivityContribution`）：
   - 角色：Edit / Explore / Run / Used / Think / Ask / Compaction / Noise / Projection
   - 计数策略已编码在角色里（path 去重 vs 调用次数 vs omit）
   - live 身份：绑 entry id，不绑全局 `streaming_thinking`
2. `UiEntry` → atom：**穷举 match，禁止 `_ => {}`**。Todo = Projection（不算 Used）。
3. 工具名字符串表从 fold 层挪走：要么投影期打角色，要么 `match` 工具名仍集中 **一处** 表，fold 只消费角色。新未知工具默认 Used（按次），不要默默当 Explored。
4. `format_cluster_body` 改读 atom 聚合，**产品可见词表不变**（Edited XOR Explored、Used N=次数、Thought 仅 thought-only…）。现有 `activity_fold` 测必须绿。
5. 加测：新变体若有人写成 `_ => {}` 会编不过——用穷举保证，不必宏魔法。

## 不要

- 插件注册表 / 运行时动态 kind
- 改 playground HTML、live specs、词表文案（除非测证明旧文案本来就错）
- 大拆 `harness.rs` / `scrollback.rs`
- 兼容 shim / 旧函数名双写

## 完成

- `cargo test --lib activity_fold` 绿
- `count_middles` 不再对 `UiEntry` `_ => {}`
- PR 说明：新块要改哪一处（atom match + 角色表）

Commit：`refactor(tui): ActivityFold kinds via exhaustive atoms`
