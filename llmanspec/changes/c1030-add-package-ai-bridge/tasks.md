# Tasks — c1030-add-package-ai-bridge

- [x] 1. Delta + design + tasks 齐；`llman sdd validate c1030-add-package-ai-bridge --no-interactive`（apply 完成后收尾再 `--strict`）
- [ ] 2. 立包：`packages/xylitol-ai-bridge` Cargo + workspace member；`lib.rs` 模块骨架；单测证明零依赖主 crate `xylitol`
- [ ] 3. 定义包内 DTO：`BridgeMessage` / `BridgeChunk` / `BridgeUsage` / `TokenProvenance` / `ContextTokenEstimate`（名称可微调，语义锁在 design）
- [ ] 4. `usage`：OpenAI/Anthropic 字段归一化 → `BridgeUsage`；可选 cost(rates)；单测字段对照
- [ ] 5. `infra` 映射层：`Bridge*` ↔ `AgentMessage` / `XyChunk` / `XyUsage`；映射不变量单测
- [ ] 6. 迁 `Fake`（或等价）进包并经映射暴露为 `XyModel`；相关单测/BDD 保持绿
- [ ] 7. 迁 Anthropic Messages adapter 进包；主树改装配；`pa7` 级 vendor 隔离仍成立
- [ ] 8. 迁 OpenAI Responses + Completions；Completions 流请求 `include_usage`（或文档化兼容端缺失）；归档前主树无重复实现体
- [ ] 9. `accounting`：实现 Api → RemoteCount → LocalTokenizer → Heuristic；锚点失效规则单测；禁止流上每 delta 全量 encode（代码审查/护栏测试）
- [ ] 10. `tokenize` builtin：tiktoken + claude-tokenizer；registry 内置条目；未命中降级 Heuristic + warn
- [ ] 11. HF `tokenizer.json` 缓存目录 + **opt-in** 下载（镜像可配）；失败降级；不默认静默拉大文件
- [ ] 12. `RemoteCount`：至少 Anthropic `count_tokens` 路径或可注入 stub；失败降级；可配置关闭
- [ ] 13. 切换 `domain-compaction` / `session/stats` 估计入口到 accounting（经 infra/端口）；更新相关单测
- [ ] 14. Driver（或 core seam）预留只读 `ContextTokenEstimate` API 形状（无 footer UI）；文档指向下游 draft c1035
- [ ] 15. 更新 `src/AGENTS.md` / 包 `AGENTS.md` 指针；`just lint` / 相关 test；`llman sdd validate c1030-add-package-ai-bridge --strict --no-interactive`
