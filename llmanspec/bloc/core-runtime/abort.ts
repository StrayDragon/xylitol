/**
 * 中止状态机（Abort）。
 *
 * 业务：abort 取消当前 turn、取消 provider 流、重置 token 计量、清空 steer 并保留 follow_up FIFO；
 * abort 的 turn 不做 LLM 落定结算（abort-persist-skip-llm）。
 *
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1028（abort-queue-semantics）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1029（abort-resets-token）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1030（abort-cancels-provider-stream）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1043（abort-persist-skip-llm）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1775（abort 对进程级 reload 的合作取消）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/runtime/state.rs（TurnEvent::Cancel）、crates/xylitol-runtime/src/agent/capabilities/queue.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type AbortPhase = "running" | "abortRequested" | "cancelled" | "settled";

export interface AbortContext {
  /** provider 流是否仍在进行。 */
  streamActive: boolean;
  /** 已下发的取消令牌数。 */
  cancelTokens: number;
  /** steer 队列是否已被清空。 */
  steerCleared: boolean;
  /** follow_up 队列保留条数。 */
  followUpRetained: number;
  /** 是否做了 LLM 落定结算（abort 路径须为 false）。 */
  llmSettlementDone: boolean;
  /** token 计量是否已重置。 */
  tokenReset: boolean;
  /** 中止后是否进入下一个 turn。 */
  nextTurnStarted: boolean;
}

export const initialContext: AbortContext = {
  streamActive: true,
  cancelTokens: 0,
  steerCleared: false,
  followUpRetained: 2,
  llmSettlementDone: false,
  tokenReset: false,
  nextTurnStarted: false,
};

export const meta = defineMeta({
  name: "abort",
  pkg: "core-runtime",
  purpose: "中止语义：取消流、清 steer、留 follow_up、跳过 LLM 结算",
  anchors: [
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1028",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1029",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1030",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1043",
    "llmanspec/specs/server-core/server-core.feature#@req:r1775",
  ],
  impl: ["crates/xylitol-runtime/src/agent/runtime/state.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as AbortContext,
    events: {} as
      | { type: "abort" }
      | { type: "streamStopped" }
      | { type: "turnFinished" }
      | { type: "nextTurn" }
  },
  actions: {
    requestCancel: assign({
      cancelTokens: ({ context }) => context.cancelTokens + 1,
      steerCleared: () => true,
    }),
    stopStream: assign({ streamActive: () => false }),
    settle: assign({
      tokenReset: () => true,
      llmSettlementDone: () => false,
    }),
    startNext: assign({ nextTurnStarted: () => true }),
  },
});

export const machine = setupMachine.createMachine({
  id: "abort",
  initial: "running",
  context: initialContext,
  states: {
    running: {
      on: {
        abort: { target: "abortRequested", actions: "requestCancel" },
      },
    },
    abortRequested: {
      on: {
        streamStopped: { target: "cancelled", actions: "stopStream" },
      },
    },
    cancelled: {
      on: {
        turnFinished: { target: "settled", actions: "settle" },
      },
    },
    settled: {
      on: {
        nextTurn: { target: "running", actions: "startNext" },
      },
    },
  },
});

export const scenarioGroups = {
  "中止取消流并跳过 LLM 结算": [
    { name: "中止", events: [{ type: "abort" }], expectStates: ["abortRequested"], expectContext: { cancelTokens: 1, steerCleared: true } },
    { name: "provider 流停止", events: [{ type: "streamStopped" }], expectStates: ["cancelled"], expectContext: { streamActive: false } },
    { name: "收尾不做 LLM 结算", events: [{ type: "turnFinished" }], expectStates: ["settled"], expectContext: { llmSettlementDone: false, tokenReset: true, followUpRetained: 2 } },
  ],
  "保留 follow_up 并续下一条": [
    { name: "中止并收尾", events: [{ type: "abort" }, { type: "streamStopped" }, { type: "turnFinished" }], expectStates: ["settled"] },
    { name: "弹出下一条 follow_up 成新 turn", events: [{ type: "nextTurn" }], expectStates: ["running"], expectContext: { nextTurnStarted: true, followUpRetained: 2 } },
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