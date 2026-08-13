# pending

agent 仍在跑时换模 / thinking：footer = **生效中**；status 行右 = **下轮预告**。

## 可观察 MUST

- 仅 `selected ≠ active` 且 agent-busy 时画下轮预告。
- 互斥优先级：模型 pending → `Next turn: {model}`；仅 thinking → `Next turn thinking: {level}`。禁止拼接。
- 仅 bang busy：不画 `Next turn:`。
- 成功路径不刷滚动提示墙。
