/**
 * Codemode 运行状态机（PTC / run_code）。
 *
 * 业务：工具暴露与 catalog 交集、声明预算、嵌套 dispatch、Wasmtime 隔离与限额、
 * 未启用零装配与缓存、提交隔离。
 *
 * @spec llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3250（exposure-and-catalog）
 * @spec llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3251（tool-exposure-intersection）
 * @spec llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3252（declaration-budget）
 * @spec llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3253（nested-dispatch）
 * @spec llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3254（wasmtime-limits）
 * @spec llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3255（zero-assembly-and-cache）
 * @spec llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3347（codemode-submission-isolation）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r3258（codemode-exec-permission）
 *
 * 实现落点：plugins/xylitol-official-plugin-codemode/。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type CodemodePhase = "disabled" | "ready" | "running" | "dispatchNested" | "finished" | "rejected";

export interface CodemodeContext {
  /** 插件是否启用。 */
  enabled: boolean;
  /** 暴露给脚本的工具数。 */
  exposedTools: number;
  /** 声明 token 预算。 */
  declarationBudget: number;
  /** 已用声明 token。 */
  declarationUsed: number;
  /** 当前执行已用毫秒。 */
  elapsedMs: number;
  /** 执行时限毫秒。 */
  maxRuntimeMs: number;
  /** 嵌套 dispatch 次数。 */
  nestedDispatches: number;
  /** 被拒绝的提交数。 */
  rejectedSubmissions: number;
  /** 脚本结果。 */
  result: string | null;
  /** 是否在隔离沙箱内执行。 */
  sandboxed: boolean;
}

export const initialContext: CodemodeContext = {
  enabled: true,
  exposedTools: 12,
  declarationBudget: 8000,
  declarationUsed: 0,
  elapsedMs: 0,
  maxRuntimeMs: 30000,
  nestedDispatches: 0,
  rejectedSubmissions: 0,
  result: null,
  sandboxed: true,
};

export const meta = defineMeta({
  name: "codemode-runtime",
  pkg: "agent-domain",
  purpose: "run_code 执行：暴露交集、声明预算、沙箱限额、嵌套 dispatch",
  anchors: [
    "llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3250",
    "llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3251",
    "llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3252",
    "llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3253",
    "llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3254",
    "llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3255",
    "llmanspec/specs/agent-codemode/agent-codemode.feature#@req:r3347",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r3258",
  ],
  impl: ["plugins/xylitol-official-plugin-codemode/"],
});

export const setupMachine = setup({
  types: {
    context: {} as CodemodeContext,
    events: {} as
      | { type: "assemble" }
      | { type: "submit"; declarations: number }
      | { type: "dispatch"; tool: string }
      | { type: "elapsed"; ms: number }
      | { type: "done"; result: string }
      | { type: "failed"; kind: string }
  },
  guards: {
    pluginDisabled: ({ context }) => context.enabled === false,
    pluginEnabled: ({ context }) => context.enabled === true,
    withinDeclarationBudget: ({ context, event }) =>
      context.declarationUsed + (event as { declarations: number }).declarations <= context.declarationBudget,
    overDeclarationBudget: ({ context, event }) =>
      context.declarationUsed + (event as { declarations: number }).declarations > context.declarationBudget,
    withinRuntimeLimit: ({ context, event }) => context.elapsedMs + (event as { ms: number }).ms <= context.maxRuntimeMs,
    overRuntimeLimit: ({ context, event }) => context.elapsedMs + (event as { ms: number }).ms > context.maxRuntimeMs,
  },
  actions: {
    countDeclarations: assign({ declarationUsed: ({ context, event }) => context.declarationUsed + (event as { declarations: number }).declarations }),
    rejectSubmission: assign({ rejectedSubmissions: ({ context }) => context.rejectedSubmissions + 1 }),
    countNested: assign({ nestedDispatches: ({ context }) => context.nestedDispatches + 1 }),
    advanceClock: assign({ elapsedMs: ({ context, event }) => context.elapsedMs + (event as { ms: number }).ms }),
    recordResult: assign({ result: ({ event }) => (event as { result: string }).result }),
  },
});

export const machine = setupMachine.createMachine({
  id: "codemode-runtime",
  initial: "disabled",
  context: initialContext,
  states: {
    disabled: {
      on: {
        assemble: [
          { guard: "pluginEnabled", target: "ready" },
          { guard: "pluginDisabled" },
        ],
        submit: { actions: "rejectSubmission" },
      },
    },
    ready: {
      on: {
        submit: [
          { guard: "withinDeclarationBudget", target: "running", actions: "countDeclarations" },
          { guard: "overDeclarationBudget", actions: "rejectSubmission" },
        ],
      },
    },
    running: {
      on: {
        dispatch: { target: "dispatchNested", actions: "countNested" },
        elapsed: [
          { guard: "withinRuntimeLimit", actions: "advanceClock" },
          { guard: "overRuntimeLimit", target: "rejected", actions: "rejectSubmission" },
        ],
        done: { target: "finished", actions: "recordResult" },
        failed: { target: "rejected" },
      },
    },
    dispatchNested: {
      always: [{ target: "running" }],
    },
    finished: { type: "final" },
    rejected: { type: "final" },
  },
});

export const scenarioGroups = {
  "启用后提交执行": [
    { name: "装配启用", events: [{ type: "assemble" }], expectStates: ["ready"] },
    { name: "提交脚本", events: [{ type: "submit", declarations: 2000 }], expectStates: ["running"], expectContext: { declarationUsed: 2000 } },
    { name: "执行完成", events: [{ type: "done", result: "ok" }], expectStates: ["finished"], expectContext: { result: "ok", sandboxed: true } },
  ],
  "声明超预算被拒": [
    { name: "装配启用", events: [{ type: "assemble" }], expectStates: ["ready"] },
    { name: "提交超预算声明", events: [{ type: "submit", declarations: 9000 }], expectStates: ["ready"], expectContext: { rejectedSubmissions: 1 } },
  ],
  "嵌套 dispatch": [
    { name: "装配并提交", events: [{ type: "assemble" }, { type: "submit", declarations: 100 }], expectStates: ["running"] },
    { name: "嵌套调用工具", events: [{ type: "dispatch", tool: "read" }], expectStates: ["running"], expectContext: { nestedDispatches: 1 } },
  ],
  "超出沙箱时限被拒": [
    { name: "装配并提交", events: [{ type: "assemble" }, { type: "submit", declarations: 100 }], expectStates: ["running"] },
    { name: "执行超时", events: [{ type: "elapsed", ms: 40000 }], expectStates: ["rejected"], expectContext: { rejectedSubmissions: 1 } },
  ],
  "执行失败": [
    { name: "装配并提交", events: [{ type: "assemble" }, { type: "submit", declarations: 100 }], expectStates: ["running"] },
    { name: "脚本异常", events: [{ type: "failed", kind: "wasm_trap" }], expectStates: ["rejected"] },
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