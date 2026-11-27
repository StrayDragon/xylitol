/**
 * ReAct 内环状态机（Thin ReAct Loop）。
 *
 * 业务：调用 XyModel → 流式消费 chunk → 若响应含 tool_calls 则执行工具批并续跑，
 * 直至无 tool_calls、abort、或 should_stop_after_step 请求停。
 *
 * 关键不变量：
 * - 工具意图须在流式阶段经 MessageUpdate 暴露，不得在 MessageEnd 前执行工具副作用（r1038）。
 * - 同一响应中的全部工具调用须在下一轮模型调用前执行完毕（r1047）。
 * - 续跑须关闭本 iteration（StepStart/StepEnd 成对），不做 ContextTokenSettlement / auto-compact 预检（r1047）。
 * - steer 须在模型调用前 drain（r1059）。
 *
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1027（thin-react-loop）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1038（tool-intent-before-execution）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1047（工具批后继续）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1041（should-stop-after-step）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1045（tool-batch-barrier-parallel）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1059（pending-queues：模型调用前 drain steer）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/runtime/react/（mod.rs / assistant.rs / step_end.rs）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type ReactPhase =
  | "awaitingSubmit"
  | "drainingSteer"
  | "callingModel"
  | "streaming"
  | "closingStep"
  | "executingTools"
  | "settling"
  | "finished";

export interface ReactContext {
  /** 已完成 step 数。 */
  stepIndex: number;
  /** 当前 step 内是否已发 StepStart。 */
  stepOpen: boolean;
  /** 流式阶段累积的工具意图数。 */
  toolIntents: number;
  /** 是否已发 MessageEnd（执行工具的前置条件）。 */
  messageEnded: boolean;
  /** 本批已执行的工具数。 */
  toolsExecuted: number;
  /** 是否在流式阶段误执行了工具（应为 false）。 */
  violatedIntentOrdering: boolean;
  /** should_stop_after_step 是否已请求停。 */
  stopRequested: boolean;
  /** 本 turn 已 drain 的 steer 条数。 */
  steerDrained: number;
  /** 是否已做 ContextTokenSettlement。 */
  settlementDone: boolean;
  /** 收尾原因。 */
  finishReason: string | null;
}

export const initialContext: ReactContext = {
  stepIndex: 0,
  stepOpen: false,
  toolIntents: 0,
  messageEnded: false,
  toolsExecuted: 0,
  violatedIntentOrdering: false,
  stopRequested: false,
  steerDrained: 0,
  settlementDone: false,
  finishReason: null,
};

export const meta = defineMeta({
  name: "react-loop",
  pkg: "core-runtime",
  purpose: "ReAct 内环：模型调用、流式意图、工具批续跑、停步收尾",
  anchors: [
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1027",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1038",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1047",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1041",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1045",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1059",
  ],
  impl: ["crates/xylitol-runtime/src/agent/runtime/react/mod.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as ReactContext,
    events: {} as
      | { type: "submit"; steerAvailable: boolean }
      | { type: "steerDrained" }
      | { type: "stepStart" }
      | { type: "chunk"; toolIntent: boolean }
      | { type: "messageEnd" }
      | { type: "stepEnd"; shouldStop: boolean }
      | { type: "executeTool" }
      | { type: "settle"; reason: "no-tools" | "stop-requested" | "aborted" | "error" }
  },
  guards: {
    steerPending: ({ event }) => (event as { steerAvailable: boolean }).steerAvailable === true,
    noSteerPending: ({ event }) => (event as { steerAvailable: boolean }).steerAvailable === false,
    hasToolIntents: ({ context }) => context.toolIntents > 0,
    noToolIntents: ({ context }) => context.toolIntents === 0,
    intentBeforeMessageEnd: ({ context }) => context.messageEnded === false,
    shouldStop: ({ event }) => (event as { shouldStop: boolean }).shouldStop === true,
    shouldContinue: ({ event }) => (event as { shouldStop: boolean }).shouldStop === false,
  },
  actions: {
    beginStep: assign({
      stepOpen: () => true,
      toolIntents: () => 0,
      messageEnded: () => false,
    }),
    countSteer: assign({ steerDrained: ({ context }) => context.steerDrained + 1 }),
    accumulateIntent: assign({ toolIntents: ({ context }) => context.toolIntents + 1 }),
    flagViolation: assign({ violatedIntentOrdering: () => true }),
    markMessageEnded: assign({ messageEnded: () => true }),
    closeStep: assign({
      stepOpen: () => false,
      stepIndex: ({ context }) => context.stepIndex + 1,
    }),
    recordStop: assign({ stopRequested: () => true }),
    countTool: assign({ toolsExecuted: ({ context }) => context.toolsExecuted + 1 }),
    markSettlement: assign({ settlementDone: () => true }),
    finish: assign({ finishReason: ({ event }) => (event as { reason: string }).reason }),
  },
});

