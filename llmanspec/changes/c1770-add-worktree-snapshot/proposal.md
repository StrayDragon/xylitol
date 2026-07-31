---
depends_on: []
---

## Why

Agent 常经 `bash`/`sed` 等改盘，变更不能只挂在 `edit`/`write` 工具结果上。要做可选的 turn-level review、整切面回滚、以及其它「相对某基线的工作树差分」能力，需要先有一层 **与产品 review UI 解耦** 的工作树检测/快照能力。

首个 MVP：**仅 git 仓**；非 git → 能力不可用（关闭，不假成功）。灵感来自 hunk 的 changeset 心智，**不**移植其 OpenTUI/session/extensions。

## What Changes

- 新增独立 **工作树快照 / 差分** 能力（infra port + git 实现；非 git → unavailable）
- **TurnStart 钉基线**；TurnEnd **静默 capture** 默认开、可关；与 `/review` 共用同一套 Snapshot
- **untracked 默认纳入**切面
- **整切面 restore** 作为 port 能力位保留；**MVP 不提供任何产品操作入口**
- **可选 review 闸**（review 后再继续）与 **review UI** 记为后续消费者，本草案以检测能力为主，不强制同 change 交付 UI

## Capabilities

| capability（意向） | 说明 |
|---|---|
| `infra-worktree-snapshot` | git 工作树 baseline / changeset / restore 能力；非 git unavailable |
| （后置）`agent-turn-changeset` | TurnStart 钉基线、TurnEnd 静默 capture、可选 gate |
| （后置）`app-tui-review` | `/review` 与展示；回滚入口另开 |

正式化 propose 时再拆/合并 capability 与 req。

## 设计要点（探索结论，非正式 design.md）

### 基线策略

**每个 `TurnStart` 钉基线**（相对「累进上一冻结」更不易错，且与静默 capture 兼容：TurnEnd 存切面，基线不因静默滑动）。

### Git 原语（轻量、含 untracked）

推荐 **临时 index + `write-tree`**，避免 `git stash create`（默认不含 untracked）：

```text
TurnStart / capture(baseline):
  GIT_INDEX_FILE=<tmp>
  git read-tree HEAD
  git add -A                    # 纳入 untracked（可配置关闭）
  baseline_tree=$(git write-tree)
  # 工作树与用户 index 不变；只得到 tree oid

TurnEnd / changeset(baseline→now):
  同样得到 now_tree
  git diff --name-status <baseline_tree> <now_tree>
  git diff <baseline_tree> <now_tree>   # 供展示 / 存储

restore(baseline)（能力位，无 MVP 入口）:
  将工作树恢复到 baseline_tree（具体命令后议：
  restore --source= / read-tree -u / 清理基线外 untracked）
  须与「中止 ≠ 回滚」产品文案对齐后再开入口
```

性能：成本主要在脏文件 `add`/`diff`；干净仓接近两次轻量 status/tree。勿全树文件拷贝旁路。

### 产品开关（意向）

| 开关 | 默认 | 含义 |
|---|---|---|
| 静默 capture | **on** | TurnEnd 采切面；可关 |
| review 闸 | **off** | 打开才 pause 至确认 |
| untracked 纳入 | **on** | 可关 |
| 非 git | 能力关闭 | — |

### 非目标（本方向 MVP）

- jj/sl、FS watch、overlay 沙箱
- hunk session daemon / mouse / extensions / STML
- 默认每工具审批（保持 allow-all）
- MVP 回滚/拒绝的 UI 或 slash 入口

## Impact

- **协议**：可能新增 changeset 相关 `XyEvent`（正式化时再定闭集增量）
- **agent**：TurnStart/End 接线静默 capture；闸为可选
- **infra**：新 port + git CLI（或后置 gix）；无现成 git2/gix 依赖
- **app**：后置 `/review`；与现有 Diff 组件可复用展示
- **信任/权限**：不改变 Trust / 工具 allow-all 定调

## Open Questions

- [ ] 评论回灌：`steer` vs 专用 `ReviewContinue`
- [ ] restore 的精确 git 命令与 untracked 清理策略（开入口前必须定）
- [ ] 是否拆成「仅 infra snapshot」与「agent/app 消费者」两个 change
- [ ] tree oid 存活：是否 `git update-ref` 挂 refs/xylitol/… 防 gc

## 探索笔记

详见 `_HANDOFF/turn-changeset-review-explore-20260730.md`。
