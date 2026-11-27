/**
 * 会话级目标状态机（Goal Lifecycle）。
 *
 * 业务：goal 状态由事实投影重建；合法转移表为
 * Open→{Running, Paused, Cancelled}；Running→{Done, Failed, Blocked, Paused, Cancelled}；
 * Blocked→{Running, Done, Failed, Paused, Cancelled}；Paused→{Running, Cancelled}；
 * Done / Failed / Cancelled 为终态锁定。非法转移须拒绝并回 LLM 可见错误。
 * evaluator 每轮 settle 后独立判定；连续 3 次失败/非法转 blocked；max_iterations 缺省 256。
 *
 * @spec llmanspec/specs/agent-goal/agent-goal.feature#@req:r3318（goal-state-transitions）
 * @spec llmanspec/specs/agent-goal/agent-goal.feature#@req:r3299（goal-facts）
 * @spec llmanspec/specs/agent-goal/agent-goal.feature#@req:r3300（goal-evaluator）
 * @spec llmanspec/specs/agent-goal/agent-goal.feature#@req:r3301（goal-decide-continue）
 * @spec llmanspec/specs/agent-goal/agent-goal.feature#@req:r3302（goal-loop-interaction）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/automation.rs、crates/xylitol-runtime/src/agent/runtime/react/step_end.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type GoalState = "open" | "running" | "paused" | "cancelled" | "done" | "failed" | "blocked";

/** 终态集合：锁定后不得再转移。 */
export const TERMINAL_GOAL_STATES: readonly GoalState[] = ["done", "failed", "cancelled"];

export interface GoalContext {
  /** 当前 goal 状态。 */
  goalState: GoalState;
  /** 已用迭代数（只计 GoalEvaluated）。 */
  iterationsUsed: number;
  /** 迭代上限，缺省 256。 */
  maxIterations: number;
  /** 连续 evaluator 失败/非法次数。 */
  consecutiveEvaluatorFailures: number;
  /** 非法转移被拒绝次数。 */
  rejectedTransitions: number;
  /** 最近一次 evaluator verdict。 */
  lastVerdict: string | null;
  /** 是否已追加 GoalStateChanged 事实。 */
  stateFactAppended: boolean;
  /** 下一条提交是否携带 not_met 指导。 */
  guidanceQueued: boolean;
}

export const initialContext: GoalContext = {
  goalState: "open",
  iterationsUsed: 0,
  maxIterations: 256,
  consecutiveEvaluatorFailures: 0,
  rejectedTransitions: 0,
  lastVerdict: null,
  stateFactAppended: false,
  guidanceQueued: false,
};

