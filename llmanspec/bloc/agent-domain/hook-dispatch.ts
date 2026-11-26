/**
 * Hook 分发状态机（Hook Dispatch）。
 *
 * 业务：为工具与 provider 生命周期按 event_type.phase.qualifier 匹配 hooks；
 * 脚本经 stdin JSON 执行并解析 stdout 的 allow/block/modify 动作；
 * 脚本失败默认按 Block 处理（fail-closed），仅 fail_open: true 时放行并告警；
 * 空配置零开销。
 *
 * @spec llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1（Hook 分发器）
 * @spec llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r3（脚本执行，失败默认 Block）
 * @spec llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1008（hook 匹配）
 * @spec llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1009（hook 动作 allow/block/modify）
 * @spec llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1010（hook 超时）
 * @spec llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1014（空配置零开销）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/runtime/hooks.rs、plugins/xylitol-official-plugin-hooks/。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type HookPhase = "idle" | "matching" | "executing" | "modifying" | "blocked" | "allowed";

export interface HookSpec {
  /** 匹配模式，形如 pre.tool_call.bash。 */
  pattern: string;
  /** 失败时是否放行。 */
  failOpen: boolean;
}

export interface HookContext {
  /** 已注册的 hooks。 */
  hooks: HookSpec[];
  /** 命中的 hook 数。 */
  matched: number;
  /** 已执行脚本数。 */
  executed: number;
  /** 当前工具参数（modify 可改写）。 */
  args: string;
  /** 被阻止次数。 */
  blockedCount: number;
  /** fail_open 放行告警次数。 */
  failOpenWarnings: number;
  /** 链是否被 block 终止。 */
  chainTerminated: boolean;
  /** 最近失败分类。 */
  lastFailureKind: string | null;
}

export const initialContext: HookContext = {
  hooks: [{ pattern: "pre.tool_call.bash", failOpen: false }],
  matched: 0,
  executed: 0,
  args: "rm -rf /",
  blockedCount: 0,
  failOpenWarnings: 0,
  chainTerminated: false,
  lastFailureKind: null,
};

export const meta = defineMeta({
  name: "hook-dispatch",
  pkg: "agent-domain",
  purpose: "hook 匹配与脚本动作：allow/block/modify、失败 fail-closed",
  anchors: [
    "llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1",
    "llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r3",
    "llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1008",
    "llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1009",
    "llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1010",
    "llmanspec/specs/agent-hooks/agent-hooks.feature#@req:r1014",
  ],
  impl: ["crates/xylitol-runtime/src/agent/runtime/hooks.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as HookContext,
    events: {} as
      | { type: "toolCall"; tool: string }
      | { type: "scriptResult"; action: "allow" | "block" | "modify"; args?: string }
      | { type: "scriptFailed"; kind: "spawn" | "nonzero_exit" | "invalid_json" | "io" | "timeout" }
  },
  guards: {
    hasHooks: ({ context }) => context.hooks.length > 0,
    noHooks: ({ context }) => context.hooks.length === 0,
    patternMatches: ({ context, event }) => {
      const tool = (event as { tool: string }).tool;
      return context.hooks.some((h) => h.pattern.endsWith(`.${tool}`) || h.pattern === `pre.tool_call.${tool}`);
    },
    patternMisses: ({ context, event }) => {
      const tool = (event as { tool: string }).tool;
      return !context.hooks.some((h) => h.pattern.endsWith(`.${tool}`) || h.pattern === `pre.tool_call.${tool}`);
    },
    anyFailOpen: ({ context }) => context.hooks.some((h) => h.failOpen),
    allFailClosed: ({ context }) => context.hooks.every((h) => !h.failOpen),
  },
  actions: {
    countMatch: assign({ matched: ({ context }) => context.matched + 1 }),
    countExecution: assign({ executed: ({ context }) => context.executed + 1 }),
    applyModify: assign({
      args: ({ context, event }) => (event as { args?: string }).args ?? context.args,
    }),
    blockChain: assign({
      blockedCount: ({ context }) => context.blockedCount + 1,
      chainTerminated: () => true,
    }),
    recordFailure: assign({ lastFailureKind: ({ event }) => (event as { kind: string }).kind }),
    failOpenRelease: assign({ failOpenWarnings: ({ context }) => context.failOpenWarnings + 1 }),
  },
});

export const machine = setupMachine.createMachine({
  id: "hook-dispatch",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: {
        toolCall: [
          { guard: "noHooks", target: "allowed" },
          { guard: "patternMatches", target: "executing", actions: "countMatch" },
          { guard: "patternMisses", target: "allowed" },
        ],
      },
    },
    executing: {
      on: {
        scriptResult: [
          { guard: ({ event }) => (event as { action: string }).action === "block", target: "blocked", actions: ["countExecution", "blockChain"] },
          { guard: ({ event }) => (event as { action: string }).action === "modify", target: "modifying", actions: ["countExecution", "applyModify"] },
          { guard: ({ event }) => (event as { action: string }).action === "allow", target: "allowed", actions: "countExecution" },
        ],
        scriptFailed: [
          { guard: "anyFailOpen", target: "allowed", actions: ["recordFailure", "failOpenRelease"] },
          { guard: "allFailClosed", target: "blocked", actions: ["recordFailure", "blockChain"] },
        ],
      },
    },
    modifying: {
      on: {
        toolCall: { target: "allowed" },
      },
    },
    blocked: { type: "final" },
    allowed: { type: "final" },
  },
});

export const scenarioGroups = {
  "限定符收窄匹配": [
    { name: "bash 调用命中 hook", events: [{ type: "toolCall", tool: "bash" }], expectStates: ["executing"], expectContext: { matched: 1 } },
    { name: "脚本放行", events: [{ type: "scriptResult", action: "allow" }], expectStates: ["allowed"], expectContext: { executed: 1 } },
  ],
  "未匹配工具直接放行": [
    { name: "read 调用无匹配 hook", events: [{ type: "toolCall", tool: "read" }], expectStates: ["allowed"], expectContext: { matched: 0 } },
  ],
  "hook 返回 block": [
    { name: "bash 调用命中 hook", events: [{ type: "toolCall", tool: "bash" }], expectStates: ["executing"] },
    { name: "脚本返回 block", events: [{ type: "scriptResult", action: "block" }], expectStates: ["blocked"], expectContext: { blockedCount: 1, chainTerminated: true } },
  ],
  "hook 返回 modify": [
    { name: "bash 调用命中 hook", events: [{ type: "toolCall", tool: "bash" }], expectStates: ["executing"] },
    { name: "脚本改写参数", events: [{ type: "scriptResult", action: "modify", args: "echo safe" }], expectStates: ["modifying"], expectContext: { args: "echo safe" } },
    { name: "改写后放行", events: [{ type: "toolCall", tool: "bash" }], expectStates: ["allowed"] },
  ],
  "脚本失败默认拦截": [
    { name: "bash 调用命中 hook", events: [{ type: "toolCall", tool: "bash" }], expectStates: ["executing"] },
    { name: "非零退出按 Block 处理", events: [{ type: "scriptFailed", kind: "nonzero_exit" }], expectStates: ["blocked"], expectContext: { lastFailureKind: "nonzero_exit" } },
  ],
  "超时也按拦截": [
    { name: "bash 调用命中 hook", events: [{ type: "toolCall", tool: "bash" }], expectStates: ["executing"] },
    { name: "脚本超时", events: [{ type: "scriptFailed", kind: "timeout" }], expectStates: ["blocked"], expectContext: { lastFailureKind: "timeout" } },
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