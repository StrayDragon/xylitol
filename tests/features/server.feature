Feature: 服务端进程生命周期

  xylitol 服务端启动时获取单实例锁，暴露 REST 和 WebSocket 端点。

  Background:
    Given 锁路径已清理

  Scenario: 服务端启动并通过健康检查
    When 服务端在空闲端口上启动
    Then healthz 端点返回 200 OK
    And 锁文件包含 port, pid, hostname

  Scenario: 第二实例被拒绝
    Given 服务端已在运行（锁文件存在）
    When 第二个服务端启动（相同锁路径）
    Then 第二个实例收到 ServerLockedError
