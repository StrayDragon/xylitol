# 三条主线 · 收尾记录（已全部归档 2026-08-17）

仓库内索引（原「可 share」工作索引）随三条线归档关闭；临时 `_HANDOFF/` 已删。

| 线 | 归档目录 | 一句话 | 交付 |
|---|---|---|---|
| A TUI | `llmanspec/changes/archive/2026-08-17-c2200-refactor-tui-kind-catalog-verify/` | 形态可注册；场景即目录；帧带可测 | ActivityAtom 穷举形态表（禁 `_ => {}`，`8863e79a`）+ SceneBuilder 场景目录走生产 paint（`f6540407`）+ live_tape 帧带 PASS/FAIL + PreviewInject（ChromeOp / LiveXy 经 `HostSession::step`，`90232f83`） |
| B Specs | `llmanspec/changes/archive/2026-08-17-c2210-update-specs-product-level/` | spec 只钉产品 WHAT | 全量审计表 + 两刀改写（`76497cdb` 摩擦 1–10、`900e9311` 余下 valid_scope、`d943be28` 迁移条款）；65 个 toon 提到 `src/` 归零 |
| C 卫生 | `llmanspec/changes/archive/2026-08-17-c2220-update-pre-release-hygiene/` | 0.0.1 前禁止 unpublished 兼容 | 根 `AGENTS.md`「Pre-0.0.1 卫生」（`1dce633d`）+ 死码分诊表与即删执行（`9ee4e388` / `169c528c`）；遗留决策见 `dead-code-triage.md` |

相邻已归档变更：

- 设计文档 / playground 收成 `designing` Web（两源模型，正交于 PreviewInject）：`llmanspec/changes/archive/2026-08-16-c2230-add-tui-designing-web/`
- 流式时间戳：`llmanspec/changes/archive/2026-08-14-c2240-add-stream-node-timestamps/`

A 线收尾口径：Chrome 其余 op（Toast/NextTurnCue/SlotChoice/SlotTree）接受 generic `apply_chrome_op` + fallback，不逐 op 写脚本 runner（壳层纯 UI 效果，非折叠/计数/流式高风险面）；目录穷举性由 `every_scene_declares_inject_seam` 测试兜底。
