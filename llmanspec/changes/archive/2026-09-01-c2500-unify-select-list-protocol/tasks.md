# Tasks — c2500 SelectList 结构下沉（行为零变化）

- [x] 1. 行为差异清单：核对五槽 + 包层 SelectList 的组装参数、过滤语义、渲染辅助调用点，
  形成「逐字重复 / 仅参数不同 / 行为不同」三分类清单（design D1 依据）。
- [x] 2. 过滤纯函数化：`set_filter` 前缀匹配抽包层纯函数 + 单测；`fuzzy_filter` 不动；
  调用点行为不变。
- [x] 3. 组装样板下沉：theme / layout / truncate 上下文构造收敛为共享构造路径，
  五槽接线点替换为等价调用。
- [x] 4. 行为零变化验证：五槽 harness 快照与重构前零 diff；包层单测 + `just test` 全绿。
- [x] 5. 门禁：`just fmt` / `just lint` / `just test`。
