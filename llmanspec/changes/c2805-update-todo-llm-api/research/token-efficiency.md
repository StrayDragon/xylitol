# Token 效率（A3 上再削）

> 单位：紧凑 JSON 字符。÷4 ≈ token。S1 = n=12 建表 + 2n 次 tick。

## 三笔账

| 账 | 付费节奏 | S1 量级 | 怎么削 |
|---|---|---|---|
| 工具 **args** | 每次调用 | A1 rewrite 每步 ~1133；update 一条 ~52 | 热路径走 update |
| 工具 **result** | 每次调用 | 全表 ~1133；fat ack ~132；slim ack ~54 | 不回全表、不回 content |
| **tools[] + description** | **每个模型 turn** | 今日 ~1074/turn × 25 ≈ 27k | schema 不写长 description；guideline 一条 |

第三笔和一整场 A1 的 args 同量级。**多开工具名是永久税**，即使从不调用。

## S1 n=12 调用账

| | args | result | total |
|---|---|---|---|
| A1 仅 rewrite 回全表 | 28661 | 28661 | 57322 |
| A3 fat ack（ok/op/回 content） | 2405 | 3405 | 5810 |
| **A3 slim ack**（`{ids}` / `{changed:[{id,status}]}`） | 2405 | 1485 | **3890** |

slim 相对 fat 再砍 ~33% result；相对 A1 是 **6.8%**。

## tools[] 每 turn

| | schema 三工具 | description 三行 | 合计/turn |
|---|---|---|---|
| 现行 | 768 | 306 | 1074 |
| 提案 verbose（字段长说明） | 1059 | — | 更贵，否决 |
| **A3 slim schema + 短 description** | 769 | 192 | **961** |

guideline：一段 ~163 字符，只挂 update，避免 list/rewrite/update 各写一段进系统提示。

## 故意不削的

- status 枚举目标为 `pending|in_progress|completed`（本波去掉 cancelled；见 design D11）。
- id `t_`+8 hex（4 hex 在 update ack 上只省 ~5 字符）。
- `todo_update` 坚持 `items[]`（单条比旧 `{id,status}` 贵 ~12 字符；换来批量与单一形状）。
- `todo_list` 仍回全表 JSON（按需，不是热路径）。

## A5 的隐藏税

`todo_add` + `todo_remove` 即使闲置，也要进每 turn 的 tools[]。按 slim schema 估 +400–500 字符/turn。S1 的偶发追加省不下这个税。
