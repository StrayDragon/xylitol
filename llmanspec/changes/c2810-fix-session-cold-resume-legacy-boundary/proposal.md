---
depends_on: []
needs_specs_change: true
---

# Session 冷段按需索引与旧格式边界

## Why

c2800 已将 session 存储切换为 manifest + active/sealed 段，但 sealed 段的按需恢复设计尚未闭环：普通 resume/context 在 leaf branch 读取之后仍会全量解析 cold 段来构造 session-wide bash done 集合。与此同时，v6 lazy migration 增加了大量一次性兼容与失败恢复分支，而产品仍处于 `0.0.0-pre`，当前尚无生产数据格式承诺。

本 change 选择**明确结束 v6 storage migration，零兼容一刀切**：旧 `.jsonl` 不被自动迁移、不被静默删除、也不被解析展示；访问时返回可操作的不支持错误并保留原文件。这样缩小实现面，同时把破坏性升级显式化。变更必须保持 manifest 为恢复 SSOT，并继续遵守单写者下的原子提交语义。产品处于 `0.0.0-pre`，无生产数据格式承诺——现在收口最便宜，不做任何过渡兼容层。

## What Changes

- 为 sealed segment 生成 immutable sidecar index，记录 `entryIds` 与 `doneBashIds`，并在 manifest 的 segment 描述中记录可选 `indexPath`。
- 让 leaf ancestry resolver 使用 `entryIds` 先筛选 cold 段；让 session-wide bash 配对使用 active 段扫描加 sealed sidecar，旧 manifest、缺失或损坏 index 回退扫描，不能产生 false negative。
- 取消 v6 storage migration、headerless repair 和 legacy cleanup retry；存在旧 `{id}.jsonl` 且没有 v7 manifest 时返回明确的旧格式不支持错误，保留源文件，显式 delete 仍可删除。
- **list 直接跳过不可解析条目**：`list_sessions` 发现无 manifest 的 legacy 文件或 v7 段不可读时，不做迁移、不做修复、不解析展示，直接跳过该 session 并记录有限诊断；不因单 session 失败拖垮整个列表。
- **恢复只走可解析 v7 的 latest leaf**：resume/context 仅从可解析 v7 session 的最新 leaf 分支按需恢复，并**继续投影 LLM API 相关信息**（`thinkingLevel`、`model` 选择）——sidecar/按需优化不得破坏该投影语义；legacy 一律拒绝自动恢复。
- 更新 session-store 规则与验收场景，明确 v7 是当前唯一自动恢复格式，旧文件不得被静默当作新 session 或自动删除。
- 为 sidecar、按需读取、索引 fallback、legacy boundary 和 manifest 失败恢复补回归测试；不改变 wire Command/Event。

## Capabilities

- `agent-session-store`：sealed sidecar、按需 cold resolver、list 跳不可解析条目与旧格式 boundary。
- `agent-session`：resume/context 的 session-wide bash done 配对继续保持原有可观察语义；latest leaf 恢复与 LLM API 信息投影（thinkingLevel/model）不被破坏。

## Impact

- 持久化格式：v7 manifest 增加可选 `indexPath`，旧 manifest 无该字段时继续可读；sidecar 仅由被新 manifest 引用的文件构成事实。
- 读取性能：compacted session 的普通 resume/context 不再无条件解析全部 cold JSONL；完整 load/export/inspect 仍可显式读取逻辑全量。
- 兼容性：**零兼容（一刀切）**——v7 继续使用 skip-warn 解析行；v6/v5/未知 legacy storage 不再自动迁移，返回可操作错误并保留旧文件；用户显式 delete 仍清理旧文件；list 直接跳过 legacy 条目，仅记录有限诊断。
- SDD：这是行为合约修复，需在绑定分支更新 `agent-session-store` 的相关 human 约束与 executable 验收场景。当前仓库已有的全局 `@human` / `@req` 校验错误作为 pre-existing baseline blocker 记录，不在本 change 中修复。
