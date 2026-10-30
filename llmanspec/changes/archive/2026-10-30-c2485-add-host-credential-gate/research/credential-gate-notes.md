# 回环凭据门禁 外部对照笔记（c2485）

> 调研来源：外部参考实现（生产级 coding agent），2026-08-23 摘录。
> 性质：**决策草案**支撑材料——先记问题与候补方向，拍板后再 propose。
> 本笔记仅作选型对照；关键结论已摘要进 proposal。

## 参考实现一手证据

**凭据生成与稳定性**（其 CLI 服务配置模块 `service-config.ts:134-143`）：

```ts
const next = value ?? randomBytes(32).toString("base64url")
// Keep one private credential across server restarts so discovered clients
// can reconnect without exposing a password flag or environment variable.
```

- 首次生成 32B 随机 base64url，跨重启复用（存 config 目录，与 state 目录的
  发现注册文件分开）；客户端从注册文件 password 字段获得，无需旗标/env。

**线上形态**（server 鉴权中间件 `authorization.ts`）：

- `Authorization: Basic base64("<user>:<password>")`，username 固定；
- 浏览器/WS 无法带 header 的场景：一次性 connect ticket URL +
  `?auth_token=` 查询参数兜底（白名单端点）；
- 未配置密码的 embedded 模式：整个中间件退化为恒等函数（46 行）——
  **门禁是可选层而非强制层**；
- 401 回 `WWW-Authenticate`；健康检查也在鉴权后，防匿名探测。

**凭据脱敏硬规则**（`server-process.ts:59-63`）：

```ts
delete process.env.<CREDENTIAL_ENV>
// Keep the lease credential out of the environment inherited by tools.
```

stdio 模式把租约凭据从工具子进程继承环境里删掉——coding agent 场景下
模型会执行任意命令，凭据进 env 等于送给被操作对象。

## 设计动机

- 本地回环 ≠ 可信边界：同机其他用户/进程（恶意 postinstall、CI 脚本）都能探测端口；
- 凭据「跨重启稳定 + 经发现契约携带」让门禁不增加交互成本；
- embedded 无密码退化为恒等——个人开箱与安全门禁可以共存。

## xylitol 现状核对（2026-08-23）

- 监听器无任何鉴权（rg 验证 server/ 下无 auth/Authorization/password 逻辑）；
- 仅 writer lease token（host.rs:108）闸非只读 unary 的并发写身份——是租约不是访问控制；
- 工具子进程继承完整进程 env（无剥离逻辑）；当前 env 里虽无常驻凭据，
  但未来 c2475 注册文件引入 password 后此风险立即成立。

## 拍板点

1. 默认生成凭据上闸 vs 配置开启 vs 保持放行（个人开箱优先级）；
2. WS 升级的兜底形态（一次性 ticket vs 查询参数）；
3. 「凭据不进工具子进程 env」建议作为独立硬规则先行，无论门禁本体是否落地。
