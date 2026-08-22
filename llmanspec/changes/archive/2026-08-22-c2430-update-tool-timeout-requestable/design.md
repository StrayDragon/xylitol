# Design

## 权限模型（用户裁决 2026-08-22）

```
模型传 timeout=N ──► 程序校验（正整数）──► 钳制 min(N, 600) ──► 必然武装计时器
省略 ──────────────► 工具默认（bash 120 / grep·find 60）
0 / 负数 ───────────► invalid_args 拒绝
```

- **钳制而非拒绝**：保留任务意图、不浪费交互轮次；上界即程序权威边界。
- `AboveMax` 错误变体随语义移除（唯一调用点在三工具的 map_err，改为只处理 ZeroOrNegative）。
- fs 四工具继续无模型参数（`wait_bound` 30s），不在本票范围。

## 合约同句

agent-tools：
- t24 → 「timeout 为模型侧请求：正整数秒被接受并钳制到全局上界 600；省略用工具默认（bash 120 / grep·find 60）；计时器必然武装，无无限语义。」
- r6 → 「…MUST 为正整数秒；超过全局上界的值 MUST 被钳制到上界；0/负数 MUST 拒绝。」

## 测试影响

- protocol 单测：above_max_err 改为 clamp 断言。
- BDD negative-timeout / zero-timeout-rejected 场景仍绿（拒绝路径未动）；bash-timeout（显式 1s）不变。
