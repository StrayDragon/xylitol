# c93-fix-resource-boundary Tasks

- [x] read 工具：添加文件大小检查（`metadata().len()` vs MAX_FILE_SIZE），超限返回错误
- [x] grep 工具：同上，拒绝超大文件（10MB 上限）
- [x] bash 工具：timeout 参数校验正值 + 硬上限 MAX_TIMEOUT_SECS（已在 c91 实现）
- [x] bash 工具：UTF-8 安全截断（已在 c91 实现 floor_char_boundary）
- [x] grep/find：max_results 校验正值范围（.max(1).min(1000)），负值回退默认
- [x] `truncate_output`：使用 `str::floor_char_boundary()`（已在 c91 实现）
- [x] find 工具：禁止绝对 pattern（以 `/` 开头返回错误）
- [x] ls 工具：添加 max_entries 上限（默认 1000）
- [ ] session/storage.rs：zstd 解压大小限制（低优先级，defer）
- [ ] RepeatDetector：ngram_set 容量上限（低优先级，defer）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c93-fix-resource-boundary --strict --no-interactive`
