---
depends_on: []
needs_specs_change: true
branch: tui/compaction-standalone-block
base_sha: 65690dd117982e127dbd5e6b155dd04e34b46cc8
---

# TUI compaction 块独立成块呈现（不折进 explorer 簇）

## Why

实测（session babb2fbc，2026-09-13 截图）中 `[compaction] Compacted from …` 行被折进
"Exploring N files…" 探索簇头内，簇折叠后 compaction 完全不可见。机制成因：

- `UiEntry::Compaction` 被归类为可折叠 middle（`src/app/tui/activity_fold/atom.rs:105`），
  簇只由可展示 assistant 正文封口（`segment.rs:135-145`），compaction 与 read/grep 工具同簇；
- 簇折叠时 `plan.skip_middle` 把包括 Compaction 在内的全部 middle 吞掉
  （`src/app/tui/widgets/scrollback/mod.rs:223-236`）；
- 现行 spec att34 规定「Compaction MUST NOT 单独切簇」，att23/att24 规定混合簇头不写
  compaction——compaction 在 explorer 场景完全不可见是 spec 规定的现状。

产品决策：**compaction 是重大操作，不应被折叠进 explorer 簇摘要，应单独显示**；
但可以随 `Worked for` 信封（L3）折叠。

## What Changes

- **att34 修订（唯一 spec 变更）**：切簇边界从「仅 assistant 正文」扩为
  「assistant 正文 ∪ Compaction」——Compaction MUST 封口当前打开簇并自成单例簇独立成块，
  其后中间活动 MUST 另起新簇。att23/att24 条文在新现实下仍全部为真，不做修改：
  - att23「纯 Compaction 簇 MUST NOT 画簇摘要头、信封展开直接露出 Compaction 块」
    由单例簇走既有 `omits_cluster_header` 路径自然满足；
  - att24「混合簇头 MUST NOT 写入 compaction」成立（compaction 不再进入混合簇）；
    「仅 Compaction MUST NOT 画簇头」仍由 att23 约束。
- **实现**：`partition_segments`（`src/app/tui/activity_fold/segment.rs`）增 Compaction
  封口分支；live 与 resume/rebuild 同构自动成立（同一 partition 函数）。
- **验收**：新增 `@executable` 场景 `compaction-seals-cluster-headless`（att34）+
  SceneBuilder 增加 Compaction 构造方法 + segment.rs 单测三形态。

## Capabilities

- `app-tui-transcript`（att34 切簇规则修订）

## Impact

- 代码：`src/app/tui/activity_fold/segment.rs`（切簇）、`activity_fold/scene.rs`
  （SceneBuilder 构造方法）、`tests/bdd/steps_app_tui_transcript.rs` +
  `tests/bdd/bindings_app_tui_transcript.rs`（新场景）。
- 行为：compaction 块在流式与回放中始终独立可见；其后的工具重新开簇
  （"Exploring…" 计数从零起算，att35 后缀语义不变）；L3 信封折叠仍收纳 compaction
  （att23 上文不变）。
- 兼容：无 wire / 持久化变更；纯 TUI 投影。

## Open Questions（propose 阶段已决）

- 尾簇头文案：不特判——compaction 封口后尾簇按 att24/att35 既有规则自然起头。
- 连续多条 Compaction：各自独立单例簇（同构于封口语义，不合并）。
- reason 修饰词：本次不加（词汇表零变更）；需要时另立 change。
