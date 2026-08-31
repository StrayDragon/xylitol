# Tasks — c2475 Serve 注册文件发现契约

- [ ] 1. 注册文件读写模块：原子写（tmp+rename，0600）、读取、删除；
  `start()` ready 后写 `~/.xylitol/serve.json`，停机路径删除。
- [ ] 2. 自我驱逐循环：5s 复读校验字段全等，失配即优雅停机。
- [ ] 3. healthz 全 phase body 携带 `pid` + `version`；`oapi.rs` 描述随动。
- [ ] 4. attach 诊断链：`probe_host` 失败路径按 D3 读注册文件分级报错；
  单测覆盖分级（无文件 / 僵死 / 异版本 / 非本服务 / 已退出）。
- [ ] 5. server-core.feature 落 `@req:sr-reg1` + 3 条 `@executable` 并实现 bindings 转绿。
- [ ] 6. 门禁：`just fmt` / `just lint` / `just test`；`llman sdd validate --strict`。
