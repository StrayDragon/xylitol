# Design：Host 回环凭据门禁

## 目标形态

```
~/.xylitol/serve.token   0600  随机凭据（跨重启稳定）
~/.xylitol/serve.json    0600  发现契约：url / pid / version / token?
        │
        ├── 产品 TUI attach：读同 URL 注册文件 → Basic auth
        └── xylitol serve：启动时读/建 token → 鉴权 hoop + 写注册文件
```

## 决策

### 1. 默认策略

产品入口 `xylitol serve` **默认开启**门禁。理由：

- 同机进程攻击面真实存在，个人开箱不等于可信边界。
- 客户端无需交互：token 经 0600 注册文件自动发现。
- 不提供 `--no-auth` 旗标，避免把「关掉鉴权」变成常规操作。

`xylitol_serve::serve` / `bind_listen` 是库入口，只有显式传入凭据才开启鉴权；
BDD/嵌入式测试不受产品默认影响。

### 2. 凭据存储

- 路径：`~/.xylitol/serve.token`（与 `serve.json` 同目录，文件分离）。
- 首次：32 字节随机（两个 UUID v4 拼接后 base64url 无填充）写入，0600 原子 rename。
- 后续：读取并 trim；非空即复用；Unix 上修正权限为 0600。
- 不写入环境变量，不打印到日志。

### 3. 鉴权形态

- `Authorization: Basic base64("xylitol:<token>")`。
- 校验用户名固定 `xylitol`，token 用常量时间比较。
- 未通过：401 + `WWW-Authenticate: Basic realm="xylitol"` + JSON
  `{"error":"unauthorized"}`。
- `POST /rpc`、`GET /healthz`、`WS /rpc` 升级统一走同一 hoop。
- WS 升级兜底：`GET /rpc?auth_token=<token>`；查询参数只在 WS 升级路径接受，
  避免 token 进入普通 HTTP 日志。

### 4. 注册契约

`Registration` 增加可选 `token` 字段（`serde(default, skip_serializing_if)`）。
无鉴权的库入口保持旧形状；产品 `serve` 写入 token。注册文件已 0600，token 不额外
落盘到别处。

客户端只在 `reg.url` 与目标 attach URL 匹配时使用 token，避免把凭据发给别的端口。

### 5. 客户端

- `HttpWsClient` 增加可选 token；WS 握手插入 `Authorization` header。
- CLI 在创建 remote driver 前读取注册文件 token；healthz 探活同样携带。
- 401 时给可操作诊断：Host 需要鉴权，但注册文件没有匹配凭据；重启 `xylitol serve`
  让注册文件重写，或从同一账号运行客户端。

### 6. 工具子进程剥离

保留变量：`XYLITOL_SERVE_TOKEN`、`XYLITOL_AUTH_TOKEN`。在以下 spawn 前移除：

- bash 工具（`tokio::process::Command`）
- 检索/进程工具（`tokio::process::Command`）
- 交互 bash（`shell_env_with_agent_bin` 的 env map）
- 外部插件 stdio 子进程
- hooks 脚本子进程
- MCP stdio 子进程

token 本身不进入进程环境；剥离是防御性硬规则，防止未来引入 env 传递时泄漏。

## 测试边界

- `protocol::auth`：Basic 编解码往返、错误输入。
- `xylitol-serve`：凭据文件创建/复用/权限；注册 token；鉴权 hoop 401/200；
  WS 查询参数 token 解析。
- `xylitol-kernel`：attach token 读取与 401 诊断。
- `xylitol-app-tui`：WS header 构造（单测）。
- `xylitol-official-plugin-tools`：env map 剥离。
- BDD：server-core 的 401/200/注册 token 场景；domain-security 的 env 剥离场景。

## 非目标

- 不做多用户/远程公网安全模型。
- 不做账号体系、OAuth、mTLS。
- 不改线协议方法表或 JSON-RPC 信封。
