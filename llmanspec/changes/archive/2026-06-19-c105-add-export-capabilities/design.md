# Design: Export Capabilities

对齐 pi `export-html/` + `exportToJsonl` / `import` / `share`。

## 决策

1. **HTML 渲染范围**：手写转义（ANSI→HTML 实体/色码基础转换 + tool 块 `<pre>` 包裹）。不引入 syntect/highlight.js（P2 范围，后续可选）。
2. **JSONL 作为 canonical 交换格式**：`export_jsonl` 与 `import_jsonl` 互为逆运算；JSONL 带 `version` 字段，import 校验版本兼容（复用 SessionManager 既有迁移机制）。
3. **share stub 策略**：`share_as_gist()` 本期只返回"未配置 GitHub token"引导，不实现网络上传（需 HTTP + token 管理，属 P2 utils）。接口先稳定，后续接入。
4. **模块位置**：放 `infra/session/export/`，与 session 持久化同层；AgentSession 仅薄封装调用。
5. **导入新建 session**：`import_from_jsonl` 创建新 session_id，不覆盖现有（防误操作）。

## 不做

- 不做 gist 真实上传、不做 HTML 主题/语法高亮、不做跨版本 entry 迁移（复用现有）。
