# 三条主线 · 仓库内索引（可 share）

`_HANDOFF/` **不进 git**。其它 worktree / 同事只看本文件与下列路径。

| 线 | Change 目录 | 派工 prompt | 调研 | main 上已合 |
|---|---|---|---|---|
| A TUI | `c2200-refactor-tui-kind-catalog-verify/` | [`PROMPT-activity-fold-scene.md`](./PROMPT-activity-fold-scene.md) · [`PROMPT-activity-atom.md`](./PROMPT-activity-atom.md) | [`code-as-design-and-tui-verify.md`](./code-as-design-and-tui-verify.md) · [`tui-as-app-stable-vs-iterate.md`](./tui-as-app-stable-vs-iterate.md) · [`preview-by-construction.md`](./preview-by-construction.md) · [`preview-looks-like.md`](./preview-looks-like.md) · [`preview-ssot-map-and-inject.md`](./preview-ssot-map-and-inject.md) | **ActivityAtom** + **fold-scene dump**。**PreviewInject** 已进目录表；`thinking_flushed` 走产品 flush。尚未：全表走同一 inject、LiveTape 进 `resolve_scene_id`、ChromeOp |
| B Specs | `../c2210-update-specs-product-level/` | [`../c2210-update-specs-product-level/research/PROMPT-spec-audit.md`](../c2210-update-specs-product-level/research/PROMPT-spec-audit.md) | [`../c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md`](../c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md) · [`../c2210-update-specs-product-level/research/audit-table.md`](../c2210-update-specs-product-level/research/audit-table.md) | 审计表；**尚未改 live specs** |
| C 卫生 | `../c2220-update-pre-release-hygiene/` | [`../c2220-update-pre-release-hygiene/research/PROMPT-dead-code.md`](../c2220-update-pre-release-hygiene/research/PROMPT-dead-code.md) | [`../c2220-update-pre-release-hygiene/research/pre-release-hygiene.md`](../c2220-update-pre-release-hygiene/research/pre-release-hygiene.md) · [`../c2220-update-pre-release-hygiene/research/dead-code-triage.md`](../c2220-update-pre-release-hygiene/research/dead-code-triage.md) | 分诊表；**尚未删生产代码** |

开 wt：`eval "$(just cargo-wt-env)"`。禁止共用 `CARGO_TARGET_DIR`。禁止在默认分支改 `llmanspec/specs/**`。

fold-scene 已 rebase 到 atom 之上并合入。新场景测走 `SceneBuilder` → `apply_xy_event` → `UiRoot::render`。

A/B 执行切片 prompt：[`../c2220-update-pre-release-hygiene/research/PROMPT-dead-code-delete.md`](../c2220-update-pre-release-hygiene/research/PROMPT-dead-code-delete.md) · [`../c2210-update-specs-product-level/research/PROMPT-spec-rewrite-friction.md`](../c2210-update-specs-product-level/research/PROMPT-spec-rewrite-friction.md)。

设计文档 / playground 收成 `designing` Web（**正交于** PreviewInject）：[`../c2230-add-tui-designing-web/research/PROMPT-designing-web.md`](../c2230-add-tui-designing-web/research/PROMPT-designing-web.md)。本会话不再跟这条。
