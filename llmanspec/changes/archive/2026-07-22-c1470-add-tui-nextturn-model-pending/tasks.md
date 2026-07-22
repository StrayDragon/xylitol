# c1470 Tasks（依赖序）

## 1. Agent：NextTurn 重读 model + thinking

- [x] 1.1 ReAct 在每 turn 模型调用前（turn 边界，对齐 pi `prepareNextTurn`）从 session 重读 selected model 与 thinking，重建 `XyModel` / `XyGenerateOptions`（或等价），**不得**整 run 握死首帧 snapshot
- [x] 1.2 in-flight 流仍用调用开始时的 snapshot；不中途拆 HTTP
- [x] 1.3 AgentEnd / abort 后无 in-flight：active 与 selected **收敛**（单测或 harness）
- [x] 1.4 单测：run 中途 `select_model` / `set_thinking_level` → 下一 turn 用新值；当前流仍旧

## 2. Model registry：默认最高档

- [x] 2.1 换模 / 精确 `/model <id>`：可调模型 thinking = 支持集全序 **最高**；不可调 = `off`
- [x] 2.2 修改 `m10` 行为：无合法 Settings 默认或默认不在支持集时，clamp 到 **最高档**（非随意低档）
- [x] 2.3 UI/footer **仅** xylitol 档名（既有 map 不变）

## 3. `/model` picker + 移除全局 cycle

- [x] 3.1 废止产品全局 `app.thinking.cycle` / editor Shift+Tab cycle（改 specs ati36、ath22、pad6）
- [x] 3.2 Models 槽：↑↓ 模型；可调时 ←→ / 槽内 Shift+Tab 选档；Enter 提交 `(model, level)`
- [x] 3.3 wide：焦点行铺开全部档；narrow：单档；无思考：`—`，方向键不改状态；footer 省略 thinking 段
- [x] 3.4 有参精确 id：SetModel + 默认最高 / off；非精确开列表预填（若本 change 时间紧可跟 pi 精确命中语义，否则保现有直设）
- [x] 3.5 busy：开列表策略保持 idle-only（或文档化）；busy 有参直设走 pending

## 4. Status trail + 双态 chrome

- [x] 4.1 active = in-flight（非裸读 Manager）；footer 显示 active
- [x] 4.2 agent-busy：status **lead**（spinner+短词）贴左；**trail** 右对齐仅 pending；禁止 lead 内 space-between
- [x] 4.3 trail 文案：模型优先 `Next turn: {model}`；仅 thinking `Next turn thinking: {level}`；禁止拼接
- [x] 4.4 bang-only 无 trail；status 短词轮换保留 trail；idle/abort 清 trail
- [x] 4.5 成功换模/thinking/theme：**无** System 确认行（theme 可同 change 顺手）

## 5. 校验

- [x] 5.1 `python3 scripts/check_tui_design_playground.py --check`
- [x] 5.2 相关 harness / `cargo test`（tui + agent 边界）
- [x] 5.3 `llman sdd validate c1470-add-tui-nextturn-model-pending --strict --no-check`
- [x] 5.4 `just fmt` + 相关 clippy

## 不做（本 change，非 task）

- SlashMeta 调度表重构
- Scope all/scoped；覆盖盘 M1+
