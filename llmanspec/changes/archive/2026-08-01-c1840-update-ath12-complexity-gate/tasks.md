# Tasks

## 1. Specs landing

- [x] 1.1 Branch binding：`llman sdd change start c1840-update-ath12-complexity-gate`
- [x] 1.2 改写 `app-tui-host` ath12 statement：结构 MUST 保留；行数 → SHOULD ~800 / 硬味 ~1200；入口复杂度 MUST（cognitive≤35、cyclomatic≤30，cccc-rs / `check_complexity.py`）
- [x] 1.3 改写 `app-tui-host.feature`：`god-files-under-budget` → 入口复杂度闸场景；保留 input-policy / effects-slash 结构场景
- [x] 1.4 扩展 `test-qa-gate`：qg06 + feature（`check_complexity.py` 经 check-scripts 入闸）
- [x] 1.5 `llman sdd validate` + commit Specs landing

## 2. 实现对齐

- [x] 2.1 降级/删除 `god_module_entry_files_under_budget` 的 850 硬顶（若保留行数测：仅硬味 ~1200 或移出 ath12 MUST）[blocked-by: 1.2]
- [x] 2.2 确认 `scripts/check_complexity.py` 阈值/路径与 ath12 一致；必要时补注释指针 [blocked-by: 1.2]
- [x] 2.3 相关单测 / 文档一句（DESIGN 或 host AGENTS 勿堆进度）[blocked-by: 2.1]

## 3. 门禁

- [x] 3.1 `python3 scripts/check_complexity.py --check` + `just check-scripts` + 相关 `cargo test` / `just qa` 切片 [blocked-by: 2.1, 2.2]
