/**
 * LSP/DAP 工具插件装配状态机（LSP/DAP Tool Registration）。
 *
 * 业务：未配置零装配（不注册工具、不 spawn 后端、无可感知成本）；有配置才
 * 注册对应工具集；首次工具调用才启动后端；后端相位显式建模（未启动/运行中/
 * 已死）；崩溃后请求显式报不可用，下一次调用重建一次并记录重启观测；卸载
 * 须 shutdown / disconnect+terminate。
 *
 * @spec llmanspec/specs/infra-lsp/infra-lsp.feature#@req:r2993（LSP 未配置零装配）
 * @spec llmanspec/specs/infra-lsp/infra-lsp.feature#@req:r2994（配置后注册 LSP 工具，首调用启动）
 * @spec llmanspec/specs/infra-lsp/infra-lsp.feature#@req:r3348（LSP 相位显式与重建观测）
 * @spec llmanspec/specs/infra-dap/infra-dap.feature#@req:r2996（DAP 未配置零装配）
 * @spec llmanspec/specs/infra-dap/infra-dap.feature#@req:r2997（配置后注册 DAP 工具）
 * @spec llmanspec/specs/infra-dap/infra-dap.feature#@req:r3349（DAP 相位显式与重建观测）
 *
 * 实现落点：plugins/xylitol-official-plugin-lsp/src/lib.rs、
 * plugins/xylitol-official-plugin-dap/src/lib.rs。
 *
 * 建模说明：同一台机器以 kind 区分 lsp/dap，两个后端的相位语义由 r3348 与
 * r3349 共享，故合并建模；场景组分别驱动两种 kind。dead 态的首次 toolCall
 * 只报不可用（不得向死进程发请求），deadReported 置位后的下一次 toolCall
 * 才重建一次并记重启观测。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type BackendKind = "lsp" | "dap";

/** 后端显式相位（r3348/r3349）：not-started / running / dead。 */
export type BackendPhase = "not-started" | "running" | "dead";

export const LSP_TOOLS = ["lsp_diagnostics", "lsp_symbols", "lsp_hover"] as const;
export const DAP_TOOLS = [
  "dap_launch",
  "dap_stack",
  "dap_scopes",
  "dap_variables",
  "dap_evaluate",
  "dap_continue",
  "dap_disconnect",
] as const;

export function toolsForKind(kind: BackendKind): readonly string[] {
  return kind === "lsp" ? LSP_TOOLS : DAP_TOOLS;
}

export interface BootEvent {
  type: "boot";
  kind: BackendKind;
  configured: boolean;
}

export interface RegistrationContext {
  kind: BackendKind | null;
  configured: boolean;
  registeredTools: readonly string[];
  phase: BackendPhase;
  servedCalls: number;
  crashes: number;
  /** 死后收到的请求按显式不可用计数，不得静默发往死进程。 */
  unavailableRequests: number;
  /** 本轮死后是否已报过不可用；报过之后的下一次调用才重建。 */
  deadReported: boolean;
  /** 重建观测计数（每次崩溃后只重建一次）。 */
  restartObservations: number;
  shutdownDone: boolean;
  refusedCalls: number;
}

export const initialContext: RegistrationContext = {
  kind: null,
  configured: false,
  registeredTools: [],
  phase: "not-started",
  servedCalls: 0,
  crashes: 0,
  unavailableRequests: 0,
  deadReported: false,
  restartObservations: 0,
  shutdownDone: false,
  refusedCalls: 0,
};

export const meta = defineMeta({
  name: "lsp-dap-tool-registration",
  pkg: "security-infra",
  purpose: "LSP/DAP 工具装配：零配置零装配、首调用启动、相位显式、崩溃后重建一次",
  anchors: [
    "llmanspec/specs/infra-lsp/infra-lsp.feature#@req:r2993",
    "llmanspec/specs/infra-lsp/infra-lsp.feature#@req:r2994",
    "llmanspec/specs/infra-lsp/infra-lsp.feature#@req:r3348",
    "llmanspec/specs/infra-dap/infra-dap.feature#@req:r2996",
    "llmanspec/specs/infra-dap/infra-dap.feature#@req:r2997",
    "llmanspec/specs/infra-dap/infra-dap.feature#@req:r3349",
  ],
  impl: [
    "plugins/xylitol-official-plugin-lsp/src/lib.rs",
    "plugins/xylitol-official-plugin-dap/src/lib.rs",
  ],
});

export const setupMachine = setup({
  types: {
    context: {} as RegistrationContext,
    events: {} as
      | BootEvent
      | { type: "toolCall" }
      | { type: "crash" }
      | { type: "unload" },
  },
  guards: {
    configPresent: ({ event }) => (event as BootEvent).configured === true,
    deadNotYetReported: ({ context }) => !context.deadReported,
  },
  actions: {
    registerTools: assign(({ event }) => {
      const e = event as BootEvent;
      return {
        kind: e.kind,
        configured: true,
        registeredTools: toolsForKind(e.kind),
        phase: "not-started" as const,
        servedCalls: 0,
        unavailableRequests: 0,
        deadReported: false,
        restartObservations: 0,
        shutdownDone: false,
      };
    }),
    noteZeroAssembly: assign({
      configured: () => false,
      registeredTools: () => [],
    }),
    refuseCall: assign(({ context }) => ({
      refusedCalls: context.refusedCalls + 1,
    })),
    startBackend: assign(({ context }) => ({
      phase: "running" as const,
      servedCalls: context.servedCalls + 1,
    })),
    serve: assign(({ context }) => ({
      servedCalls: context.servedCalls + 1,
    })),
    markDead: assign(({ context }) => ({
      phase: "dead" as const,
      crashes: context.crashes + 1,
      deadReported: false,
    })),
    reportUnavailable: assign(({ context }) => ({
      unavailableRequests: context.unavailableRequests + 1,
      deadReported: true,
    })),
    rebuildOnce: assign(({ context }) => ({
      phase: "running" as const,
      restartObservations: context.restartObservations + 1,
      deadReported: false,
      servedCalls: context.servedCalls + 1,
    })),
    shutdown: assign(() => ({
      configured: false,
      registeredTools: [],
      phase: "not-started" as const,
      servedCalls: 0,
      deadReported: false,
      shutdownDone: true,
      // r2997：DAP 卸载走 disconnect + terminate，与 shutdown 同一收口动作。
    })),
  },
});

