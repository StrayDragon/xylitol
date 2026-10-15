# Design: c2921-refactor-hooks-wiring-scenario-outline

## 决策

### D1: 两个大纲按「观察 / 取消」分型，不合并

观察型与取消型的 then 谓词不同（`hook 被调用且上下文含键 X` vs `操作失败原因包含 X`），given 形态也不同（纯注册 vs 注册+block 返回）。合并单表需「谓词选择列」或稀疏列，可读性劣化；两表各 6/3 行与 r1815 规则句「观察型 / 可取消」的二分法直接对应。

### D2: model-select / thinking-select 并入观察矩阵，双事件串联场景不动

`model-and-thinking-dispatch`（agent-hooks r1005）是单场景内两段 given-when-then（model_select 后接 thinking_level_select），表达「同一次运行内先后两类分发」，矩阵行是独立运行语义，不可混同——该场景保持原样。test-hooks-wiring 侧的 `model-select`/`thinking-select` 是两个独立单发场景，与矩阵行同构，收敛。

### D3: 占位符无类型提示（`.+?`），非 `:string`

c2920 先例（`{flag}`/`{expect_compact}`）。理由：观察矩阵 event 值（`session_start`）与 op 值（`切换会话 target`）在 spec 里均不带引号，`:string` 会强迫表格值写引号形式；无类型提示在整句锚定 + 尾部单占位符下无歧义。运行时 `max_by(specificity)` 保证新参数化 step 不抢占老字面变体（字面字符更多者胜）。

### D4: 旧 then/given 变体保留

`agent-hooks` 的 `react-script-bridge-tool-call` / `model-and-thinking-dispatch` / `tree-and-switch-before-block` / `user-bash-blocks-run` 与 r1816 组合 given（`pre.tool_call 的 hook 且返回 block 不允许` 等）仍引用字面变体；删除会牵连无关 capability。字面变体与参数化版并存由 specificity 消歧，无运行时冲突。

### D5: unknown-op 不入矩阵

其 given 缺席（无注册步骤）、断言文案（`未知操作`）非 block reason 语义，硬入取消矩阵需 nullable 列。单例保持。

## 权衡

- 备选「连 agent-hooks 三场景一起大纲化」：跨 capability（r1003/r1005/r1006 各需保留场景句柄），且 r1005 是双事件串联——收益不抵语义扭曲，留待后续按需。
- 备选 `op` 列 `:string` 化：与 c2920 已验证通路不一致，且强迫 spec 值带引号，弃。
