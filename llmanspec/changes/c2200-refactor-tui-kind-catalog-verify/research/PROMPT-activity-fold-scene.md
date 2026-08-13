# 派工 prompt：ActivityFold 场景切片（c2200 体力，非难核）

把下面整段交给另一个 agent。这是 **证明「场景 = 生产 paint」** 的窄切片，**不是** 形态注册表，**不是** 删除 playground。

---

你在 xylitol 的一个 **独立 git worktree** 里工作。

## 环境（硬）

```bash
eval "$(just cargo-wt-env)"
```

读（按序）：

1. `.tmp/c1762-activity-fold-labels-lessons.md`（若本树没有，读 `llmanspec/changes/archive/2026-08-14-c1762-update-tui-activity-fold-labels/` 与产品测）
2. `src/app/tui/activity_fold/summary.rs`（`count_*` / `format_cluster_body`）
3. `src/app/tui/design/activity-fold.md` MUST（意图，不当第二套实现）
4. `llmanspec/changes/c2200-refactor-tui-kind-catalog-verify/research/code-as-design-and-tui-verify.md` §3–5
5. 既有产品 harness 里 activity-fold 测试（`src/app/tui/harness.rs` / `activity_fold` 单测）——**扩既有文件**，不要新开平行 harness

## 目标

加一层 **Scene**（名字可变）：给定 `UiEntry` 切片 + fold 态 + 可选 streaming 标志，调用 **与产品相同的** `count_*` / `format_*` / 现有 scrollback paint 路径，产出：

1. **语义 dump**（纯文本，给人和测看，不是截图）：
   - 每一行标明和弦：`L3 envelope` / `L2 cluster` / `L1 block`
   - 簇头字符串、Used/Ran 的 N、是否 thought-only
2. 至少 **3 个场景** 用语义 dump 断言（不要只 assert 子串碰巧出现）：
   - thinking + todo_* → 簇头 Used 不是 Thought（教训 1）
   - 四个同名未知工具 → Used 按**调用次数**（教训 2）
   - 已封口 Thought 后新 Thinking 流 → 封口簇头仍是 Thought（教训 3；若现 API 难注入 Clock，先测纯函数 + 注明缺帧带）
3. （可选）把语义 dump 打成 ANSI/HTML **生成物** 写到 `target/` 或测试 tmp，**禁止**手改 `design/playground/index.html` 当本切片的 SSOT。

## 明确不要做

- 不要改 `format_cluster_body` 词表优先级（那是架构师难核 / ActivityAtom）
- 不要引入运行时插件注册表
- 不要删 playground / `design/*.md`
- 不要把 `agent_demo` 当产品 SSOT
- 不要为场景新建 BDD capability
- 不要在默认分支改 live specs

## 完成定义

- `cargo test --lib activity_fold` 与你加的场景测绿
- 新测失败信息能看出是 L2 还是 L1 错了（语义 dump 出现在 assert 消息或 `Debug`）
- PR/commit 说明里写清：Scene 与产品 paint 的调用点（文件+函数名）

## 提交

在本 worktree 的 feature 分支 commit。标题建议：`test(tui): activity-fold semantic scene dump`。不要 ff 进默认分支；等架构师 review。
