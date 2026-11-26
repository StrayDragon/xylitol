/**
 * TUI 宿主生命周期状态机（TUI Host Lifecycle）。
 *
 * 业务：host 驱动引擎、终端进入/恢复、尺寸降级、重载编排、单 mux 循环。
 * 终端恢复必须在退出路径上执行；尺寸不足走降级而非崩溃。
 *
 * @spec llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1240（host-driven-engine）
 * @spec llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1248（terminal-restore）
 * @spec llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1270（resize-and-min-size）
 * @spec llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1279（single-mux-loop）
 * @spec llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1249（reload-orchestrates-foundations）
 * @spec llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1257（reload-in-progress-ux）
 * @spec llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1253（tick-gated-local-paint）
 *
 * 实现落点：crates/xylitol-app-tui/src/（host 驱动与终端生命周期）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type HostPhase = "booting" | "attached" | "running" | "reloading" | "restoring" | "exited" | "degraded";

export interface HostContext {
  /** 是否已 attach 本机 Host。 */
  hostAttached: boolean;
  /** 终端是否处于 raw 模式。 */
  rawMode: boolean;
  /** 终端是否已恢复。 */
  terminalRestored: boolean;
  /** 当前列宽。 */
  cols: number;
  /** 最小列宽。 */
  minCols: number;
  /** 重载次数。 */
  reloadCount: number;
  /** 是否在重载期间显示状态。 */
  reloadUxShown: boolean;
  /** 本 tick 内重绘次数。 */
  paintsThisTick: number;
}

export const initialContext: HostContext = {
  hostAttached: false,
  rawMode: false,
  terminalRestored: false,
  cols: 120,
  minCols: 40,
  reloadCount: 0,
  reloadUxShown: false,
  paintsThisTick: 0,
};

export const meta = defineMeta({
  name: "tui-host-lifecycle",
  pkg: "app-surface",
  purpose: "TUI 宿主：attach、raw 终端、尺寸降级、重载、退出恢复",
  anchors: [
    "llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1240",
    "llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1248",
    "llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1270",
    "llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1279",
    "llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1249",
    "llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1257",
    "llmanspec/specs/app-tui-host/app-tui-host.feature#@req:r1253",
  ],
  impl: ["crates/xylitol-app-tui/src/"],
});

export const setupMachine = setup({
  types: {
    context: {} as HostContext,
    events: {} as
      | { type: "attachOk" }
      | { type: "attachFailed" }
      | { type: "enterRaw" }
      | { type: "resize"; cols: number }
      | { type: "reload" }
      | { type: "reloadDone" }
      | { type: "tick" }
      | { type: "quit" }
      | { type: "restoreDone" }
  },
  guards: {
    wideEnough: ({ context, event }) => (event as { cols: number }).cols >= context.minCols,
    tooNarrow: ({ context, event }) => (event as { cols: number }).cols < context.minCols,
  },
  actions: {
    markAttached: assign({ hostAttached: () => true }),
    markRaw: assign({ rawMode: () => true }),
    applyWidth: assign({ cols: ({ event }) => (event as { cols: number }).cols }),
    markReloading: assign({ reloadUxShown: () => true }),
    countReload: assign({ reloadCount: ({ context }) => context.reloadCount + 1 }),
    clearReloadUx: assign({ reloadUxShown: () => false }),
    resetPaint: assign({ paintsThisTick: () => 0 }),
    restoreTerminal: assign({ terminalRestored: () => true, rawMode: () => false }),
  },
});

export const machine = setupMachine.createMachine({
  id: "tui-host-lifecycle",
  initial: "booting",
  context: initialContext,
  states: {
    booting: {
      on: {
        attachOk: { target: "attached", actions: "markAttached" },
        attachFailed: { target: "degraded" },
      },
    },
    attached: {
      on: {
        enterRaw: { target: "running", actions: "markRaw" },
      },
    },
    running: {
      on: {
        resize: [
          { guard: "wideEnough", actions: "applyWidth" },
          { guard: "tooNarrow", target: "degraded", actions: "applyWidth" },
        ],
        reload: { target: "reloading", actions: ["markReloading", "countReload"] },
        tick: { actions: "resetPaint" },
        quit: { target: "restoring" },
      },
    },
    reloading: {
      on: {
        reloadDone: { target: "running", actions: "clearReloadUx" },
        quit: { target: "restoring" },
      },
    },
    degraded: {
      on: {
        resize: [
          { guard: "wideEnough", target: "running", actions: "applyWidth" },
          { guard: "tooNarrow", actions: "applyWidth" },
        ],
        quit: { target: "restoring" },
      },
    },
    restoring: {
      on: {
        restoreDone: { target: "exited", actions: "restoreTerminal" },
      },
    },
    exited: { type: "final" },
  },
});

export const scenarioGroups = {
  "正常启动到运行": [
    { name: "attach 成功", events: [{ type: "attachOk" }], expectStates: ["attached"], expectContext: { hostAttached: true } },
    { name: "进入 raw 终端", events: [{ type: "enterRaw" }], expectStates: ["running"], expectContext: { rawMode: true } },
  ],
  "attach 失败降级": [
    { name: "attach 失败", events: [{ type: "attachFailed" }], expectStates: ["degraded"], expectContext: { hostAttached: false } },
  ],
  "尺寸不足降级": [
    { name: "启动到运行", events: [{ type: "attachOk" }, { type: "enterRaw" }], expectStates: ["running"] },
    { name: "缩窄到低于最小列宽", events: [{ type: "resize", cols: 20 }], expectStates: ["degraded"], expectContext: { cols: 20 } },
    { name: "恢复宽度回运行", events: [{ type: "resize", cols: 120 }], expectStates: ["running"], expectContext: { cols: 120 } },
  ],
  "重载期间显示状态": [
    { name: "启动到运行", events: [{ type: "attachOk" }, { type: "enterRaw" }], expectStates: ["running"] },
    { name: "触发重载", events: [{ type: "reload" }], expectStates: ["reloading"], expectContext: { reloadCount: 1, reloadUxShown: true } },
    { name: "重载完成", events: [{ type: "reloadDone" }], expectStates: ["running"], expectContext: { reloadUxShown: false } },
  ],
  "退出恢复终端": [
    { name: "启动到运行", events: [{ type: "attachOk" }, { type: "enterRaw" }], expectStates: ["running"] },
    { name: "请求退出", events: [{ type: "quit" }], expectStates: ["restoring"] },
    { name: "终端恢复完成", events: [{ type: "restoreDone" }], expectStates: ["exited"], expectContext: { terminalRestored: true, rawMode: false } },
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