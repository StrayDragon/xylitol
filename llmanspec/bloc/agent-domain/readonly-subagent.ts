/**
 * 子代理状态机（Readonly Subagent）。
 *
 * 业务：配置不含 subagent 时零装配；配置存在时注册 subagent_readonly，
 * 另建隔离 runtime 与临时会话，只暴露只读工具（不得含 write / edit / bash）；
 * 父级取消须 abort 子 runtime；子代理返回最后 assistant 文本，空文本须报错。
 *
 * @spec llmanspec/specs/agent-subagent/agent-subagent.feature#@req:r2999（未配置零装配）
 * @spec llmanspec/specs/agent-subagent/agent-subagent.feature#@req:r3000（配置后注册只读子代理）
 * @spec llmanspec/specs/agent-tools/agent-tools.feature#@req:r32（工具 trait 与命名前缀）
 * @spec llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1030（abort 取消流：父级传播）
 *
 * 实现落点：crates/xylitol-kernel/src/subagent.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, runScenarios, type ScenarioStep } from "../shared/conventions.ts";

export type SubagentPhase = "unconfigured" | "registered" | "spawned" | "running" | "returned" | "aborted" | "failed";

/** 默认允许的只读工具集。 */
export const DEFAULT_READONLY_TOOLS = ["read", "grep", "find", "ls", "ffgrep", "fffind"] as const;

export interface SubagentContext {
  /** 配置是否含 subagent。 */
  configured: boolean;
  /** 工具注册表是否含 subagent_readonly。 */
  toolRegistered: boolean;
  /** 子代理可见工具集。 */
  visibleTools: string[];
  /** 是否已建独立 runtime。 */
  isolatedRuntime: boolean;
  /** 是否共享父 session（须为 false）。 */
  sharesParentSession: boolean;
  /** 子代理返回文本。 */
  resultText: string | null;
  /** 父级取消是否传播。 */
  parentCancelPropagated: boolean;
}

export const initialContext: SubagentContext = {
  configured: true,
  toolRegistered: false,
  visibleTools: [],
  isolatedRuntime: false,
  sharesParentSession: false,
  resultText: null,
  parentCancelPropagated: false,
};

export const meta = defineMeta({
  name: "readonly-subagent",
  pkg: "agent-domain",
  purpose: "只读子代理：零装配、隔离 runtime、只读工具集、父级取消传播",
  anchors: [
    "llmanspec/specs/agent-subagent/agent-subagent.feature#@req:r2999",
    "llmanspec/specs/agent-subagent/agent-subagent.feature#@req:r3000",
    "llmanspec/specs/agent-tools/agent-tools.feature#@req:r32",
    "llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1030",
  ],
  impl: ["crates/xylitol-kernel/src/subagent.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as SubagentContext,
    events: {} as
      | { type: "assemble" }
      | { type: "invoke"; prompt: string }
      | { type: "childDone"; text: string }
      | { type: "childErrored" }
      | { type: "parentCancel" }
  },
  guards: {
    isConfigured: ({ context }) => context.configured === true,
    isUnconfigured: ({ context }) => context.configured === false,
    textNonEmpty: ({ event }) => ((event as { text: string }).text ?? "").trim().length > 0,
    textEmpty: ({ event }) => ((event as { text: string }).text ?? "").trim().length === 0,
  },
  actions: {
    registerTool: assign({
      toolRegistered: () => true,
      visibleTools: () => [...DEFAULT_READONLY_TOOLS],
    }),
    spawnIsolated: assign({
      isolatedRuntime: () => true,
      sharesParentSession: () => false,
    }),
    recordText: assign({ resultText: ({ event }) => (event as { text: string }).text }),
    propagateCancel: assign({ parentCancelPropagated: () => true }),
  },
});

/** 按配置生成机器：configured=false 走零装配分支，true 走注册分支。 */
export function makeSubagentMachine(configured: boolean) {
  return setupMachine.createMachine({
    id: "readonly-subagent",
    initial: "unconfigured",
    context: { ...initialContext, configured },
    states: subagentStates,
  });
}

/** 状态表抽出，供两种配置共用。 */
const subagentStates = {
    unconfigured: {
      on: {
        assemble: [
          { guard: "isConfigured", target: "registered", actions: "registerTool" },
          { guard: "isUnconfigured" },
        ],
      },
    },
    registered: {
      on: {
        invoke: { target: "spawned", actions: "spawnIsolated" },
      },
    },
    spawned: {
      always: [{ target: "running" }],
    },
    running: {
      on: {
        childDone: [
          { guard: "textNonEmpty", target: "returned", actions: "recordText" },
          { guard: "textEmpty", target: "failed" },
        ],
        childErrored: { target: "failed" },
        parentCancel: { target: "aborted", actions: "propagateCancel" },
      },
    },
    returned: { type: "final" },
    aborted: { type: "final" },
    failed: { type: "final" },
} as const;

/** 未配置机器：用于验证零装配分支。 */
export const unconfiguredMachine = makeSubagentMachine(false);

/** 已配置机器：用于验证注册与调用分支。 */
export const machine = makeSubagentMachine(true);

export const scenarioGroups = {
  "未配置零装配": [
    { name: "装配时未配置", events: [{ type: "assemble" }], expectStates: ["unconfigured"], expectContext: { toolRegistered: false, isolatedRuntime: false } },
  ],
  "配置后注册只读工具": [
    { name: "装配", events: [{ type: "assemble" }], expectStates: ["registered"], expectContext: { toolRegistered: true } },
    { name: "可见工具只含只读集", expect: (s) => {
        const tools = (s as { context: SubagentContext }).context.visibleTools;
        const forbidden = ["write", "edit", "bash"];
        const hit = tools.filter((t) => forbidden.includes(t));
        return hit.length === 0 ? true : `可见工具含禁用项 ${hit.join(",")}`;
      } },
  ],
  "隔离 runtime 不共享父会话": [
    { name: "装配并调用", events: [{ type: "assemble" }, { type: "invoke", prompt: "查一下" }], expectStates: ["running"], expectContext: { isolatedRuntime: true, sharesParentSession: false } },
  ],
  "返回最后 assistant 文本": [
    { name: "装配并调用", events: [{ type: "assemble" }, { type: "invoke", prompt: "查一下" }], expectStates: ["running"] },
    { name: "子代理返回文本", events: [{ type: "childDone", text: "结果是 X" }], expectStates: ["returned"], expectContext: { resultText: "结果是 X" } },
  ],
  "空文本报错": [
    { name: "装配并调用", events: [{ type: "assemble" }, { type: "invoke", prompt: "查一下" }], expectStates: ["running"] },
    { name: "子代理返回空文本", events: [{ type: "childDone", text: "   " }], expectStates: ["failed"] },
  ],
  "父级取消传播": [
    { name: "装配并调用", events: [{ type: "assemble" }, { type: "invoke", prompt: "查一下" }], expectStates: ["running"] },
    { name: "父级中止", events: [{ type: "parentCancel" }], expectStates: ["aborted"], expectContext: { parentCancelPropagated: true } },
  ],
} satisfies Readonly<Record<string, readonly ScenarioStep[]>>;

if (import.meta.main) {
  const unconfiguredResult = runScenarios(unconfiguredMachine, scenarioGroups["未配置零装配"]!);
  const configuredGroups = { ...scenarioGroups };
  delete (configuredGroups as Record<string, unknown>)["未配置零装配"];
  const results = { ...runScenarioGroups(machine, configuredGroups), "未配置零装配": unconfiguredResult };
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