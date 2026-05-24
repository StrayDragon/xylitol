# c93-fix-resource-boundary Tasks

- [ ] read 工具：添加文件大小检查（`metadata().len()` vs MAX_FILE_SIZE），超限返回错误
- [ ] grep 工具：同上，拒绝超大文件
- [ ] bash 工具：改用 `Command::spawn` + 流式 stdout/stderr 读取 + 字节上限 + kill
- [ ] bash 工具：timeout 参数校验 `1..=config.security.bash.timeout_secs`，使用 `u64::try_from`
- [ ] grep/find：max_results 校验正值范围，负值回退默认
- [ ] `truncate_output`：使用 `str::floor_char_boundary()`（Rust 1.73+）或手动安全截断
- [ ] find 工具：禁止绝对 pattern（以 `/` 开头返回错误）
- [ ] ls 工具：添加 `max_entries` 参数（默认 1000）
- [ ] session/storage.rs：zstd 解压前记录压缩大小，设置 max_output 比例限制
- [ ] RepeatDetector：ngram_set 添加容量上限或 LRU 淘汰
- [ ] 添加相关单元测试
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c93-fix-resource-boundary --strict --no-interactive`
