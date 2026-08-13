# 派工 prompt：live spec 产品级审计（c2210）

把下面整段交给另一个 agent。它只出审计表，**不**在默认分支改 `llmanspec/specs/**`。

---

你在 xylitol 仓库的一个 **独立 git worktree** 里工作（与主线并行，勿挡主线）。

## 环境（硬）

```bash
eval "$(just cargo-wt-env)"   # 禁止与其它 worktree 共用 CARGO_TARGET_DIR
```

读：

- `llmanspec/AGENTS.md` → 「spec 约束层级」
- `llmanspec/changes/c2210-update-specs-product-level/proposal.md`
- `llmanspec/changes/c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md`

尺子（OpenSpec 同构）：**实现能改、对外行为不变 → 不进 spec。**

| 判定 | 含义 |
|---|---|
| `keep` | 用户/下游可观察；语句已干净 |
| `rewrite-product` | 应留产品语义，但现句钉了路径/类型/行数/函数名 → 改写成 WHAT |
| `move-to-AGENTS` | 代码组织 / 文件切分 / 复杂度闸 → 迁 `src/AGENTS.md` 或面 AGENTS（本任务只标记，不改 AGENTS） |
| `delete` | 过期、防复活、空套话、与测试行号绑定 |

大组织方向可 `keep`：分层依赖方向、端口 seam、crate 边界、组合根、跨面同源。不要 `keep` 具体 `mod.rs`、行数、测试文件行号。

## 范围（分两批，先交第一批）

**批次 1（必须先完成）：**

- `layer-architecture`
- `app-tui-host`
- `app-tui-chrome`（尤其 `atc4`）
- `app-tui-design-playground`（整 cap）
- `package-tui-testing`（尤其 `tt02` 行号）

**批次 2：** 其余 `llmanspec/specs/*/spec.toon`。

## 交付物

只写一个文件（覆盖写）：

`llmanspec/changes/c2210-update-specs-product-level/research/audit-table.md`

表列：

```text
| capability | req_id | 判定 | 现行摘句（≤40字） | 改写草稿或迁入何处 | 歧义？（计数/时态/身份是否未钉） |
```

批次 1 全部 req 都要有行。批次 2 可按 cap 汇总 + 只展开明显违规 req。

文末给 10 行以内「最高摩擦 / 建议先改」清单。

## 禁止

- 编辑 `llmanspec/specs/**`、`.feature`、产品 Rust
- `llman sdd change start/attach`（主线架构师尚未正式化本 change）
- 为「看起来完整」编造改写；吃不准标 `rewrite-product` 并写「需确认」
- 提交进默认分支；在本 worktree 的 feature 分支上 commit 审计表即可

## 完成定义

批次 1 表无空判定；`rg 'src/' llmanspec/specs/{layer-architecture,app-tui-host,app-tui-chrome,app-tui-design-playground,package-tui-testing}` 的命中都在表里有对应行。
