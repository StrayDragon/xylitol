# Tasks: c276-hoist-shared-vocabulary-to-core

## P1 — source_info + xml_escape（独立模块，先做）

- [x] T1 新建 `src/core/source_info.rs`：迁入 `infra/source_info.rs` 全部内容（`SourceScope`/
  `SourceOrigin`/`SourceInfo` + 2 工厂 fn）+ `infra/skills/loader.rs::xml_escape`
- [x] T2 `core/mod.rs` 注册 `pub mod source_info;`
- [x] T3 `infra/source_info.rs` 改为 `pub use crate::core::source_info::*;`（过渡薄 re-export）
- [x] T4 `infra/skills/loader.rs`：删 `xml_escape` 本体，加 `pub use crate::core::source_info::xml_escape;`
- [x] T5 agent 生产 import 改指向 core：`prompt/commands.rs`、`prompt/templates.rs`、
  `tools/definition.rs`（source_info）、`prompt/skills.rs`、`prompt/system.rs`（xml_escape）
- [x] T6 删白名单对应条目；验证 build + arch_guard

## P2 — AgentLifecycleEvent

- [x] T7 新建 `src/core/lifecycle.rs`：迁入 `infra/event/lifecycle.rs` 的 `AgentLifecycleEvent`
  enum（含所有 variant）；`EventBus` 实现留 infra（c277）
- [x] T8 `core/mod.rs` 注册；确认仅依赖 `core::message`（方向正确）
- [x] T9 `infra/event/lifecycle.rs` 改为 `pub use crate::core::lifecycle::AgentLifecycleEvent;` + 保留 EventBus 相关
- [x] T10 agent import 改指向 core：`session/events.rs`、`session/mod.rs`、`session/steering.rs`
- [x] T11 删白名单对应条目；验证

## P3 — SessionEntry 族（拆分 types.rs）

- [x] T12 新建 `src/core/session_types.rs`：迁入 SessionHeader/EntryBase/MessageEntry/
  CompactionEntry/BranchSummaryEntry/ModelChangeEntry/ThinkingLevelChangeEntry/CustomEntry/
  CustomMessageEntry/LabelEntry/SessionInfoEntry/BashExecutionEntry/SessionContext/
  SessionTreeNode/SessionEntry enum + SESSION_VERSION 常量 + SessionEntry 方法
- [x] T13 `SessionBackend` 从 types.rs 移到 `infra/session/manager.rs`（唯一用户）或 `infra/session/mod.rs`
- [x] T14 `infra/session/types.rs` 改为 `pub use crate::core::session_types::*;` + SessionBackend re-export
- [x] T15 agent import 改指向 core：`compaction/{cut_detector,file_ops,message_converter,mod}.rs`
  （session::types）、`session/export.rs`（session）
- [x] T16 删白名单对应条目；验证

## P4 — CompactionSettings/Config（评估，低成本则做）

- [x] T17 评估 `CompactionSettings`（infra/settings）与 `CompactionConfig`（infra/config）能否
  抽到 core；若 config 嵌在 AppConfig 难抽则只迁 settings
- [x] T18 若迁：agent `compaction/settings.rs` 的 `From` impl 改指向 core；删白名单条目

## P5 — resource 类型（评估，高牵连可能不做）

- [x] T19 评估 `SkillInfo`/`PromptTemplate`/`ThemeInfo`：已从 loader.rs 抽出到 core/resource_types.rs（纯数据，低成本）；prompt/system.rs 的 DefaultResourceLoader 是 service，白名单条目重标 c277
  条目，单列后续 change（design §6）

## 收尾

- [x] T20 全量 QA：`cargo fmt && cargo clippy --all-targets && cargo nextest run --profile ci
  && cargo test --test bdd -- --test-threads=1`
- [x] T21 确认 arch_guard 白名单显著收缩（~10–18 项删除，剩余条目均有后续标注）
- [x] T22 `llman sdd validate c276-hoist-shared-vocabulary-to-core --strict --no-interactive` 通过
