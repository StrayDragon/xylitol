# Design

## 现状形状

`MethodEntry` 六列，其中两列零消费：

```text
MethodEntry {
    name:            &'static str   有消费者（lookup 比较、is_unary_method）
    auth:            Auth           有消费者（is_writer_method, host.rs:1114）
    idem:            Idem           零消费者
    resp:            Resp           零消费者
    command_backed:  bool           有消费者（registry 内部守卫测试 :570/:705/:1019-1031、tests/bdd/steps_server.rs:4358；`parse_command` 本身不读该列）
    exec:            Exec           有消费者（r1197 钉死；effects/mod.rs:32）
}
```

`REGISTRY` 50 行，构造器 `m(name, auth, idem, resp, command_backed)` 与 `mx(name, auth, idem, resp, command_backed, exec)` 各带 `idem` / `resp` 两个无语义实参。

## 目标形状

```text
MethodEntry {
    name:            &'static str
    auth:            Auth
    command_backed:  bool
    exec:            Exec
}
```

构造器收敛为 `m(name, auth, command_backed)` 与 `mx(name, auth, command_backed, exec)`。

## 决策：为什么保留 exec_class 双真相源

`exec_class(cmd)` 与 `REGISTRY.exec` 看似重复，实为 r1197 明确要求的锁定结构：「声明须与 wire REGISTRY 属性行同源并由守卫测试锁定逐变体穷举一致」。

- `REGISTRY.exec` 是声明（wire 侧唯一真值）。
- `exec_class(cmd)` 是按 `Command` 变体穷举的推导（客户端消费面）。
- `exec_class_matches_registry_declaration`（`registry.rs:813-1050`，47 个 `command_backed` 变体全覆盖）是锁定测试。

若合并两者（例如让 `exec_class` 只查 `REGISTRY`），就失去了「逐变体穷举」的编译期保障——新增 `Command` 变体时 `match` 会强制补全，而查表不会。**本 change 不动这个结构。** 这是与「删零消费者」相反方向的判断：`exec_class` 做删除想象后会在 47 个变体处重新冒出来，且被 spec 钉死。

## 决策：为什么删 names()

`names()` 是 `REGISTRY.iter().map(|e| e.name)` 的裸转发。零调用点。删掉后无复杂度回流。`MethodEntry.name` 字段本身保留（`lookup` 与 `is_unary_method` 需要）。

## 迁移顺序

1. 删 `Resp` 枚举 + 字段 + 构造器实参 + `lib.rs` 导出。
2. 删 `Idem` 枚举 + 字段 + 构造器实参 + `lib.rs` 导出。
3. 删 `names()` + 导出。
4. 收敛 `m()` / `mx()` 签名，`REGISTRY` 每行同步。
5. 删 `writer_set_and_bypass_set_match_declaration` 的 `Bypass` 断言段。

第 1–4 步同批完成（同一文件），第 5 步随后。

## 风险

- `tests/snapshots/agent_session_api_baseline__agent_session_public_api_surface_baseline.snap` 可能捕获 `xylitol-protocol` 公开面。删 `Resp`/`Idem` 后需重跑 snapshot 并核对差异仅含被删项。
- `tests/bdd/steps_server.rs:4342-4360` 只读 `Auth` / `Exec` / `command_backed`，不受影响。

## 不做什么

- 不改 `Auth` 列。
- 不改 `Exec` 列与 `exec_class`。
- 不改 `parse_command`。
- 不碰 `plugin-sdk` 的 `Registry<T>`（那是通用容器，有真实多消费者）。
- 不碰 `runtime` 的 `ModelRegistry`。
