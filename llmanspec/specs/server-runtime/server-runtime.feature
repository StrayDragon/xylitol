# language: en
# migrated from tests/features/server.feature
Feature: server-runtime
  Background:
    Given 锁路径已清理

  Scenario: start-healthz
    When 服务端在空闲端口上启动
    Then healthz 端点返回 200 OK
    And 锁文件包含 port, pid, hostname

  Scenario: second-instance-rejected
    Given 服务端已在运行（锁文件存在）
    When 第二个服务端启动（相同锁路径）
    Then 第二个实例收到 ServerLockedError
