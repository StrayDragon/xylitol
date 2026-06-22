---
id: c220-add-sandbox
title: "添加沙箱执行隔离 — 工具/进程运行在受限环境中"
depends_on: []
---

## Why

Pi 的文档中记载了三种沙箱模式（Gondolin QEMU VM、Plain Docker、OpenShell policy sandbox），以及一个 Anthropic sandbox-runtime 扩展（macOS sandbox-exec / Linux bubblewrap）。

Xylitol 作为**配置驱动的通用 agent**，不做可插拔扩展系统，因此沙箱必须是**内置于 agent 的可选功能**，通过 `config.yaml` 配置启用。核心目标：

1. **文件系统隔离** — 限制 bash/read/write/edit 等工具对主机文件系统的访问范围
2. **网络隔离** — 限制 bash 中的 curl/wget 等网络请求范围
3. **进程隔离** — 工具子进程在受限环境中运行（Linux Landlock / macOS sandbox）
4. **配置驱动** — 所有策略在 `config.yaml` 的 `sandbox` 块中声明，无需扩展/插件

## What Changes

1. **`SandboxConfig` 配置结构体** — 在 `infra/config/types.rs` 中添加 `SandboxConfig`：
   - `enabled: bool`
   - `filesystem.read_allowed: Vec<String>` / `read_denied: Vec<String>`
   - `filesystem.write_allowed: Vec<String>` / `write_denied: Vec<String>`
   - `network.allowed_domains: Vec<String>` / `denied_domains: Vec<String>`
   - `process.allowed_paths: Vec<String>`
   - `backend: "none" | "landlock" | "macos-sandbox"`（平台自适应）

2. **`SandboxEngine` 实现** — 在 `src/infra/sandbox/` 新建模块：
   - `mod.rs` — SandboxEngine trait + 工厂函数
   - `policy.rs` — sandbox 策略评估（路径匹配、域名匹配）
   - `platform/linux.rs` — Landlock LSM 绑定（Linux 5.13+）
   - `platform/macos.rs` — macOS sandbox 绑定（sandbox-exec / Seatbelt）
   - `platform/mod.rs` — 平台无关回退（仅应用级路径检查）

3. **工具执行包装** — 在 bash/read/write/edit 等工具执行前，通过 `SandboxEngine` 检查：
   - 文件系统操作（read/write/edit）：检查路径是否在允许范围内
   - bash 执行：检查网络请求域名、进程路径
   - 不通过则返回错误消息而非执行

4. **Feature flag** — `infra-sandbox`（已在 Cargo.toml 中预留）

## Capabilities

- security-policy

## Impact

- `src/infra/sandbox/` — 新建模块（600-800 行）
- `src/agent/tools/bash.rs` — 沙箱包装
- `src/agent/tools/read.rs` / `write.rs` / `edit.rs` — 路径检查
- `src/infra/config/types.rs` — 新增 SandboxConfig
- `src/agent/loop.rs` — 初始化沙箱引擎
- `Cargo.toml` — 可选依赖 `rustix`（Landlock）或 `sandbox` 系统 crate

## Definition of Done

- `SandboxConfig` 可从 `config.yaml` 加载并合并三层配置
- `SandboxEngine` 可对路径访问做正确判断（allow > deny，最具体规则优先）
- bash 执行网络检查至少能拦截 `curl evil.com` 当 `denied_domains` 包含 `evil.com`
- 测试覆盖：允许/拒绝路径匹配、网络域名匹配、Landlock 基础集成
- `cargo build` 通过且 `cargo test` 通过
