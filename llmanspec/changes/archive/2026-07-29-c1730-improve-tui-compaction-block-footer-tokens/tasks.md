# Tasks: c1730-improve-tui-compaction-block-footer-tokens

## 1. Specs + design SSOT

- [x] 1.1 确认 design playground / compaction-status.md / footer.md（目视已通过）
- [x] 1.2 修订 live `app-tui-chrome` / `app-tui-bridge`：compaction 块默认折叠 + footer 刷新时机
- [x] 1.3 `change attach` feature 分支

## 2. Transcript block

- [x] 2.1 CompactionStart → 占位块；End 成功就地变完成块（默认折叠）
- [x] 2.2 End 失败/aborted 短失败态；resume/rebuild 映射 CompactionEntry→折叠块
- [x] 2.3 展开键与 tool 共用；bridge 单测覆盖

## 3. Footer tokens

- [x] 3.1 CompactionEnd / TurnEnd / mid-turn Api usage（节流）触发 footer refresh
- [x] 3.2 异步不阻塞；restore/resume 刷新保持

## 4. Gate

- [x] 4.1 `python3 scripts/check_tui_design_playground.py --check`
- [x] 4.2 相关测试 + `llman sdd validate c1730-… --strict`
