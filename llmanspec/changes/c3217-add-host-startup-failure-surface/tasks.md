# Tasks — Surface host startup failure fast

- [x] 1. SpawnedHost：保留 child、stderr 0600 临时文件、Drop detach/清理。
- [x] 2. wait_until_ready：轮询子进程退出，提前返回含状态与 stderr 摘要的诊断。
- [x] 3. CLI：Host 未在听时输出 starting local Host 提示。
- [x] 4. 测试：子进程提前退出单测；现有 ensure-host 测试更新。
- [x] 5. Specs 与门禁：user-experience 条款；just fmt / just lint / just qa / validate --strict。
