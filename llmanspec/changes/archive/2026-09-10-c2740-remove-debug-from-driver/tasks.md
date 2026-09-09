# Tasks: c2740-remove-debug-from-driver

> pre-start。依赖 `c2710`。需要 Specs landing（debug 退出 wire）。与 `c2730` 可能同时改执行器：优先串行或同一认领人。

## 1. 合约

- [x] 1.1 start 后：live spec 去掉产品 `/debug` 经 Driver/Host；标明 fixture 仅本地。
- [x] 1.2 [blocked-by: 1.1] validate strict。

## 2. 删除产品路径

- [x] 2.1 [blocked-by: 1.1] 删 Command/registry/host/remote/trait 的 debug scene。
- [x] 2.2 [blocked-by: 2.1] TUI slash：debug 仅本地注入；完成器/registry 同步。
- [x] 2.3 [blocked-by: 2.2] 测试改为 harness inject；断言产品 unary 无 debug method。

## 3. Host 泵（可选后半）

- [x] 3.1 [blocked-by: 2.3] 测入口复杂度；超闸则下沉 effects，不拆 crate。（已测：try_busy_input 44/25 超闸，为 busy 输入策略既有分支、与 debug 解耦无关 → 按 design 次段出口后置 follow-up，见 proposal Further Notes）
- [x] 3.2 若本 PR 不做 3.1，在 proposal Further Notes 写明 follow-up，**不要**留空 trait 方法占位。

## 4. 验证

- [x] 4.1 [blocked-by: 2.3] `just test-tui` + `just qa`。（均 exit 0，1927 tests）
