# Design — Todo LLM API

## 范围

重做 LLM 工具面（schema、回包、id、guidelines）。
`TodoUpdated` / `agent_todo` 仍全量快照。待办栏三区、content ≤80、四态不动。
message 绑定 / 嵌套 / 额外条目字段 park。
**不**为旧 `{id,status}` 单对象形状做兼容升格。

## 原则

1. 一工具一动词；schema 直给，不用 oneOf。
2. 数组即批量；单条 = 长度 1。
3. 服务端发 id，回包带回；模型不必为拿 id 而立刻 list。
4. mutate 对模型回短 ack；全表只在 `todo_list`（以及事件/SSOT）。
5. `additionalProperties: false`。
6. **tools[] 与 description 每轮都付费**：字段说明能短则短；WHEN 只写在一行 description + 一条 guideline。不把同一段话写进 schema / description / Guidelines 三份。

---

## Token 效率（在 A3 上再削）

数字：`research/ablation-sim.md`、`research/token-efficiency.md`。单位 JSON 字符。

| 成本源 | 何时付 | 结论 |
|---|---|---|
| rewrite 整表 args | 每步若走 rewrite | S1 n=12：A1=57k。这是要消灭的热路径 |
| mutate result 全表 | 今日每步 | A2 只把 S1 打到 55%；ack 才打到 10% |
| fat ack（ok/op/回 content） | 每步 update | S1 5810 → slim 3890。值得，但小于「别回全表」 |
| **tools[] schema + description** | **每个模型 turn** | 今日约 1074/turn；25 turn ≈ 27k，和一整场 A1 args 同量级 |
| 多开 `todo_add`/`todo_remove` | 每个 turn 的 tools[] | 热路径零收益，却是**永久税**。A5 否决的 token 理由 |

落地规则：

- 回包：rewrite `{"ids":[…]}`；update `{"changed":[{"id","status"}]}`。不回 content，不加 ok/op/count。
- schema：字段不写长 description；`after_id` 语义放在 `todo_update` 的一行 description。
- guideline **一条**（挂在 update 上即可）：rewrite 建表、update 勾 id、没有 id 才 list；禁止为勾一项而 rewrite。
- id 仍 `t_`+8 hex（再短到 4 hex 只省几个字符，不值得）。
- 不加 A5：多两个 schema 每轮都付。

---

## 消融：最少几把工具就够

编码 agent 的 Todo 使用频率（从同行 + 自身现状归纳，不是遥测）：

| 意图 | 频率 | 若没有专用工具，用什么代替 |
|---|---|---|
| 写下/推翻整张计划 | 每任务 1 次 | —（必须有「整表写」） |
| 勾一条进度（status） | **热路径，每步** | 整表重发 |
| 读当前表/id | 丢了 id 时 | 上一次 mutate 若回全表则可省 |
| 中途追加一项 | 偶发 | 整表重发（带上旧项） |
| 丢掉一项 | 少见 | 整表不带该项 |
| 重排 | 少见 | 整表按新顺序 |

热路径是 **tick status**。add/remove/reorder 用整表表达，token 浪费是偶发的。

### 配置

| 编号 | 工具 | 建表 | tick | 追加/删 | 读 | 热路径 args | 说明 |
|---|---|---|---|---|---|---|---|
| A1 | 仅 `todo_rewrite`，回全表 | rewrite | rewrite O(n) | rewrite | 结果里有 | 每步 O(n) | Codex `update_plan` / 旧 TodoWrite。最少把数=1 |
| A2 | `rewrite` + `update`，回全表 | rewrite | update O(1) | rewrite | 结果里有 | tick 便宜、读仍 O(n) | 可删 `todo_list` |
| **A3** | **`list` + `rewrite` + `update`，mutate 回 ack** | rewrite | update | rewrite | list | **tick O(1)+ack；读按需** | 现行三名；每把一个动词 |
| A4 | A3 + `todo_add` | rewrite 或 add | update | 追加便宜；删仍 rewrite | list | 删仍 O(n) | |
| A5 | A4 + `todo_remove` | 五工具 | update | 增删都 O(1) | list | 闭集 10→12 | 完整增量 |

A1 不解决提案里的热路径浪费。A2 省掉 list，但每次 tick 仍把全表塞回上下文（和今日一样贵在 result）。**真正同时满足「少工具」和「热路径便宜」的最小集是 A3。**

A3 里三把工具的意图已经正交，不必再拆 add/remove：

| 工具 | 唯一动词 |
|---|---|
| `todo_list` | 读全表 |
| `todo_rewrite` | 整表替换（建表 / 推翻 / 追加删改重排 / `[]` 清空） |
| `todo_update` | 只改已有行（status / content / 位置） |

