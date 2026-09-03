# Tasks：c2510 探索簇头类目计数后缀

- [x] t1 计数与格式化：`ActivityCounts` 增 `read_calls` / `search_calls`（`counts_from_atoms` Explore 臂按 `ExploreKind` 计次，无路径搜索计入 searches）；`format_cluster_body` 探索分句附加 `· N reads[ · M searches]` 后缀（Edited 头不附、仅列非零类目、单复数）；summary.rs 库内单测转绿
- [x] t2 BDD：`explore-head-suffix-counts` / `explore-head-suffix-live-update` 两可执行场景转绿（TranscriptBdd + SceneBuilder 缝新 steps，bindings_app_tui_transcript 注册）
- [x] t3 designing 随动：activity-fold intent.md 词表后缀条目 + 2 固定态 + draft.yaml 取舍；chrome 词表 G 类条目；`just gen-designing-index` 与 designing lint 过闸
- [x] t4 门禁收口：`just fmt` / `just lint` / `just test`（含既有 BDD + 新增场景 + designing lint）；`llman sdd validate --strict`
