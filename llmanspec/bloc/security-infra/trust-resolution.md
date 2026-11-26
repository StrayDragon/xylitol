# trust-resolution

> 项目信任按固定优先级解析：覆盖、无输入、store、UI 提示、回退拒绝

子包：`security-infra`　代码：`security-infra/trust-resolution.ts`

## 状态图

```mermaid
stateDiagram-v2
    [*] --> idle
    idle --> overrideGate: resolve
    overrideGate --> settled: ε（无条件） [hasOverride]
    overrideGate --> inputsGate: ε（无条件）
    inputsGate --> settled: ε（无条件） [noTrustInputs]
    inputsGate --> storeGate: ε（无条件）
    storeGate --> uiGate: ε（无条件） [storeTampered]
    storeGate --> settled: ε（无条件） [storeUsable]
    storeGate --> uiGate: ε（无条件）
    uiGate --> settled: ε（无条件） [uiUnavailable]
    uiGate --> prompting: ε（无条件）
    prompting --> settled: promptAnswered [promptKeyValid]
    prompting --> settled: promptAnswered
    settled --> overrideGate: recheck
    settled --> [*]
```

## 状态清单

- `idle`
- `overrideGate`
- `inputsGate`
- `storeGate`
- `uiGate`
- `prompting`
- `settled`

## spec 锚点

- `llmanspec/specs/domain-security/domain-security.feature#@req:r73`
- `llmanspec/specs/domain-security/domain-security.feature#@req:r72`
- `llmanspec/specs/domain-security/domain-security.feature#@req:r71`
- `llmanspec/specs/domain-security/domain-security.feature#@req:r74`
- `llmanspec/specs/domain-security/domain-security.feature#@req:r75`
- `llmanspec/specs/domain-security/domain-security.feature#@req:r3216`
- `llmanspec/specs/domain-security/domain-security.feature#@req:r3218`
- `llmanspec/specs/domain-security/domain-security.feature#@req:r3219`
- `llmanspec/specs/domain-security/domain-security.feature#@req:r3220`
- `llmanspec/specs/app-tui-trust/app-tui-trust.feature#@req:r3307`
- `llmanspec/specs/app-tui-trust/app-tui-trust.feature#@req:r3223`

## 实现落点

- `crates/xylitol-runtime/src/trust/resolve.rs`
