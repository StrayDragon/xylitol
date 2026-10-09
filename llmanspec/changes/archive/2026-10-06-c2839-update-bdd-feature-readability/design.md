# Design — c2839 BDD feature 可读性治理（收敛范围）

## 调研结论（修正早期估计）

| 项 | 早期估计 | 精修实测 | 治理 |
|---|---|---|---|
| 格式噪声 | 32 处/agent-tools | `@req` 紧贴全局仅 3 处 | 修 3 处 |
| 重复场景 | 72 组同步序 | ×4 真重复（before-hook/拖选/otel-session-id）；**跨 cap 不可物理合并** | research 清单 |
| 大文件导航 | 7 个 >300 行 | 8 个（agent-tools/server-core/domain-compaction/app-tui-input/app-tui-host/app-tui-transcript/agent-session-store/agent-runtime） | 头部语义索引注释 |

## 决策

### D1：仅修复确定性格式缺陷（3 处 @req 紧贴）
不动其余布局（场景前无空行是既有可执行示例形态，非噪声）。

### D2：头部语义索引而非物理分区
在 8 个 >300 行文件头加注释块（`# sections: …`），按既有规则标题归组。
不动文件内部顺序/间距——**零改写风险**（对比 585 行内插分区线）。

### D3：重复候选清单（research 产物，不做机械合并）
判定真伪（抽检 ×4 组证实 before-hook 拒绝组为真重复）、标注归属建议
（该行为语义属 agent-runtime，其余 capability 保留自证或 cross-ref）。

## 验证
BDD 947 全绿（内容不变仅注释/空行）+ literal-bytes 门禁 + validate strict。
