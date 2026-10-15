---
depends_on: []
title: BDD 孤儿场景清理与 outline 化第二波（cli-entry / agent-tools）
---

## Why

specs 对账（1012 场景句柄 vs 1007 `#[scenario]` 绑定）暴露 5 个孤儿场景句柄——feature 里有 `场景:` 但无绑定消费，BDD 门禁对它们零覆盖宣告失真；其中 2 个是 c2921 outline 化后遗留的冗余句柄。同时 c2920/c2921 已验证场景大纲通路的两个关键约束（sync binding 才全量展开 case；表值引号形态对齐 `{x:string}`），可复制到两处完美矩阵。

## What Changes

- 删孤儿句柄 2：`test-hooks-wiring::model-select` / `thinking-select`——r1820 语义已被 `wiring-observe-matrix` case 4/5 覆盖，规则正文改为指向大纲。
- 补绑定 2：`app-tui::surface-local-clipboard-and-host-resources`（步骤已在 `steps_app_tui_host.rs`）与 `app-tui-transcript::product-gutter-click-jumps`（复用包侧 gutter 步骤短语，产品侧补 given 壳）。
- 澄清双定义 1：`estimate-fallback-heuristic` 在主仓 domain-compaction 与包仓 package-ai-bridge-accounting 各有一份（主仓版已绑定）。包仓版正文加 `# verified-by` 指回主仓绑定，声明镜像关系，不删（包仓 spec 是包内契约视图）。
- outline 化 `cli-entry`：r1383/r1390 两规则各持半边矩阵（persisted→有提示 / unpersisted→无提示），合并为每规则一个 2 行场景大纲（`persisted` × `hint_present`），4 场景 → 2 大纲；绑定 4 → 2（sync fn）。
- outline 化 `agent-tools` read 族：`read-entire` / `read-offset-limit` / `read-offset-oob` / `read-truncate` 共享「存在文件→调用 read→断言」骨架，差异全在路径/偏移/期望 → 1 大纲 4 行；绑定 4 → 1。

## Out of Scope

- server-core / app-tui-transcript / infra-otel 的同构组：step 序列同构但 then 断言各异，参数化收益低且断言语义逐场景不同，留待后续按需。
- archive freeze（22 条候选）：独立维护操作，与本 change 无关。

## Verification

- `cargo test --test bdd` 全绿；孤儿清零后绑定数 == 场景句柄数（对账脚本口径）。
- `llman-sdd validate --all --strict` 绿。
- cli-entry / agent-tools 大纲 case 展开数与原场景数一致（4+4）。
