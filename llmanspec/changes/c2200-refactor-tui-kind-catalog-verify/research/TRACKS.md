# 三条主线 · 仓库内索引（可 share）

`_HANDOFF/` **不进 git**。其它 worktree / 同事只看本文件与下列路径。

| 线 | Change 目录 | 派工 prompt | 调研 | main 上已合 |
|---|---|---|---|---|
| A TUI | `c2200-refactor-tui-kind-catalog-verify/` | [`PROMPT-activity-fold-scene.md`](./PROMPT-activity-fold-scene.md) · [`PROMPT-activity-atom.md`](./PROMPT-activity-atom.md) | [`code-as-design-and-tui-verify.md`](./code-as-design-and-tui-verify.md) · [`tui-as-app-stable-vs-iterate.md`](./tui-as-app-stable-vs-iterate.md) · [`preview-by-construction.md`](./preview-by-construction.md) · [`preview-looks-like.md`](./preview-looks-like.md) · [`preview-ssot-map-and-inject.md`](./preview-ssot-map-and-inject.md) | **ActivityAtom** + **fold-scene dump** + **PreviewInject**（`ChromeOp` + `LiveXy` `/debug activity-fold-live-xy` 走 `HostSession::step`）。尚未：Chrome 其余 op 的 `/debug` 行；全表走同一 inject 文案 |
| B Specs | `../c2210-update-specs-product-level/` | [`../c2210-update-specs-product-level/research/PROMPT-spec-rewrite-friction.md`](../c2210-update-specs-product-level/research/PROMPT-spec-rewrite-friction.md) · [`../c2210-update-specs-product-level/research/PROMPT-spec-rewrite-remainder.md`](../c2210-update-specs-product-level/research/PROMPT-spec-rewrite-remainder.md) | [`../c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md`](../c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md) · [`../c2210-update-specs-product-level/research/audit-table.md`](../c2210-update-specs-product-level/research/audit-table.md) | 审计表 + 摩擦 1–10（`76497cdb`）+ **余下 valid_scope / 组织钉第二刀**（本分支）。夹具路径（agent-tools）keep |
| C 卫生 | `../c2220-update-pre-release-hygiene/` | [`../c2220-update-pre-release-hygiene/research/PROMPT-dead-code-delete.md`](../c2220-update-pre-release-hygiene/research/PROMPT-dead-code-delete.md) | [`../c2220-update-pre-release-hygiene/research/pre-release-hygiene.md`](../c2220-update-pre-release-hygiene/research/pre-release-hygiene.md) · [`../c2220-update-pre-release-hygiene/research/dead-code-triage.md`](../c2220-update-pre-release-hygiene/research/dead-code-triage.md) | 分诊表 + **可立刻删清单已执行**（`9ee4e388`）。Windows cfg 的 decode 留下；tests/support 跨 target allow 留下；拿不准 5 项 / `XyRemoteDriver` 未动 |

开 wt：`eval "$(just cargo-wt-env)"`。禁止共用 `CARGO_TARGET_DIR`。禁止在默认分支改 `llmanspec/specs/**`。

fold-scene 已 rebase 到 atom 之上并合入。新场景测走 `SceneBuilder` → `apply_xy_event` → `UiRoot::render`。

A/B 执行切片 prompt：[`../c2220-update-pre-release-hygiene/research/PROMPT-dead-code-delete.md`](../c2220-update-pre-release-hygiene/research/PROMPT-dead-code-delete.md) · [`../c2210-update-specs-product-level/research/PROMPT-spec-rewrite-friction.md`](../c2210-update-specs-product-level/research/PROMPT-spec-rewrite-friction.md)。

设计文档 / playground 收成 `designing` Web（**正交于** PreviewInject）：[`../c2230-add-tui-designing-web/research/PROMPT-designing-web.md`](../c2230-add-tui-designing-web/research/PROMPT-designing-web.md)。本会话不再跟这条。
