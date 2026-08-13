# 三条主线 · 仓库内索引（可 share）

`_HANDOFF/` **不进 git**。其它 worktree / 同事只看本文件与下列路径。

| 线 | Change 目录 | 派工 prompt | 调研 |
|---|---|---|---|
| A TUI | `c2200-refactor-tui-kind-catalog-verify/` | [`PROMPT-activity-fold-scene.md`](./PROMPT-activity-fold-scene.md) · [`PROMPT-activity-atom.md`](./PROMPT-activity-atom.md) | [`code-as-design-and-tui-verify.md`](./code-as-design-and-tui-verify.md) · [`tui-as-app-stable-vs-iterate.md`](./tui-as-app-stable-vs-iterate.md) · [`preview-by-construction.md`](./preview-by-construction.md) · [`preview-looks-like.md`](./preview-looks-like.md) · [`preview-ssot-map-and-inject.md`](./preview-ssot-map-and-inject.md) |
| B Specs | `../c2210-update-specs-product-level/` | [`../c2210-update-specs-product-level/research/PROMPT-spec-audit.md`](../c2210-update-specs-product-level/research/PROMPT-spec-audit.md) | [`../c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md`](../c2210-update-specs-product-level/research/spec-rigidity-and-product-level.md) |
| C 卫生 | `../c2220-update-pre-release-hygiene/` | [`../c2220-update-pre-release-hygiene/research/PROMPT-dead-code.md`](../c2220-update-pre-release-hygiene/research/PROMPT-dead-code.md) | [`../c2220-update-pre-release-hygiene/research/pre-release-hygiene.md`](../c2220-update-pre-release-hygiene/research/pre-release-hygiene.md) |

开 wt：`eval "$(just cargo-wt-env)"`。禁止共用 `CARGO_TARGET_DIR`。禁止在默认分支改 `llmanspec/specs/**`。
