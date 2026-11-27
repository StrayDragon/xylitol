/**
 * OpenTelemetry 出口状态机（OTEL Export）。
 *
 * 业务：默认与失败路径均不收集；Cargo feature otel + 配置显式开启后才出口；
 * 经 fastrace-opentelemetry 导出，与文件管线 fanout；失败不阻塞；
 * Langfuse 会话、GenAI 属性与 turn 内父子 span 树在 opt-in 之上叠加。
 *
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1466（otel-default-none）
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1477（otel-config-opt-in）
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1486（otel-fallback-no-block）
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1487（otel-fastrace-only）
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1488（otel-fanout-with-file）
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1489（otel-session-id-always）
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1468（otel-turn-span-tree）
 * @spec llmanspec/specs/infra-otel/infra-otel.feature#@req:r1478（otel-turn-terminal-status）
 *
 * 实现落点：plugins/xylitol-official-plugin-obs/。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, runScenarios, type ScenarioStep } from "../shared/conventions.ts";

export type OtelPhase = "disabled" | "initializing" | "exporting" | "degradedFileOnly" | "shutdown";

export interface OtelContext {
  /** Cargo feature otel 是否编译进。 */
  featureEnabled: boolean;
  /** 配置是否显式开启。 */
  configOptIn: boolean;
  /** 已导出 span 数。 */
  exportedSpans: number;
  /** 文件管线是否仍在写。 */
  filePipelineActive: boolean;
  /** 出口失败次数。 */
  exportFailures: number;
  /** 是否阻塞了主流程（须为 false）。 */
  blockedMainPath: boolean;
  /** 是否带 session_id。 */
  hasSessionId: boolean;
  /** 当前 turn 的父 span 数。 */
  turnParentSpans: number;
  /** turn 终态。 */
  turnStatus: string | null;
}

export const initialContext: OtelContext = {
  featureEnabled: true,
  configOptIn: false,
  exportedSpans: 0,
  filePipelineActive: true,
  exportFailures: 0,
  blockedMainPath: false,
  hasSessionId: true,
  turnParentSpans: 0,
  turnStatus: null,
};

export const meta = defineMeta({
  name: "otel-export",
  pkg: "config-meta",
  purpose: "OTEL 出口：默认不收集、opt-in、失败不阻塞、与文件管线 fanout",
  anchors: [
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1466",
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1477",
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1486",
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1487",
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1488",
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1489",
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1468",
    "llmanspec/specs/infra-otel/infra-otel.feature#@req:r1478",
  ],
  impl: ["plugins/xylitol-official-plugin-obs/"],
});

export const setupMachine = setup({
  types: {
    context: {} as OtelContext,
    events: {} as
      | { type: "boot" }
      | { type: "turnStart" }
      | { type: "span"; name: string }
      | { type: "exportFailed" }
      | { type: "turnEnd"; status: "ok" | "aborted" | "error" }
      | { type: "shutdown" }
  },
  guards: {
    optInActive: ({ context }) => context.featureEnabled && context.configOptIn,
    notOptedIn: ({ context }) => !(context.featureEnabled && context.configOptIn),
  },
  actions: {
    openTurnSpan: assign({ turnParentSpans: ({ context }) => context.turnParentSpans + 1 }),
    countSpan: assign({ exportedSpans: ({ context }) => context.exportedSpans + 1 }),
    degradeToFile: assign({
      exportFailures: ({ context }) => context.exportFailures + 1,
      blockedMainPath: () => false,
      filePipelineActive: () => true,
    }),
    recordTurnStatus: assign({ turnStatus: ({ event }) => (event as { status: string }).status }),
  },
});

/** 按配置生成机器：configOptIn=false 走默认不收集分支。 */
export function makeOtelMachine(configOptIn: boolean) {
  return setupMachine.createMachine({
    id: "otel-export",
    initial: "disabled",
    context: { ...initialContext, configOptIn },
    states: otelStates,
  });
}

const otelStates = {
    disabled: {
      on: {
        boot: [
          { guard: "optInActive", target: "initializing" },
          { guard: "notOptedIn" },
        ],
      },
    },
    initializing: {
      always: [{ target: "exporting" }],
    },
    exporting: {
      on: {
        turnStart: { actions: "openTurnSpan" },
        span: { actions: "countSpan" },
        exportFailed: { target: "degradedFileOnly", actions: "degradeToFile" },
        turnEnd: { actions: "recordTurnStatus" },
        shutdown: { target: "shutdown" },
      },
    },
    degradedFileOnly: {
      on: {
        turnStart: { actions: "openTurnSpan" },
        span: { actions: "countSpan" },
        turnEnd: { actions: "recordTurnStatus" },
        shutdown: { target: "shutdown" },
      },
    },
    shutdown: { type: "final" },
} as const;

/** 默认机器：未 opt-in。 */
export const machine = makeOtelMachine(false);

/** opt-in 机器：用于验证出口路径。 */
export const optInMachine = makeOtelMachine(true);

export const scenarioGroups = {
  "opt-in 后导出": [
    { name: "显式开启启动", events: [{ type: "boot" }], expectStates: ["exporting"] },
    { name: "turn 内导出 span", events: [{ type: "turnStart" }, { type: "span", name: "agent.generate" }], expectStates: ["exporting"], expectContext: { exportedSpans: 1, turnParentSpans: 1, hasSessionId: true } },
  ],
  "出口失败不阻塞": [
    { name: "显式开启启动", events: [{ type: "boot" }], expectStates: ["exporting"] },
    { name: "导出失败降级为仅文件", events: [{ type: "exportFailed" }], expectStates: ["degradedFileOnly"], expectContext: { blockedMainPath: false, filePipelineActive: true } },
    { name: "降级后主流程继续", events: [{ type: "span", name: "agent.tool" }], expectStates: ["degradedFileOnly"], expectContext: { blockedMainPath: false } },
  ],
  "turn 终态记录": [
    { name: "显式开启启动并跑 turn", events: [{ type: "boot" }, { type: "turnStart" }], expectStates: ["exporting"] },
    { name: "turn 中止收尾", events: [{ type: "turnEnd", status: "aborted" }], expectStates: ["exporting"], expectContext: { turnStatus: "aborted" } },
  ],
} satisfies Readonly<Record<string, readonly ScenarioStep[]>>;

if (import.meta.main) {
  const defaultResult = runScenarios(machine, [
    { name: "未 opt-in 启动", events: [{ type: "boot" }], expectStates: ["disabled"], expectContext: { exportedSpans: 0 } },
  ]);
  const results = { ...runScenarioGroups(optInMachine, scenarioGroups), "默认不收集": defaultResult };
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
