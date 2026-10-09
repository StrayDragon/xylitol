# 设计：生效模型状态同步

## 目标与判定

产品端「开箱即用」：用户显式配置了 `models.default_model` 就该看到自己被解析出的默认在跑，而不是 NOT-SET + 悄悄用一个隐藏模型。本 change 只解决**已配置默认模型的状态可见性**，不改变执行语义（执行本来就该用配置默认）。

## 方案权衡

| 方案 | 行为 | 代价 | 判定 |
|---|---|---|---|
| A（本文）：装配/绑定时同步 ModelSelect 下行 | 徽标显示已解析默认 | 服务端事件即可，复用现有 `session/event` + v3 映射（均已存在） | ✅ 最小、收敛、符合 r1387 修订后语义 |
| B：保留 NOT-SET + run 前提示 | 仍需隐藏执行 | 只满足 r1380「提示」但执行/显示仍背离 | 淘汰 |
| C：get_state 改读写者模型 | 需先解决 v3 其它方法的会话路由（更大面） | 依赖面大，本次不做 | 记后续 |

方案 A 不打 schema、不 bump 协议、（在 v3 映射完备前提下）纯服务端广播 + 客户端既有 apply 面，是收敛最大、风险最小的一步。

## 同步点与幂等

- `materialize_writer_at` 装配写者成功且 `restore_model(default_model_id)` 生效后，若会话已有订阅者，向该会话广播一帧模型同步（ModelSelect）。
- `bind_mux_to_session` 绑定新订阅者并完成 replay 后，若写者已装配且持有当前模型，向该订阅者下发一帧模型同步——覆盖 attach / 重连 / 会话切换，保证徽标收敛不依赖 get_state。
- 事件走 `append_and_push`（journal 化、seq 单调），复用现有下行管线；对不识别 ModelSelect 的旧客户端，v3 映射与文本轨均按既有降级语义处理，不断链。
- 显式 `set_model` / `cycle_model` 的现有 ModelSelect 广播优先于/叠加于同步（同一会话内后到者胜），无冲突。

## 显示语义（spec 层）

- 用户显式配置默认（`models.default_model` / profile 默认）＝ 可解析的「已配置默认」→ 产品面显示为当前模型。
- 仅凭 provider 环境变量可映射、但无任何显式配置的厂商默认（gpt-4o 等）→ 仍 NOT-SET（维持 r1387 既有禁令，防止「未配置却假装选了」）。

## 不做

- v3 其它 command_backed 方法的会话身份保真（get_state/set_model/steer…仍回落 fallback）：独立主题，后续 change。
- `get_state` 返回写者当前模型：依赖上一项路由保真，后续一并处理。