追加一项 = rewrite 时带上旧行 + 新行（偶发，可接受）。热路径禁止走 rewrite。

### 建议切点

**落地 A3。** 不扩闭集。不把 add/remove 塞进 `todo_update`（一口多意图，已否决）。
A5 留给「rewrite 仍被模型当 TodoWrite 每步狂发」时再加，不提前。

---

## A3 目标 schema

### `todo_list`

```json
{ "type": "object", "properties": {}, "additionalProperties": false }
```

结果：`{"items":[{"id","content","status"}, ...]}`

### `todo_rewrite`

完整计划一次性写下。省略 id → 服务端铸造；带旧 id → 保留身份。

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["items"],
  "properties": {
    "items": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["content"],
        "properties": {
          "id": { "type": "string" },
          "content": { "type": "string" },
          "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
        }
      }
    }
  }
}
```

结果：`{"ids":["t_…"]}`（清表 `{"ids":[]}`）。不回 content、不加 `ok`/`op`/`count`（工具名已在 envelope 里）。

### `todo_update`

只 patch。每条必须有 `id`，且至少改 `status` / `content` / `after_id` 之一。
**没有 id 的项直接拒绝**（不要猜成 add）。

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["items"],
  "properties": {
    "items": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["id"],
        "properties": {
          "id": { "type": "string" },
          "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] },
          "content": { "type": "string" },
          "after_id": { "type": "string" }
        }
      }
    }
  }
}
```

未知 id 或一条没有任何改动字段 → 整批拒绝、不落盘。

结果：`{"changed":[{"id","status"}]}`。不回 content（模型刚写过或本来就有）。move 只改位置时 `status` 可省略，只回 `id`。

旧调用 `{id, status}` **不**静默接受；description 写明 `items` 数组。

## id

`t_` + 8 位小写 hex，写入内唯一。禁止再生成 `todo-{idx}`。
旧快照里的 `todo-1` 当普通 id 继续用。

## Prompt

`description()` 各一行（进 Available tools 与 tools[].description，每 turn 付费）：

- `todo_list`: `Read the session checklist.`
- `todo_rewrite`: `Replace the whole checklist (new plan or []). Do not tick one item.`
- `todo_update`: `Patch existing items by id. items[{id, status?, content?, after_id?}].`

`prompt_guidelines` 只挂在 `todo_update` 一条：

`todo_rewrite for a new/replaced plan; todo_update to tick existing ids; todo_list only if ids are unknown. Do not rewrite the list to mark one item done.`

`todo_list` / `todo_rewrite` 不再各写一段 guideline（避免三份重复）。

## 端口 / 事件 / 展示

`list` / `rewrite` / `update(patches)`。成功仍 persist 全量 + `TodoUpdated { list }`。
对模型编短 ack，不把全表塞进 result。

att36：`todo_list` 仍画全表勾选；rewrite/update 按 ack 的 ids/changed 画受影响行；
清表 `(cleared)`。header：rewrite 继续 `N items · M in progress`；update 用
`{n} updated`（args 是 `items` 补丁数组，不是全表）。

## 否决

| 备选 | 原因 |
|---|---|
| A1 只 rewrite | 热路径每步 O(n)，提案要修的就是这个 |
| A2 不设 list、mutate 回全表 | 省一把工具，result 仍然每步 O(n) |
| 在 `todo_update` 里省略 id=add | 一口多意图 |
| 本波做 A5 五工具 | 热路径收益 0；两个新 schema **每个 turn 都付税** |
| `TodoUpdated` 改 diff | 端投影，不是模型面 |

## 风险

- 模型训练先验是 TodoWrite 整表 → rewrite 承接该意图；guidelines 把 tick 赶到 update。拦不住就再评估 A4/A5。
- 旧 `{id,status}` 调用会失败（有意，无兼容层）。

## D11 去掉 `cancelled`（用户选定）

领域与 LLM enum 收成三态：`pending | in_progress | completed`（对齐 Claude/Codex）。
软取消不再是一个 status：**从清单拿掉 = `todo_rewrite` 不带那一行**。

- `TodoStatus::Cancelled` 删除；glyph `[-]` 删除。
- 待办栏前翼只含 `completed`；翼头不再出现 `· n cancelled`。
- `done_total` 只计 completed。
- **旧快照**：`status: "cancelled"` 的行在读取时 **丢掉该行、保留其余行**。
  禁止因一条 cancelled 让整份 `agent_todo` 反序列化失败（`latest_agent_todo` 会丢整表）。
  下一次成功写入按三态落盘。不单独做 JSONL 改写脚本。

改 r1118 状态闭集、r1129 前翼定义。
