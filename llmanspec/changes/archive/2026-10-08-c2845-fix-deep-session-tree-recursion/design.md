# 设计：session_tree 深度安全（RAW 透明，拆除递归地雷）

## 根因链

1. v3 `TreeResult` 强 schema 的 `SessionTreeNode` 含 `children: [SessionTreeNode]` → 生成 codec 对每个子节点递归写。
2. 树深 ≈ 会话条目的父子链深；真实长会话可达数百层。
3. debug 构建 codec 帧 ~7KB×6 帧/层（~44KB/层）→ 默认 worker 栈（~2MiB）在 ~45 层即危险、~185 层必爆；abort 不可捕获。
4. `thread_stack_size(64MB)` 只延后（~1400 层），且让所有 worker 背一份没必要的大虚拟栈——治标。

## 方案（治本）

- 与 c2842 「命令身份透明化」同源原则扩展为**应答形状透明化**：`session_tree` 不再强 schema 化，直接 `RawOk` 原文（与 JSON 轨 `{"tree":[...]}` 同构）。
- 客户端：`payload_value(RawOk)` 现成路径 → 返回同一 JSON；driver 对 `value["tree"]` serde 消费（小帧递归，深度安全一个量级以上）。
- 深度的 fory 递归键仅剩 tree_result_to_v3/v3_to_tree_nodes（保留兼容 + 对拍），但在 session_tree 产品路径上不再被调用。
- 栈补丁回退：无递归即无栈依赖，默认 2MiB 正确。

## 守卫

- spec r1921 场景 `v3-session-tree-raw-parity`（BDD）：双轨取树 → v3 应答变体断言 **RawOk**（防回归 typed），两轨 result 领域等价（r1908 纪律），tree 形状存在。
- wire_v3 单测同步迁移断言直至 RawOk + 兼容解码面（强 schema 往返仍可用）。
- 实机验证：默认栈 + 185 层深会话 + v3 `/session-tree` 多轮/Esc 无崩溃、树渲染正常。

## 明确不做

- 不动 fbs/generated（TreeResult 类型保留，foryc 无必要）。
- 不动 get_messages/travel（flat，无递归风险，保持强 schema 收益）。
- 不加栈器、不加递归迭代重写生成 codec。
