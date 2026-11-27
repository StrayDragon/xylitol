/**
 * 自动重试状态机（Auto Retry）。
 *
 * 业务：provider 调用失败后按退避策略重试；重试期间经 XyEvent 暴露状态，
 * 耗尽后以 Error 收尾；中止时不再重试。
 *
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3304（auto-retry-event-semantics）
 * @spec llmanspec/specs/app-tui-bridge/app-tui-bridge.feature#@req:r1184（auto-retry-status-scrollback）
 * @spec llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1239（compaction-retry-single-status-line）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1031（unknown-event-degrade）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/runtime/retry.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type RetryPhase = "requesting" | "waitingBackoff" | "retrying" | "succeeded" | "exhausted" | "aborted";

export interface RetryContext {
  /** 已尝试次数。 */
  attempts: number;
  /** 允许的最大尝试次数。 */
  maxAttempts: number;
  /** 当前退避毫秒。 */
  backoffMs: number;
  /** 退避上限。 */
  maxBackoffMs: number;
  /** 最近一次失败类别。 */
  lastFailureKind: string | null;
  /** 是否发出过重试状态事件。 */
  statusEventEmitted: boolean;
}

export const initialContext: RetryContext = {
  attempts: 0,
  maxAttempts: 3,
  backoffMs: 500,
  maxBackoffMs: 4000,
  lastFailureKind: null,
  statusEventEmitted: false,
};

export const meta = defineMeta({
  name: "auto-retry",
  pkg: "core-runtime",
  purpose: "provider 失败重试：退避、状态事件、耗尽与中止",
  anchors: [
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r3304",
    "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature#@req:r1184",
    "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1239",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1031",
  ],
  impl: ["crates/xylitol-runtime/src/agent/runtime/retry.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as RetryContext,
    events: {} as
      | { type: "request" }
      | { type: "failed"; kind: "retryable" | "fatal" }
      | { type: "backoffElapsed" }
      | { type: "succeeded" }
      | { type: "abort" }
  },
  guards: {
    retryable: ({ event }) => (event as { kind: string }).kind === "retryable",
    fatal: ({ event }) => (event as { kind: string }).kind === "fatal",
    attemptsRemain: ({ context }) => context.attempts < context.maxAttempts,
    attemptsExhausted: ({ context }) => context.attempts >= context.maxAttempts,
  },
  actions: {
    countAttempt: assign({ attempts: ({ context }) => context.attempts + 1 }),
    scheduleBackoff: assign({
      backoffMs: ({ context }) => Math.min(context.backoffMs * 2, context.maxBackoffMs),
      statusEventEmitted: () => true,
    }),
    recordFailure: assign({ lastFailureKind: ({ event }) => (event as { kind: string }).kind }),
  },
});

export const machine = setupMachine.createMachine({
  id: "auto-retry",
  initial: "requesting",
  context: initialContext,
  states: {
    requesting: {
      on: {
        request: { actions: "countAttempt" },
        failed: [
          { guard: "fatal", target: "exhausted", actions: "recordFailure" },
          { guard: "retryable", target: "waitingBackoff", actions: ["recordFailure", "scheduleBackoff"] },
        ],
        succeeded: { target: "succeeded" },
        abort: { target: "aborted" },
      },
    },
    waitingBackoff: {
      on: {
        backoffElapsed: [
          { guard: "attemptsRemain", target: "retrying" },
          { guard: "attemptsExhausted", target: "exhausted" },
        ],
        abort: { target: "aborted" },
      },
    },
    retrying: {
      on: {
        request: { target: "requesting", actions: "countAttempt" },
        abort: { target: "aborted" },
      },
    },
    succeeded: { type: "final" },
    exhausted: { type: "final" },
    aborted: { type: "final" },
  },
});

export const scenarioGroups = {
  "可重试失败后成功": [
    { name: "首次请求", events: [{ type: "request" }], expectStates: ["requesting"], expectContext: { attempts: 1 } },
    { name: "可重试失败进入退避", events: [{ type: "failed", kind: "retryable" }], expectStates: ["waitingBackoff"], expectContext: { backoffMs: 1000, statusEventEmitted: true } },
    { name: "退避到期重试", events: [{ type: "backoffElapsed" }], expectStates: ["retrying"] },
    { name: "重试成功", events: [{ type: "request" }, { type: "succeeded" }], expectStates: ["succeeded"], expectContext: { attempts: 2 } },
  ],
  "致命失败不重试": [
    { name: "首次请求", events: [{ type: "request" }], expectStates: ["requesting"] },
    { name: "致命失败直接耗尽", events: [{ type: "failed", kind: "fatal" }], expectStates: ["exhausted"], expectContext: { lastFailureKind: "fatal" } },
  ],
  "重试耗尽": [
    { name: "三次请求", events: [{ type: "request" }, { type: "failed", kind: "retryable" }, { type: "backoffElapsed" }, { type: "request" }, { type: "failed", kind: "retryable" }, { type: "backoffElapsed" }, { type: "request" }], expectStates: ["requesting"], expectContext: { attempts: 3 } },
    { name: "最后一次失败进入耗尽", events: [{ type: "failed", kind: "retryable" }], expectStates: ["waitingBackoff"] },
    { name: "退避到期无剩余次数", events: [{ type: "backoffElapsed" }], expectStates: ["exhausted"] },
  ],
  "中止不再重试": [
    { name: "首次请求", events: [{ type: "request" }], expectStates: ["requesting"] },
    { name: "中止", events: [{ type: "abort" }], expectStates: ["aborted"] },
  ],
} satisfies Readonly<Record<string, readonly ScenarioStep[]>>;

if (import.meta.main) {
  const results = runScenarioGroups(machine, scenarioGroups);
  const failed = Object.entries(results).filter(([, r]) => !r.passed);
  if (failed.length > 0) {
    for (const [group, r] of failed) {
      console.error(`组 ${group}`);
      for (const line of r.failures) console.error(`  ${line}`);
    }
    process.exit(1);
  }
  console.log(`${meta.name}: ${Object.keys(results).length} 组场景通过`);
}
