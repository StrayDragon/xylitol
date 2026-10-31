# Tasks

## 1. 规则
- [x] runtime-model-registry 新增重载规则

## 2. 运行时
- [x] `ModelManager::replace_registry`
- [x] `AgentCapabilities::replace_model_registry`
- [x] `AgentRuntime::replace_model_registry`

## 3. 内核
- [x] `build_model_registry` 可见性
- [x] reload `models` 步骤

## 4. TUI
- [x] 重载后刷新模型目录

## 5. 测试
- [x] 运行时 replace_registry 单测（保留/回落/空）
- [x] 内核 reload 步骤测试
- [x] `just qa` 绿
