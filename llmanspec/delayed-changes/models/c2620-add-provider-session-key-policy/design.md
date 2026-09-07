# Design: c2620 供应商键策略(缓存优先)

> **deferred（2026-09-08）**：与 proposal 同迁 `delayed-changes/models/`。公开预览不落地。

## Decision

键取值 = **命名策略表**(按供应商/轮廓分流),不是新 trait 市场:

| 键 | 今日 | 策略方向 |
|---|---|---|
| `x-opencode-session` | = 书签 UUID,子本必换新 header | 按轮廓决定 fork 后取值;同树干尽量同会话身份 |
| `prompt_cache_key` | 产品默认关 | 打开时按策略取(树干稳定身份),禁止书签 |
| `previous_response_id` | 产品默认关 | 按策略取;子本不继承父本**切点之后**的值 |

不变式(任一实现 MUST 保):
- 书签 UUID 不出现在任何供应商键与模型稿
- fork 子本不因「新书签」主动冷化前缀/网关缓存
- 策略以轮廓(`compat` 一类)表达,禁止 per-插件注册

## Open(实现期再钉)

- 各轮廓下 `x-opencode-session` 的具体取值算法(树干 id 哈希?父键继承?)
- DeepSeek / llama.cpp(无 header 族)是否只需「不发键」即达成缓存优先

## Non-goals

- 观测双 id 字段与树边(c2600)
- 缓存命中率保证(打网 lab 另证)
- COW 盘格式、fork 拷贝路径