export const meta = defineMeta({
  name: "goal-lifecycle",
  pkg: "agent-domain",
  purpose: "会话级目标：合法转移表、evaluator 判定、逐 turn 续跑",
  anchors: [
    "llmanspec/specs/agent-goal/agent-goal.feature#@req:r3318",
    "llmanspec/specs/agent-goal/agent-goal.feature#@req:r3299",
    "llmanspec/specs/agent-goal/agent-goal.feature#@req:r3300",
    "llmanspec/specs/agent-goal/agent-goal.feature#@req:r3301",
    "llmanspec/specs/agent-goal/agent-goal.feature#@req:r3302",
  ],
  impl: ["crates/xylitol-runtime/src/agent/automation.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as GoalContext,
    events: {} as
      | { type: "start" }
      | { type: "evaluated"; verdict: "met" | "not_met" | "impossible" | "invalid"; hasEvidence: boolean }
      | { type: "evaluatorFailed" }
      | { type: "pause" }
      | { type: "resume" }
      | { type: "cancel" }
      | { type: "approveUnblock" }
      | { type: "fail" }
  },
  guards: {
    verdictMetWithEvidence: ({ event }) => {
      const e = event as { verdict: string; hasEvidence: boolean };
      return e.verdict === "met" && e.hasEvidence === true;
    },
    verdictNotMet: ({ event }) => (event as { verdict: string }).verdict === "not_met",
    verdictImpossible: ({ event }) => (event as { verdict: string }).verdict === "impossible",
    verdictInvalid: ({ event }) => (event as { verdict: string }).verdict === "invalid",
    iterationCapReached: ({ context }) => context.iterationsUsed >= context.maxIterations,
    iterationCapOpen: ({ context }) => context.iterationsUsed < context.maxIterations,
    threeConsecutiveFailures: ({ context }) => context.consecutiveEvaluatorFailures + 1 >= 3,
  },
  actions: {
    enterRunning: assign({
      goalState: () => "running",
      stateFactAppended: () => true,
    }),
    countIteration: assign({ iterationsUsed: ({ context }) => context.iterationsUsed + 1 }),
    clearFailureStreak: assign({ consecutiveEvaluatorFailures: () => 0 }),
    recordVerdict: assign({ lastVerdict: ({ event }) => (event as { verdict: string }).verdict }),
    markDone: assign({ goalState: () => "done", stateFactAppended: () => true }),
    markFailed: assign({ goalState: () => "failed", stateFactAppended: () => true }),
    markBlocked: assign({ goalState: () => "blocked", stateFactAppended: () => true }),
    markPaused: assign({ goalState: () => "paused", stateFactAppended: () => true }),
    markCancelled: assign({ goalState: () => "cancelled", stateFactAppended: () => true }),
    queueGuidance: assign({ guidanceQueued: () => true }),
    countFailure: assign({ consecutiveEvaluatorFailures: ({ context }) => context.consecutiveEvaluatorFailures + 1 }),
    rejectTransition: assign({ rejectedTransitions: ({ context }) => context.rejectedTransitions + 1 }),
  },
});

export const machine = setupMachine.createMachine({
  id: "goal-lifecycle",
  initial: "open",
  context: initialContext,
  states: {
    open: {
      on: {
        start: { target: "running", actions: "enterRunning" },
        pause: { target: "paused", actions: "markPaused" },
        cancel: { target: "cancelled", actions: "markCancelled" },
      },
    },
    running: {
      on: {
        evaluated: [
          { guard: "verdictMetWithEvidence", target: "done", actions: ["recordVerdict", "countIteration", "markDone"] },
          { guard: "verdictImpossible", target: "failed", actions: ["recordVerdict", "countIteration", "markFailed"] },
          { guard: "verdictNotMet", target: "running", actions: ["recordVerdict", "countIteration", "queueGuidance", "clearFailureStreak"] },
          { guard: "verdictInvalid", actions: ["countFailure", "rejectTransition"] },
        ],
        evaluatorFailed: [
          { guard: "threeConsecutiveFailures", target: "blocked", actions: ["countFailure", "markBlocked"] },
          { actions: "countFailure" },
        ],
        pause: { target: "paused", actions: "markPaused" },
        cancel: { target: "cancelled", actions: "markCancelled" },
        fail: { target: "failed", actions: "markFailed" },
      },
    },
    blocked: {
      on: {
        approveUnblock: { target: "running", actions: "enterRunning" },
        evaluated: [
          { guard: "verdictMetWithEvidence", target: "done", actions: ["recordVerdict", "markDone"] },
          { guard: "verdictImpossible", target: "failed", actions: ["recordVerdict", "markFailed"] },
          { actions: "rejectTransition" },
        ],
        pause: { target: "paused", actions: "markPaused" },
        cancel: { target: "cancelled", actions: "markCancelled" },
      },
    },
    paused: {
      on: {
        resume: { target: "running", actions: "enterRunning" },
        cancel: { target: "cancelled", actions: "markCancelled" },
        evaluated: { actions: "rejectTransition" },
      },
    },
    done: {
      on: {
        start: { actions: "rejectTransition" },
        resume: { actions: "rejectTransition" },
        pause: { actions: "rejectTransition" },
        cancel: { actions: "rejectTransition" },
        evaluated: { actions: "rejectTransition" },
      },
    },
    failed: {
      on: {
        start: { actions: "rejectTransition" },
        resume: { actions: "rejectTransition" },
        pause: { actions: "rejectTransition" },
        cancel: { actions: "rejectTransition" },
        evaluated: { actions: "rejectTransition" },
      },
    },
    cancelled: {
      on: {
        start: { actions: "rejectTransition" },
        resume: { actions: "rejectTransition" },
        pause: { actions: "rejectTransition" },
        evaluated: { actions: "rejectTransition" },
      },
    },
  },
});

