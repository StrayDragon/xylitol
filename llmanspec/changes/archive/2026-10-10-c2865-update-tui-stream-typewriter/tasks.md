# Tasks

测试缝：包 `Markdown::render` 单测、产品 `src/app/tui/tests.rs` 流式助手 harness、`tests/bdd` 既有 host / package-tui 绑定。不新开 harness。

## T1 — 引擎剥半截闭合围栏

- [x] `Markdown::render` 在 pulldown-cmark parse 前修剪未闭合围栏末行的不完整 closer（`` ` `` / `~`，长度 < opener）。
- [x] 完整闭合围栏、围栏外文本、空输入 MUST NOT 被误剥。
- [x] 校验：半截 `` ` `` / `` `` `` 与随后补全 ``` 的可见行数相同；完整围栏回归 r1645（不画围栏铬）。

## T2 — [blocked-by: T1] 产品流式助手吃同一把尺子

- [x] streaming assistant 路径继续走引擎 `Markdown`（不另写第二套围栏修剪）。
- [x] 校验：harness 喂「开围栏 + 代码 + 半截 closer → 补全 closer」序列，高度不变；r1255 前缀复用计数仍绿。

## T3 — [blocked-by: T1, T2] BDD 对齐已落地 specs

- [x] 绑定 r1936 / r1937 场景步骤；既有 r1255 / r1645 场景仍绿。
- [x] 校验：相关 `cargo test --all-features --test bdd` 与包 markdown 单测绿。

不在本任务列 finalize。
