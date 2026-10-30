# Tasks

- [x] 1.1 `xylitol-protocol` 增加 Basic auth 常量与编解码、保留凭据环境变量常量，含单测
- [x] 1.2 `xylitol-serve` 增加凭据文件：`~/.xylitol/serve.token` 0600 原子写、首次生成、跨重启复用，含单测
- [x] 2.1 `Registration` 增加可选 `token`；`arm_registration` 写入；注册文件测试覆盖 token 往返与缺省省略
- [x] 2.2 `bind_listen` 鉴权 hoop：`POST /rpc`、`GET /healthz`、`WS /rpc` 未授权返回 401 + `WWW-Authenticate`；WS `auth_token` 查询兜底；含单测
- [x] 3.1 产品 `start` 默认加载/生成凭据并传入监听；`HttpWsClient` 支持 token 并写入 WS header；CLI attach 读取注册 token，含单测
- [x] 3.2 `attach_preflight` 携带 token 探活；401 返回可操作诊断，含单测
- [x] 4.1 工具子进程 spawn 前剥离保留凭据环境变量（bash / 检索 / 交互 bash / 外部插件 / hooks / MCP），含单测
- [x] 5.1 specs：`server-core` 回环凭据门禁、`domain-security` 工具子进程凭据剥离；新增 BDD 场景与绑定
- [x] 5.2 `llman-sdd validate --strict --check`、`just qa`、`just qa-e2e` 全绿
