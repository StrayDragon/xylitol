---
depends_on: []
title: BDD 孤儿清理、xstate 流转门禁接入与就绪状态机表改写
branch: sdd/c2922-refactor-bdd-orphans-outline-wave2
base_branch: main
base_sha: 1e0fc94ce5db04fc161cf1de7e843de57f04eb34
---

## Why

三重背景叠加：

1. **孤儿欠账**：specs 对账（1012 场景句柄 vs 1007 `#[scenario]` 绑定）暴露 5 个孤儿场景句柄——feature 里有 `场景:` 但无绑定消费，BDD 覆盖宣告失真；其中 2 个是 c2921 outline 化后遗留的冗余句柄。
2. **llman-sdd 0.8.2 状态机门禁已可用**：`specs.check_transitions` 经 XState 提供 E1-E4 四道 ERROR 门禁（表结构/确定性/可达性/死锁），对本仓三个真状态机（server-core 就绪态、attach 连接代际、ReAct 回合）是白拿的图论完备性守卫——现行离散 `场景:` 只是状态空间抽样，无孤岛/死锁检测。
3. **rutis 集成前置审查**：P1（hook 层替换）在即，specs 须区分「真真理（行为合约，重写后仍成立）」与「过渡态快照（锚定现行实现形状，重写时腐烂）」，避免 BDD 沦为迭代阻碍；hooks 域 r1-r3 时代场景属后者，先声明缓议而非现在固化。

## What Changes

- 删孤儿句柄 2：`test-hooks-wiring::model-select` / `thinking-select`——语义已被 `wiring-observe-matrix` case 4/5 覆盖，r1820 规则正文改为指向该大纲。
- 补绑定 2：`app-tui::surface-local-clipboard-and-host-resources`（步骤已在 `steps_app_tui_host.rs`）与 `app-tui-transcript::product-gutter-click-jumps`（复用包侧 gutter 步骤语义，产品侧补步骤壳）。
- 澄清双定义 1：`estimate-fallback-heuristic` 在主仓 domain-compaction 与包仓 package-ai-bridge-accounting 各一份（主仓版已绑定）。包仓版加 `# verified-by` 指回主仓绑定声明镜像，不删（包仓 spec 是包内契约视图）。
- `llmanspec/config.yaml` 开启 `specs.check_transitions: true`（已实测现有 47 specs / 4 outline 全过 E1-E4）。
- server-core r1786 就绪状态机改写：`starting/ready/stopping/failed` 四态 + 装配完成/装配失败/优雅停机等事件收进 `场景大纲` 迁移表（from/event/to 列），纳入 xstate E1-E4 门禁；现有 3 个离散场景语义并入表行，补齐 stopping→failed 等未守护转移。
- hooks 域过渡态标注：`agent-hooks.feature` 头部注释声明 r1-r3（HookDispatcher 机制细节）为 rutis P1 重审对象，现行场景保持但不再新增同类机制锚定场景。

## Out of Scope

- cli-entry / agent-tools read 族 outline 化：真真理但收益仅省 binding，rutis 落地后再按需评估。
- app-tui-host 连接代际 / agent-runtime ReAct 回合的迁移表改写：r1786 探路后再议。
- archive freeze（22 条候选）：独立维护操作。
- rutis 集成本身：见 `~/Projects/__straydragon__/rutis/integrated-xylitol-suggestions.md` 独立立项。

## Verification

- `cargo test --test bdd` 全绿；孤儿清零（对账口径：绑定数 == 场景句柄数，包仓镜像经 verified-by 豁免）。
- `llman-sdd validate --all --strict` 绿（含 check_transitions 门禁）。
- r1786 迁移表通过 E1-E4 且转移覆盖 starting/ready/stopping/failed 全部可达路径。
