# Design — interrupted bang 的 LLM 短提示

## 1. 折叠落点与缝

- 配对 + 模板做成**纯函数**（无 IO，输入消息列表 + done 集，输出折叠结果），落
  `agent::llm_project`——env→LLM 折叠形状与文案的既有 SSOT（c2725 起 text
  templates 集中于此；as48「稳定折叠串 MUST NOT 无显式 change 改」的本体）。
- 调用点两处，均在「SessionEntry → AgentMessage」装配完成后、送达模型前：
  - `agent/capabilities/session_ops.rs load_conversation_history`（as45 ReAct 播种路径）；
  - `infra SessionManager::build_session_context`（as48 单路径一致性；TUI restore / 统计消费面）。
- 不改 `EnvMessage::llm_projection` 的单行语义（`Running → None` 保持）：跨行
  bash_id 配对不进单行投影，由纯函数在列表级做一次。

## 2. 配对口径（防误报）

- **会话级 done 集**：orphan = 该 running 行的 `bash_id` 在**整个会话**（store 全量
  条目）中无任何 done 行。崩溃场景下 done 从未写出，会话级即路径级；而 fork/travel
  到早于 done 行的叶子时，路径上孤儿 running 若按路径级配对会误报 interrupted——
  会话级配对杜绝该误报（done 存在即不提示，done 行自身按既有语义投影或被裁切）。
- **投影窗口 = 裁切后列表**（`build_context_entries` 之后）：孤儿 running 在窗口内
  才产出提示，被 compaction 裁掉的 running 不复活。
- **稳定性**：done 集与会话内容同源；崩溃会话不再写入 ⇒ done 集不变 ⇒ 重复构建
  逐字节一致（前缀不变量）。正常会话（无孤儿）投影零变化。

## 3. 提示形状

- 单条 Env 行，模型可见文案钉死为 `[interrupted] $ <command>`（command 原文，
  不含前导 `!`），无 output 段。具体 Env 载体（既有 `EnvLlmProjection` 变体复用或
  新形状）由 apply 按 llm_project 既有折叠器最小改动选择；模板字节即 spec。
- `exclude_from_context=true` 的孤儿 running 不投影（`!!` 语义优先于提示）。
- 旧条目无 `bash_id`/`status` 字段视为 done（c2760 既有 serde 默认），永不产出提示。

## 4. 不做（沿 proposal 非目标）

- 不写回 session JSONL（提示仅投影层组合，不落盘）。
- 不改 UI interrupted 渲染、running/done 双 entry 写入与 `session/bash_output` 事件。
