# Design — compaction 切簇独立成块

## 方案

切簇点集合 = {可展示 assistant 正文} ∪ {Compaction}。`partition_segments` 的 match 循环中，
`UiEntry::Compaction` 分支：先 `seal_open(None)` 封口当前打开簇，push 自身后立即再
`seal_open(None)`——形成单例簇；后续 middle 另起新簇。

单例簇的呈现零新增逻辑：`ActivityCounts::omits_cluster_header()`（summary.rs:45）对
纯 compaction 簇已返回 true，scrollback plan（mod.rs:230-233）对 omit_header 簇直接
不登记 skip_middle——compaction 块按普通块绘制。att23 例外句从「罕见形态」变为「唯一形态」，
条文不变。

## 备选与取舍

- **A. compaction 完全退出簇模型**（envelope 直挂块、不进 clusters）：动 ActivitySegment
  形状、scrollback plan、att23 嵌套不变量与 live/rebuild 同构证明——影响面大，否。
- **B. 切簇点扩展（所选）**：单点改动 + 复用既有 omit_header 直露路径；live 与 resume
  走同一 partition 函数，同构性免费获得。

## 边界与风险

- 连续多条 Compaction：各自单例簇（封口语义同构，不合并）。
- compaction 后尾簇重新计数（"Exploring 1 file" 从零起算）：att35 计数语义本就按簇统计，自然成立。
- 既有测试影响面：att29 三处测试无 User 条目或纯 compaction 轮，不成混合簇，预期不受影响；
  如有 pin 破坏以测试输出为准修订。
- 不改 `docs/architecture/TUI信息呈现与固定区词汇.md`（无新词形；reason 修饰词本次不做）。

## 测试 seam

- `segment.rs` 单测：tools→compaction→tools 三簇；连续两条 compaction；纯 compaction 轮
  （与现状同构）。
- BDD `@executable`（att34）：SceneBuilder 增 `compaction` 构造 → when「以场景构建器回放
  工具后压缩再工具序列」→ then 断言两簇头 + compaction 块独立呈现、无 Worked for。
- 复用既有 harness：`render_plain`（80 宽纯文本帧）与 `TranscriptBdd.frames`，不发明新 seam。
