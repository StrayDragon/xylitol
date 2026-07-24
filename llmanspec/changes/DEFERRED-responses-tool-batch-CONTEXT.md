# Deferred cluster: Responses 优先 + tool 批开箱 + 提示片段 + 抢跑

> 状态：purpose-draft 族（2026-07-24 会话沉淀）。**不是**进度板；实现以各 `cNNNN/proposal.md` 为准。
> 触发会话：tool_batch ROI 复盘、方言误配、系统提示与抢跑边界。

## 背景（为何有这组 draft）

1. **`c1545` barrier_parallel 已落地**，Langfuse 证明同窗并行正确；但日常体感常被「模型单 tool/turn + 大 read 预填」淹没，批内并行 ROI 窄。
2. **误以为卡在「首个 tool 流水线」**——事实是 `ar21` MessageEnd 后批执行（与 pi 同构），不是流中抢跑。
3. **配置写 `openai-completions`，实际跑 Responses**：bootstrap 丢弃 `models.*.api`，`default_for(OpenAi)=Responses`。排障与「弃 Completions」方向被静默混淆。
4. **产品取向**：pi 重写 → 开箱合适特性、少配置面；开发 yaml 开试验档是临时态。能力切换应自动注入提示片段（非用户再贴 SYSTEM）。
5. **方言**：逐步以 OpenAI Responses 为日常默认；Completions 遗留。Responses 上 ToolCallEnd 更早，才值得谈流中抢跑（`c1615`）。

## 目标

| 目标 | Change |
|---|---|
| YAML `api` 真生效 | `c1598`（**本提交已接线代码**；draft 保留审计上下文） |
| Responses 推荐 / Completions 遗留 + 方言可观测一等 | `c1600` |
| 策略 ↔ 提示片段机制 | `c1605` |
| 开箱 barrier_parallel + 默认多-tool 片段 | `c1610` |
| 流中 ParallelSafe 抢跑（后置） | `c1615` |

## 依赖简图

```text
c1598 (fix api wiring) ──► c1600 (prefer Responses)
c1545 (batch scheduler) ──► c1610 (OOB defaults)
c1605 (fragment mechanism) ──► c1610
c1598 + c1600 + c1545 ──► c1615 (eager; optional)
```

## 实验分派与烟雾结果（2026-07-24）

详情：[`c1610/.../experiments.md`](./c1610-update-oob-tool-batch-defaults/experiments.md)。

| # | 内容 | 结果 |
|---|---|---|
| 1 | FIFO 慢读并行（`scripts/xylitol_batch_slow_fifos.py` delay=2） | session `95338a3a-…`：三 read 同 `barrier_index=0`，批墙钟 **2.001s**（非 ~6s）；`api=openai-responses` |
| 2 | End→Done 空隙 | 未跑（后置，决定 `c1615`） |
| 3 | APPEND_SYSTEM 多-tool | session `ec1351b6-…`：同消息 3×read，`toolCall 个数=3`；`api=openai-responses` |

## 观测要点

- Langfuse / fastrace：`llm.request` 属性 **`api`**（已有）；`c1598` 后应与 YAML 一致。
- 并行窗：`tool_batch.mode` + `tool_batch.barrier_index`。

## 本提交清单

- [x] c1598 代码 + 单测
- [x] `.xylitol/config.yaml` → `openai-responses`
- [x] `.xylitol/APPEND_SYSTEM.md`（实验 3）
- [x] FIFO 脚本 + purpose-draft 五件 + 本文件
- [x] 实验 1/3 烟雾
- [ ] 实验 2；c1600+ 完整 promote