export const scenarioGroups = {
  "Open到Running": [
    { name: "开始运行", events: [{ type: "start" }], expectStates: ["running"], expectContext: { goalState: "running", stateFactAppended: true } },
  ],
  "met带证据转Done": [
    { name: "开始运行", events: [{ type: "start" }], expectStates: ["running"] },
    { name: "evaluator 判定 met 且有证据", events: [{ type: "evaluated", verdict: "met", hasEvidence: true }], expectStates: ["done"], expectContext: { goalState: "done", iterationsUsed: 1 } },
  ],
  "met无证据按not_met": [
    { name: "开始运行", events: [{ type: "start" }], expectStates: ["running"] },
    { name: "met 但无证据仍续跑", events: [{ type: "evaluated", verdict: "not_met", hasEvidence: false }], expectStates: ["running"], expectContext: { guidanceQueued: true, iterationsUsed: 1 } },
  ],
  "not_met续跑带指导": [
    { name: "开始运行", events: [{ type: "start" }], expectStates: ["running"] },
    { name: "not_met 生成下一条指导", events: [{ type: "evaluated", verdict: "not_met", hasEvidence: false }], expectStates: ["running"], expectContext: { guidanceQueued: true } },
  ],
  "impossible转Failed": [
    { name: "开始运行", events: [{ type: "start" }], expectStates: ["running"] },
    { name: "evaluator 判定 impossible", events: [{ type: "evaluated", verdict: "impossible", hasEvidence: false }], expectStates: ["failed"], expectContext: { goalState: "failed" } },
  ],
  "连续三次失败转Blocked": [
    { name: "开始运行", events: [{ type: "start" }], expectStates: ["running"] },
    { name: "两次失败仍运行", events: [{ type: "evaluatorFailed" }, { type: "evaluatorFailed" }], expectStates: ["running"], expectContext: { consecutiveEvaluatorFailures: 2 } },
    { name: "第三次失败转 blocked", events: [{ type: "evaluatorFailed" }], expectStates: ["blocked"], expectContext: { consecutiveEvaluatorFailures: 3, goalState: "blocked" } },
  ],
  "Blocked批准后回Running": [
    { name: "开始运行并失败三次", events: [{ type: "start" }, { type: "evaluatorFailed" }, { type: "evaluatorFailed" }, { type: "evaluatorFailed" }], expectStates: ["blocked"] },
    { name: "用户批准解除阻塞", events: [{ type: "approveUnblock" }], expectStates: ["running"], expectContext: { goalState: "running" } },
  ],
  "终态锁定拒绝非法转移": [
    { name: "开始运行", events: [{ type: "start" }], expectStates: ["running"] },
    { name: "判定完成", events: [{ type: "evaluated", verdict: "met", hasEvidence: true }], expectStates: ["done"] },
    { name: "done 后再 pause 被拒绝", events: [{ type: "pause" }], expectStates: ["done"], expectContext: { goalState: "done", rejectedTransitions: 1 } },
  ],
  "Paused只能Running或Cancelled": [
    { name: "开始运行后暂停", events: [{ type: "start" }, { type: "pause" }], expectStates: ["paused"] },
    { name: "暂停时 evaluator 判定被拒绝", events: [{ type: "evaluated", verdict: "met", hasEvidence: true }], expectStates: ["paused"], expectContext: { rejectedTransitions: 1 } },
    { name: "恢复运行", events: [{ type: "resume" }], expectStates: ["running"] },
  ],
  "Cancelled为终态": [
    { name: "开始运行后取消", events: [{ type: "start" }, { type: "cancel" }], expectStates: ["cancelled"], expectContext: { goalState: "cancelled" } },
    { name: "取消后 resume 被拒绝", events: [{ type: "resume" }], expectStates: ["cancelled"], expectContext: { rejectedTransitions: 1 } },
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
