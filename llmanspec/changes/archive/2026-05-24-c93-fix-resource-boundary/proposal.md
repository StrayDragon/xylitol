---
depends_on: [c50-add-security, c20-add-tools]
---

# c93-fix-resource-boundary

## Why

漏洞审查发现多处资源边界缺失，使系统面临 DoS/OOM 风险：

1. **read 工具无文件大小上限**：`read_to_string` 一次性载入整个文件，GB 级文件导致 OOM
2. **bash 输出全量缓冲**：`Command::output()` 在 truncate 前已将全部 stdout/stderr 载入内存
3. **bash timeout 整数溢出**：LLM 传入负 timeout → `as u64` → `u64::MAX`，等价无限等待
4. **grep/find max_results 负值**：`as usize` 转换后变为极大值，取消结果上限
5. **zstd 解压炸弹**：`decode_all` 无最大输出字节限制，小压缩包可解压为 GB 级 JSON
6. **ls 无目录项数量上限**：百万文件目录导致内存/响应膨胀
7. **find 绝对路径逃逸**：pattern 以 `/` 开头时忽略 root 约束
8. **RepeatDetector ngram_set 无界增长**：长流式输出导致内存持续膨胀
9. **UTF-8 截断 panic**：`truncate_output` 在多字节字符中间切割导致 panic

## What Changes

1. read/grep 工具增加 `MAX_FILE_SIZE`（10MB），超限返回错误或强制 offset/limit
2. bash 改用 `tokio::process` 流式读取 + 字节上限，超限 kill 子进程
3. bash timeout 校验范围（`1..=config.timeout_secs`），拒绝非法值
4. grep/find max_results 校验正值范围（`1..=MAX_RESULTS`）
5. zstd 解压增加 `max_output_size` 校验
6. ls 增加 `max_entries` 参数与默认值
7. find 禁止绝对 pattern 或 canonicalize 验证
8. RepeatDetector 为 ngram_set 设容量上限（LRU 淘汰）
9. `truncate_output` 使用 `floor_char_boundary()` 安全截断

## Capabilities

- `tool-system`: 工具系统资源边界

## Impact

- 工具行为变更：超大文件/输出将被截断或拒绝（需文档说明）
- 向后兼容：对正常使用（<10MB 文件、合理 timeout）无影响
- 性能：bash 流式读取可能略微增加代码复杂度
