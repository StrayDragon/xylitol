# Tasks

## 1. 规则
- [ ] runtime-model-registry 新增重载规则

## 2. 运行时
- [ ] `ModelManager::replace_registry`
- [ ] `AgentCapabilities::replace_model_registry`
- [ ] `AgentRuntime::replace_model_registry`

## 3. 内核
- [ ] `build_model_registry` 可见性
- [ ] reload `models` 步骤

## 4. TUI
- [ ] 重载后刷新模型目录

## 5. 测试
- [ ] 运行时 replace_registry 单测（保留/回落/空）
- [ ] 内核 reload 步骤测试
- [ ] `just qa` 绿
