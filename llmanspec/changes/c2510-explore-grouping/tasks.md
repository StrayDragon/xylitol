# Tasks：c2510 探索分组

- [ ] t1 策略层：activity_fold 纯函数分类器（相邻同类检索段 ≥3 合组、类别断开/写类混入断组、进行时计数、auto_group 开关短路）+ settings `auto_group` 加载（runtime-config rc29 姊妹场景的配置单测随动）；库内边界单测转绿
- [ ] t2 渲染与交互：分组摘要行进 scrollback（词形、✱ 标记、计数更新）、展开交互与手动退组保持、稳定组 id 与重建同构；transcript harness 可执行场景转绿（`explore-group-merge-and-expand` / `explore-group-rebuild-isomorphic`，HostPumpBdd 同族新 steps+bindings）
- [ ] t3 designing 随动：activity-fold intent.md 词表 + 4 固定态 + draft.yaml 取舍；chrome 词表 G 类条目；`just gen-designing-index` 与 designing lint 过闸
- [ ] t4 门禁收口：`just fmt` / `just lint` / `just test`（含既有 BDD + 新增场景 + designing lint）；`llman sdd validate --strict`
