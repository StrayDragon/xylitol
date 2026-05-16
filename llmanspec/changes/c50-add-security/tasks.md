# c50-add-security Tasks

- [ ] 定义 SecurityPolicy 结构体（bash/filesystem/network 三级限制）
- [ ] 实现 bash 命令限制（正则 allowed/forbidden patterns）
- [ ] 实现文件系统访问控制（glob path allow/blocklist）
- [ ] 实现网络访问控制（域名/IP 匹配）
- [ ] 实现资源配额（子进程数/内存/CPU 时间）
- [ ] 实现规则合并（仅收紧不放宽）
- [ ] 集成到工具执行流程（执行前检查 + tool_call_blocked 事件）
- [ ] 编写测试：每种限制场景、规则合并逻辑
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c50-add-security --strict --no-interactive`
