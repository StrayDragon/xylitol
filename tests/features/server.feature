Feature: Server process lifecycle

  The xylitol server starts, acquires a single-instance lock, exposes
  REST and WebSocket endpoints, and supports reconnection with event replay.

  Background:
    Given a clean lock path

  Scenario: Server starts and serves health check
    When the server starts on a free port
    Then the healthz endpoint returns 200 OK
    And the lock file exists with port, pid, and hostname

  Scenario: Second instance is rejected
    Given a running server with a lock file
    When a second server starts against the same lock path
    Then the second server receives a ServerLockedError
