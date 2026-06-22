# Sandbox 设计权衡

## 方案对比

| 方案 | OS 级隔离 | 跨平台 | 配置复杂度 | 实现成本 |
|---------|-----------|-----------|-----------------|-------------|
| **纯应用层路径/域名检查** | ❌ | ✅ (纯 Rust) | 低（glob 匹配） | ~300 行 |
| **Linux Landlock** | ✅ (内核 LSM) | ❌ (仅 Linux) | 中（rustix API） | ~200 行 |
| **macOS sandbox-exec** | ✅ (系统调用) | ❌ (仅 macOS) | 高（C 绑定） | ~150 行 stub |
| **Docker 容器运行时** | ✅ (容器) | ✅ | 高（需 Docker） | ~100 行 wrapper |

## 推荐策略

**分层实现，从轻到重：**

1. **第一阶段（本 change）**：
   - `FallbackBackend` — 应用级路径/域名 glob 匹配，纯 Rust
   - 覆盖 90% 的常见沙箱需求（别写 .env、别 curl 恶意域名）
   - 零外部依赖，跨平台

2. **第二阶段（后续 change）**：
   - Linux Landlock 后端（`rustix` 的 Landlock API）
   - 真正的 OS 级强制隔离，不可绕过

3. **第三阶段（后续 change）**：
   - macOS sandbox-init 后端
   - 容器化执行后端（可选的 Docker 集成）

## 不做的事

- ❌ 不做 QEMU 微 VM（Gondolin 方案）— 太重，不适合配置驱动
- ❌ 不做完整的 seccomp-bpf — 与 Landlock 重复，且跨架构维护成本高
- ❌ 不做 OpenShell 集成 — 第三方策略引擎，不符合"开箱即用"定位
