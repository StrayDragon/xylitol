# Design — c230-refactor-module-cohesion

## 决策:session.rs 拆分粒度

**采纳:facade + 协作组件**(非完全拆散)。

`AgentSession` 保留为唯一 public 入口(facade),内部委托给聚焦组件:
- `ModelManager` — 模型注册、切换(cycle/select)、thinking level
- `ToolManager` — 工具注册、过滤、allowed/excluded
- `CompactionOrchestrator` — 压缩阈值检查、触发、与 SessionManager 协作
- `SkillManager` — skill 激活、prompt 注入
- `SessionIO` — 持久化、导入导出、命令分发

**为何保留 facade**:现有事件流(`AgentEvent`)、public 方法、BDD 场景都绑定 `AgentSession`。完全拆散会破坏调用方契约,facade 模式让重构对外无感(测试套件即验证)。

**替代(否决)**:完全消除 AgentSession,让调用方直接持有多个组件——破坏面太大,违反"零行为变更"目标。

## 决策:compaction.rs 拆分边界

按**职责**而非按函数大小切分。每个产出模块对应一类可独立测试的职责:
- `token_estimator` — `estimate_tokens` 系列
- `cut_detector` — `find_cut_point` 及 entry 类型判定
- `file_ops_tracker` — `<read-files>`/`<modified-files>` 提取
- `llm_summarizer` — `generate_summary` / `serialize_conversation` / 迭代更新
- `branch_summarizer` — 分支摘要
- `message_converter` — `SessionEntry` ↔ `AgentMessage` 转换

共享的纯函数(阈值比较等)留在模块根部或独立 `util`。

## 决策:compaction 配置统一

现状三类型:
1. `infra/config/types.rs::CompactionConfig` — AppConfig.compaction(YAML 加载期,`#[cfg(feature="infra-session")]`)
2. `infra/session/config.rs::CompactionConfig` — **同名第二个**(session 内部默认值)
3. `infra/session/compaction.rs::CompactionSettings` — 运行时实际喂给 `should_compact` 的类型

**采纳方案**:
- **保留** `CompactionSettings`(运行时真相,`should_compact` 直接消费)。
- **保留** `AppConfig.compaction`(YAML 加载期面),但类型名沿用 `CompactionConfig`,作为唯一的加载期结构。
- **删除** `session/config.rs` 中重复的 `CompactionConfig`(意外同名第二个)。
- 在 `SessionManager` 初始化处提供**唯一映射** `CompactionConfig → CompactionSettings`,并加注释说明加载期/运行期之分。

**边界准则**:`AppConfig.*` = 不可变加载期(YAML/schema 校验);`Settings`/`*Settings` = 可变运行期(三层 merge、热更新)。两者职责不同,都保留但消除重复定义。

**替代(否决)**:合并 AppConfig 与 Settings 为一体——违反"加载期不可变 vs 运行期可变"的设计分离,且 Settings 有 camelCase/merge/locking 特性不适合 YAML 面。

## 风险与回滚

- session facade 保留 → public API 不变,测试套件是强保障。
- compaction 拆分纯文件移动 + 可见性调整,无逻辑变更。
- 配置统一删除的是未使用的重复定义(session/config.rs 的 CompactionConfig 若有引用需一并迁移到映射点——apply 时由编译器定位)。
- 回滚:git revert 整个 change。