export const machine = setupMachine.createMachine({
  id: "lsp-dap-tool-registration",
  initial: "bare",
  context: initialContext,
  states: {
    bare: {
      on: {
        boot: [
          { guard: "configPresent", target: "registered", actions: "registerTools" },
          { actions: "noteZeroAssembly" },
        ],
        toolCall: { actions: "refuseCall" },
      },
    },
    registered: {
      on: {
        // r2994/r2997：首次工具调用才启动后端。
        toolCall: { target: "running", actions: "startBackend" },
        unload: { target: "bare", actions: "shutdown" },
      },
    },
    running: {
      on: {
        toolCall: { actions: "serve" },
        crash: { target: "dead", actions: "markDead" },
        unload: { target: "bare", actions: "shutdown" },
      },
    },
    dead: {
      on: {
        toolCall: [
          // r3348/r3349：死后首个请求显式报不可用，不发往死进程。
          { guard: "deadNotYetReported", actions: "reportUnavailable" },
          // 下一次调用重建一次并记录重启观测。
          { target: "running", actions: "rebuildOnce" },
        ],
        unload: { target: "bare", actions: "shutdown" },
      },
    },
  },
});

export const scenarioGroups = {
  "未配置零装配": [
    {
      name: "boot 无 lsp/dap 配置时零装配（r2993/r2996）",
      events: [
        { type: "boot", kind: "lsp", configured: false },
      ],
      expectStates: ["bare"],
      expectContext: { configured: false, registeredTools: [] },
    },
    {
      name: "未注册时工具调用被显式拒绝，不 spawn",
      events: [{ type: "toolCall" }],
      expectStates: ["bare"],
      expectContext: { refusedCalls: 1, phase: "not-started" },
    },
  ],
  "LSP 配置注册与首调用启动": [
    {
      name: "配置 lsp 后注册三个工具（r2994）",
      events: [{ type: "boot", kind: "lsp", configured: true }],
      expectStates: ["registered"],
      expectContext: { kind: "lsp", registeredTools: ["lsp_diagnostics", "lsp_symbols", "lsp_hover"], phase: "not-started" },
    },
    {
      name: "首次工具调用启动后端（r2994）",
      events: [{ type: "toolCall" }],
      expectStates: ["running"],
      expectContext: { phase: "running", servedCalls: 1 },
    },
  ],
  "LSP 崩溃显式不可用与重建": [
    {
      name: "运行中的后端崩溃进入 dead 相位（r3348）",
      events: [
        { type: "boot", kind: "lsp", configured: true },
        { type: "toolCall" },
        { type: "crash" },
      ],
      expectStates: ["dead"],
      expectContext: { phase: "dead", crashes: 1, deadReported: false },
    },
    {
      name: "崩溃后首个请求显式报不可用，不发往死进程（r3348）",
      events: [{ type: "toolCall" }],
      expectStates: ["dead"],
      expectContext: { unavailableRequests: 1, deadReported: true },
    },
    {
      name: "下一次调用重建一次并记录重启观测",
      events: [{ type: "toolCall" }],
      expectStates: ["running"],
      expectContext: { restartObservations: 1, servedCalls: 2, phase: "running" },
    },
  ],
  "DAP 注册七工具": [
    {
      name: "配置 dap 后注册七个工具（r2997）",
      events: [{ type: "boot", kind: "dap", configured: true }],
      expectStates: ["registered"],
      expectContext: { kind: "dap", registeredTools: ["dap_launch", "dap_stack", "dap_scopes", "dap_variables", "dap_evaluate", "dap_continue", "dap_disconnect"] },
    },
    {
      name: "首调用启动适配器，后续调用正常服务",
      events: [
        { type: "toolCall" },
        { type: "toolCall" },
      ],
      expectStates: ["running"],
      expectContext: { servedCalls: 2, phase: "running" },
    },
  ],
  "DAP 卸载 shutdown": [
    {
      name: "运行中卸载回零装配并收口 shutdown（r2997）",
      events: [
        { type: "boot", kind: "dap", configured: true },
        { type: "toolCall" },
        { type: "unload" },
      ],
      expectStates: ["bare"],
      expectContext: { configured: false, registeredTools: [], shutdownDone: true, phase: "not-started" },
    },
    {
      name: "卸载后工具调用被拒，回到零装配语义（r2996）",
      events: [{ type: "toolCall" }],
      expectStates: ["bare"],
      expectContext: { refusedCalls: 1 },
    },
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
