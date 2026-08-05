# Design: c1930 Session ↔ Provider view 有序幂等转换

## 目标

用户 **resume / import session** 后，最终 Responses 请求的 **稳定前缀**（system + tools + 历史 `input`）须与「同进程续跑」在固定旋钮下尽可能一致，避免 Prompt Cache / KV 无故失效。

**本波不做**状态栏。字段级矩阵与实验臂见 [`landing.tmp.md`](./landing.tmp.md)。

## 架构（钉死）

```text
resume/import JSONL
        │  parse + build_context_entries
        ▼
SessionEntry[] → as_agent_message → AgentMessage
        │  project_for_llm
        ▼
Vec<AiBridgeMessage>
        │  ResponsesAssembler
        ▼
body.input / tools / reasoning…
        │  ProviderRequestTrace（可选）
        ▼
Langfuse observation.input + cache_read
```

## 前缀组件与本波职责

| 组件 | 本波 |
|---|---|
| P2 历史 `input` 序与内容、折叠文案、reasoning 序 | **主责**（合约 + 单测 + lab） |
| P0 system（含 date/cwd） | 声明敏感；实验写死 date；实现 → c1905 |
| P1 tools[] | 实验固定；产品冻表 → c1900 |
| P4 reasoning/include/store | 与 thinking_level 一致即可 |

## 不变量（摘要）

- 同冻结历史 + 同旋钮 → assemble 结果规范化相等（幂等）。
- compaction cut / abort 跳过集合稳定。
- 单路径折叠；禁止 infra 再折。

## 测试与实验

| 层 | 内容 |
|---|---|
| 单测 | fixture → project → assemble 两次哈希相等；bash/compaction 文案；reasoning 序 |
| Lab（不进 qa） | 扩展 `lab_resume_prompt_cache` 或新 example：内存续跑 vs 序列化 resume vs JSONL import 形；`live-provider.local.yaml`；可选 Langfuse 对比 input |
| 在线闸 | resume `cache_read` 不低于同条件续跑 / warm 基线（informational 地板同 c1925） |

## Out of scope

- `AgentStatusBar`、导出 strip 产品、Todo、tool_search、压缩算法、date 日界实现、`previous_response_id`

## Ethics 映射

- 禁止无故改稳定折叠串；禁止第二套折叠；不承诺 Env 可逆或全 compat cache。
