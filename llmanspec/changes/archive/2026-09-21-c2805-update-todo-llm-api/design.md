# Design — Todo LLM API

## 范围

重做 LLM 可见面：**两工具**（rewrite / update）+ **请求时尾插**当前清单。
`TodoUpdated` / `agent_todo` 仍全量快照。待办栏三区、content ≤80 不动。
message 绑定 / 嵌套 park。不为旧 `{id,status}` 单对象做兼容升格。
**不**实现 delayed `c1895`–`c1898` 状态栏子系统（吸收说明见 `research/delayed-status-bar-absorb.md`）。

## 原则

1. 一工具一动词；schema 直给，不用 oneOf。
2. 数组即批量。一次动作一把工具。
3. n 很小：贵的是 **tools[] 每 turn** 和 **多一次工具往返**，不为运行时 O(n) 优化。
4. 读表不占工具名：最新清单在出站 generate 时注入 provider messages。
5. 不拆 add/remove。增删 = 一次 `todo_rewrite`。
6. `additionalProperties: false`；字段不写长 description。
7. 不另开注入通道：无 `todo_list`、无 `statusline_refresh`、无 Agent 列 publish、无 persist 栏消息。

## 模型怎么看见清单

**AgentStatusBar** 就是出站读数袋 + 至多一条末尾 meta。类型 `AgentStatusBar`，入口 `project_outbound`。Todo 是子树，不是根、不是假 user 回合。

三条缝，禁止第四条（`project_for_llm_with_*`、插件注册表、每跳 persist 栏消息、Agent 列 publish）：

| 缝 | 何时用 | 今日落点 | MUST NOT |
|---|---|---|---|
| **A 稀疏落盘** | 低频真变、resume 要看见、可进 cache 前缀 | `session_env` | 每 generate 都 append；塞进 system |
| **B AgentStatusBar** | 高频、或 SSOT 已在别处 | `AgentStatusBar` → `render` → **至多一条** user 行 | 落 JSONL；每个读数一条假 user；进 `EstimateOpts` |
| **C TUI 固定区** | 给人看的壳 | 待办栏 / 状态条 / 页脚… **各 widget** | 当 LLM 通道或业务 SSOT |

Todo 走 **B + C**。业务 SSOT 是 `agent_todo` Custom。TUI 待办栏三区无 id；XML 表序 + id + status。

History fold 仍是 `project_for_llm`。B 在其后。compact 摘要不跑 B。token 估计与 generate 同形（栏从**全叶**抽）。

扩展：`AgentStatusBar` 加字段 + `render` 穷举 `match`。新读数是根下新子标签（如 `<goal>`），**不是**第二条 user 行。

形状：

```xml
<agent_status_bar>
<todo>
<item id="t_a1b2c3d4" status="in_progress">do the thing</item>
<item id="t_e5f6a7b8" status="pending">next</item>
</todo>
</agent_status_bar>
```

空表省略整行。同 turn mutate 仍回全表 `{items}`；栏给下一跳出站。

砍掉的 c1895 壳：persist 栏消息、注册表、独立 session kind、always-on clock / tool_calls。

## 两工具

| 工具 | 动词 | 一次调用能做完 |
|---|---|---|
| `todo_rewrite` | 整表替换 | 建表 / 推翻 / 追加或删几项 / `[]` 清空 |
| `todo_update` | 按 id patch | 一批改 status/content/位置（完成当前+开始下一项 = 1 次） |

默认闭集 **9** 名：`read` `bash` `edit` `write` `grep` `find` `ls` `todo_rewrite` `todo_update`。
内部 gateway 仍可 `list()`（注入 / 测试）；**不**对模型暴露 `todo_list`。

### `todo_rewrite`

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

省略 id → 服务端铸造；带旧 id → 保留身份。结果：全表 `{items}`。

### `todo_update`

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

每条必有 id，且至少改 status / content / after_id 之一。未知 id 或全无改动字段 → 整批拒绝。
结果：全表 `{items}`。旧 `{id, status}` 顶层对象不静默接受。

## id

`t_` + 8 位小写 hex。禁止 `todo-{idx}`。旧 `todo-1` 当普通 id 继续用。

## Prompt

- `todo_rewrite`: `Replace the whole checklist (new plan or []). Do not tick one item.`
- `todo_update`: `Patch existing items by id. items[{id, status?, content?, after_id?}].`

guideline 只挂 update 一条：

`todo_rewrite for a new/replaced plan; todo_update to tick existing ids (batch in items[]). Do not rewrite the list to mark one item done. The current list is under <todo> in the <agent_status_bar> message when non-empty.`

## 端口 / 事件 / 展示

gateway：`list`（内部）/ `rewrite` / `update(patches)`。成功 persist 全量 + `TodoUpdated { list }`。
对模型的 mutate result 是全表 `{items}`。工具块 att36 仍画 glyph+content。

header：rewrite 仍 `N items · M in progress`；update 用 `{n} updated`。
rewrite/update 流式未齐无 `items` → 空摘要。不再有 list 无参调用。
空表 body 用 rewrite `items: []` 的成功结果覆盖（`(empty list)`）。

## D11 去掉 `cancelled`

三态：`pending | in_progress | completed`。删一项 = rewrite 不带该行。
旧快照 `cancelled` 行读取时丢掉该行，不得让整表反序列化失败。
glyph `[-]` 与翼头 `· n cancelled` 删除。前翼只含 completed。

## 否决

| 备选 | 原因 |
|---|---|
| 保留 `todo_list` | 每 turn schema 税；读表已由尾插覆盖 |
| slim ack | 省 result 换来额外往返 |
| persist `<agent_status_bar>` 进 transcript | 额外注入 + 压缩堆积（c1897）；SSOT 已有 Custom |
| 塞进 system / session_env | 伤稳定前缀；env 是 date/cwd bootstrap |
| 升格 c1895 全栏 | coding 仪表盘 ROI 低；本波只要 Todo 可见 |
| todo_add / todo_remove | 常驻 schema 税 |
| update 省略 id=add | 一口多意图 |

## 风险

- 模型仍可能每步 rewrite → guideline 把 tick 赶到 update。
- 旧 `{id,status}` 单对象会失败。
- 历史会话若曾调用 `todo_list`，工具块重建仍按未知/旧名渲染，不要求复活该工具。
