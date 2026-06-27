# c276-hoist-shared-vocabulary-to-core — Design

> 记录词汇上提的迁移边界与降风险策略。纯类型迁移 + import 路径重写，无运行时行为变化。

## 1. 判据：什么算"共享词汇"该进 core

c260 锚点定义 core = "被 ≥2 层共享的纯数据类型 + port + 纯算法，零 crate 依赖"。判据：

1. **纯数据/纯函数**（serde derive、字段容器、无 I/O、无 trait impl 依赖外部资源）
2. **被 agent 和 infra 同时引用**（仅 infra 内部用的留 infra）
3. **迁移后依赖闭包仍为 core-safe**（只依赖 serde/serde_json/std，不反向依赖 infra）

满足三条 → 进 core。仅 #3 不满足（牵连 infra 实现）→ 留 infra，本期不做。

## 2. 本期迁移清单（按成本/收益/风险排序）

| 来源（infra） | 目标（core） | 性质 | 消白名单项 |
|---|---|---|---|
| `infra/source_info.rs`（整文件） | `core/source_info.rs` | 纯类型 + 2 工厂 fn，闭包仅 std+serde | 4（prompt/commands, prompt/templates, tools/definition, prompt/skills 的部分） |
| `infra/event/lifecycle.rs`（`AgentLifecycleEvent`） | `core/lifecycle.rs` | 纯 enum，依赖 `core::message`（方向正确） | 4（session/events, session/mod, session/steering） |
| `infra/skills/loader.rs::xml_escape` | `core/source_info.rs`（或 `core/text.rs`） | 纯函数 | 2（prompt/skills, prompt/system） |
| `infra/session/types.rs`（`SessionEntry` 族 + `SessionContext`/`SessionTreeNode`） | `core/session_types.rs` | 纯 serde 类型，**但需从 types.rs 拆出 `SessionBackend`**（存储后端语义，留 infra） | 8（compaction/*, session/export） |

**预期消白名单 ~18 项**（c276 标记的全部 18 项）。

## 3. 拆分 infra/session/types.rs（唯一需要拆分的文件）

`types.rs` 当前混合两类内容：
- **共享类型**（→ core）：`SessionHeader`、`EntryBase`、`MessageEntry`、`CompactionEntry`、
  `BranchSummaryEntry`、`ModelChangeEntry`、`ThinkingLevelChangeEntry`、`CustomEntry`、
  `CustomMessageEntry`、`LabelEntry`、`SessionInfoEntry`、`BashExecutionEntry`、`SessionContext`、
  `SessionTreeNode`、`SessionEntry` enum、`SESSION_VERSION` 常量。
- **infra 私有**（留 `infra/session/`）：`SessionBackend` enum（仅 `manager.rs` 使用，是存储后端
  实现细节——`Persisted{PathBuf}` / `InMemory{Vec}`）。grep 确认无 agent 引用。

**做法**：
- 新建 `core/session_types.rs`，迁入共享类型（连同 `SessionEntry::base()`/`entry_type()`/`entry_id()`/
  `parent_id()` 方法，这些是纯 match）。
- `SessionBackend` 移到 `infra/session/manager.rs`（它的唯一用户）或保留在 `infra/session/mod.rs`，
  从 types.rs 删除。
- `infra/session/types.rs` 改为对 core 的薄 re-export：`pub use crate::core::session_types::*;`
  （过渡期保留，避免一次改所有 infra 内部引用；后续可清）。`SessionBackend` 单独 `pub use`。

## 4. infra 旧位置的过渡策略：薄 re-export

为控制 blast radius，**不一次性改所有 infra 内部引用**。迁移后 infra 旧路径改为：
```rust
// infra/source_info.rs → 删除文件，infra/mod.rs 改：
pub mod source_info { pub use crate::core::source_info::*; }
// 或保留文件内容仅一行 re-export
```
这样 `crate::infra::source_info::SourceInfo` 仍可解析（infra 内部 + 现有 agent 测试代码不需全改），
但 agent **生产代码**的 import 改指向 `crate::core::source_info`（消白名单）。

**为什么不用 `pub use` 一劳永逸**：c260 HC-6 与 AGENTS.md 要求"不留兼容 shim"。但此处 re-export
是**架构层重组的过渡**（infra 自身还在用旧路径），不是新旧 API 兼容。白名单的 agent 条目删除即达成
HC-1 目标；infra 内部路径清理可在 c278 或独立小 change 做。design §6 标注此天花板。

## 5. `xml_escape` 归并

`xml_escape` 是 5 行纯函数，跨 prompt/skills 与 prompt/system 使用。归入 `core/source_info.rs`
（与 SourceInfo 同为"资源文本"语义邻近）或新建 `core/text.rs`。选 **`core/source_info.rs`**
（避免新建过碎模块；若将来有更多纯文本工具再提取）。

## 6. 不做的事（YAGNI / 高牵连，留给后续）

- **`SkillInfo`/`PromptTemplate`/`ThemeInfo`**（在 `infra/resource/loader.rs` 内与 loader 逻辑、
  ResourceLoader 缓存、解析逻辑混在一起）→ 抽出需重写 resource 模块，风险高，**本期不做**。
  相关白名单条目（prompt/skills, prompt/system, session/mod 的 `crate::infra::resource`）部分依赖，
  本期改指向 core 的部分（source_info/xml_escape）后，剩余 resource 类型条目保留在白名单，
  单列后续 change。
- **`CompactionConfig`**（嵌在 `infra/config/types.rs` 的 `AppConfig` 大 struct 内）→ 抽出牵连配置
  加载链，**本期不做**，白名单保留。
- **`CompactionSettings`**（`infra/settings/types.rs`）→ 同上，但 agent 的
  `From<infra::settings::types::CompactionSettings>` 实现可改为接受 core 类型；评估后若低成本则做。

**因此实际消白名单 ~10–18 项**（取决于 resource/config 抽出程度），剩余条目保留并标注后续。

## 7. 验证护栏

- 每迁一个类型：`cargo build --lib` 确认 core 无反向依赖（grep `crate::infra\|crate::agent` 在
  新 core 文件中为 0）。
- 每改一组 agent import：删对应白名单条目，跑 `arch_guard` 确认无新违规、无遗漏。
- 全程 BDD 88 场景不变（纯类型迁移，零行为变化）——任一失败立即停下。
