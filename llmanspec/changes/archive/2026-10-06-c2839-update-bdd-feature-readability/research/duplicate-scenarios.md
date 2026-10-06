# Research — BDD 重复场景候选清单（c2839）

## 方法论

- 对全部 feature 提取每个场景的步骤序列，归一化（去 `{param}`/引号/内联码）后按序列签名聚类；
- 同步序组 = 「步骤文本序相同」候选；再按组抽检内容判定**真重复**（步骤文本逐字重合）vs 必要边界重复。

## 统计

- 场景 947，同步序重复组 **72**（多数为 3 步对：`假如/当/那么` 骨架同形属**结构同形**而非语义重复）；
- 真重复（步骤文本逐字重合）：**×4 组 ×5 组**——如 before-hook 拒绝组（app-tui-trust:hook-point-remains-after-trust / app-tui-input:yolo-toggle-after-trust / agent-runtime:before-denies / infra-bash:bash-before-hook-rejects 步骤逐字一致）。

## 结论：跨 capability 不可物理合并

spec 单轨（每 capability 独立 feature 自证，r1813），跨 cap 同行为各自保留声明是**规范要求**——不做物理去重。处置建议（语义归属）：

| 候选组 | 语义真身 | 建议 |
|---|---|---|
| before-hook 拒绝 ×4（trust/input/runtime/infra-bash） | agent-runtime（hook 语义第一方） | 其余 capability 保留自证 + 可加 cross-ref 注释（可选，不强制） |
| 拖选/复制 ×4（infra-clipboard/package-tui-interaction-modes） | 端侧行为（interaction-modes） | 同为产品互证，保留 |
| otel session-id ×4（infra-otel 内部） | 同一 capability 内变体 | 保留（同 cap 变体不算冗余） |

**实施决策**：清单即交付物（不自动合并、不加 cross-ref）——保持 feature 自证完整性；语义去重留给未来 capability 边界重构时统一处理。

## 附：72 组原始清单

（步骤同步序聚类全表——实施时如需可导出；此处仅保留判定结论与代表性组。）