export const machine = setupMachine.createMachine({
  id: "react-loop",
  initial: "awaitingSubmit",
  context: initialContext,
  states: {
    awaitingSubmit: {
      on: {
        submit: [
          { guard: "steerPending", target: "drainingSteer" },
          { guard: "noSteerPending", target: "callingModel", actions: "beginStep" },
        ],
      },
    },
    drainingSteer: {
      on: {
        steerDrained: { target: "callingModel", actions: ["countSteer", "beginStep"] },
      },
    },
    callingModel: {
      on: {
        chunk: { target: "streaming", actions: "accumulateIntent" },
        messageEnd: { target: "closingStep", actions: "markMessageEnded" },
        settle: { target: "settling", actions: "finish" },
      },
    },
    streaming: {
      on: {
        chunk: [
          { guard: "intentBeforeMessageEnd", actions: "accumulateIntent" },
          { actions: "flagViolation" },
        ],
        messageEnd: { target: "closingStep", actions: "markMessageEnded" },
        executeTool: { actions: "flagViolation" },
        settle: { target: "settling", actions: "finish" },
      },
    },
    closingStep: {
      on: {
        stepEnd: [
          { guard: "shouldStop", target: "settling", actions: ["closeStep", "recordStop", "finish"] },
          { guard: "hasToolIntents", target: "executingTools", actions: "closeStep" },
          { guard: "noToolIntents", target: "settling", actions: "closeStep" },
        ],
      },
    },
    executingTools: {
      on: {
        executeTool: { actions: "countTool" },
        submit: { target: "callingModel", actions: "beginStep" },
        settle: { target: "settling", actions: "finish" },
      },
    },
    settling: {
      on: {
        settle: { actions: ["markSettlement", "finish"] },
      },
    },
    finished: {
      type: "final",
    },
  },
});

export const scenarioGroups = {
  "先工具后结束": [
    { name: "提交", events: [{ type: "submit", steerAvailable: false }], expectStates: ["callingModel"] },
    { name: "流式暴露工具意图", events: [{ type: "chunk", toolIntent: true }], expectStates: ["streaming"], expectContext: { toolIntents: 1, messageEnded: false } },
    { name: "流结束", events: [{ type: "messageEnd" }], expectStates: ["closingStep"], expectContext: { messageEnded: true } },
    { name: "关闭本 step 并执行工具", events: [{ type: "stepEnd", shouldStop: false }], expectStates: ["executingTools"], expectContext: { stepIndex: 1, stepOpen: false } },
    { name: "执行工具", events: [{ type: "executeTool" }], expectStates: ["executingTools"], expectContext: { toolsExecuted: 1 } },
    { name: "续跑下一 step", events: [{ type: "submit", steerAvailable: false }], expectStates: ["callingModel"] },
    { name: "无工具直接结束", events: [{ type: "messageEnd" }, { type: "stepEnd", shouldStop: false }], expectStates: ["settling"] },
  ],
  "工具意图早于MessageEnd": [
    { name: "提交并流式", events: [{ type: "submit", steerAvailable: false }, { type: "chunk", toolIntent: true }], expectStates: ["streaming"] },
    { name: "流未结束时执行工具被标记违规", events: [{ type: "executeTool" }], expectStates: ["streaming"], expectContext: { violatedIntentOrdering: true } },
  ],
  "改向在模型调用前注入": [
    { name: "提交且有 steer 待 drain", events: [{ type: "submit", steerAvailable: true }], expectStates: ["drainingSteer"] },
    { name: "drain 后进入模型调用", events: [{ type: "steerDrained" }], expectStates: ["callingModel"], expectContext: { steerDrained: 1, stepIndex: 0 } },
  ],
  "停步跳过后续": [
    { name: "提交并收到工具意图", events: [{ type: "submit", steerAvailable: false }, { type: "chunk", toolIntent: true }, { type: "messageEnd" }], expectStates: ["closingStep"] },
    { name: "shouldStop 请求停", events: [{ type: "stepEnd", shouldStop: true }], expectStates: ["settling"], expectContext: { stopRequested: true, toolsExecuted: 0 } },
  ],
  "中止收尾": [
    { name: "提交", events: [{ type: "submit", steerAvailable: false }], expectStates: ["callingModel"] },
    { name: "中止", events: [{ type: "settle", reason: "aborted" }], expectStates: ["settling"], expectContext: { finishReason: "aborted" } },
    { name: "落定结算", events: [{ type: "settle", reason: "aborted" }], expectStates: ["settling"], expectContext: { settlementDone: true } },
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
