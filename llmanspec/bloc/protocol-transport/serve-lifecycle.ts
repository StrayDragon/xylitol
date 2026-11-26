/**
 * Serve 进程生命周期状态机（Serve Lifecycle）。
 *
 * 业务：serve 支持 persistent 与 last-client 两种生命周期模式（ServeLifecycle
 * 枚举，缺省 persistent）。last-client 模式下最后一条客户端连接断开进入空闲宽限
 * （DEFAULT_IDLE_GRACE 30s），宽限内重连续占，到期优雅停机；persistent 模式
 * 无人驻留也不退出。产品 TUI 确保本机 Host（tui-ensure-host）按版本决策：
 * 已有健康 Host 且版本相同直接 attach；旧于客户端须请求替换，新进程就绪后再 attach；
 * 新于客户端须非零退出且不得杀该进程；版本无法比较不得替换。
 * 确保等待超过 150ms 输出一行 Starting host 进度，已就绪不闪等待面；
 * 等待失败须清行并落可操作诊断。
 *
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1794（serve 托管 agent：Host profile 载体）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1796（产品 HTTP 与 WS 入口就绪后暴露）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1391（tui-ensure-host：版本比较替换/退出决策）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r3261（ensure-host 进度：150ms 阈值、失败清行）
 *
 * 实现落点：crates/xylitol-serve/src/lifecycle.rs（ServeLifecycle + DEFAULT_IDLE_GRACE）、
 * crates/xylitol-serve/src/listen.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 空闲宽限缺省值：与 crates/xylitol-serve/src/lifecycle.rs 的 DEFAULT_IDLE_GRACE（30s）对齐。 */
export const DEFAULT_IDLE_GRACE_MS = 30000;

/** ensure-host 进度阈值：等待超过 150ms 才输出一行 Starting host 进度（r3261）。 */
export const ENSURE_PROGRESS_THRESHOLD_MS = 150;

/** 语义化版本比较：a 新于 b 返回正数，旧于 b 返回负数，相等返回 0。 */
function compareVersions(a: string, b: string): number {
  const pa = a.split(".");
  const pb = b.split(".");
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const na = Number(pa[i] ?? "0");
    const nb = Number(pb[i] ?? "0");
    if (na !== nb) return na > nb ? 1 : -1;
  }
  return 0;
}

/** Serve 进程生命周期上下文：模式、客户端数、宽限与 ensure-host 决策事实。 */
export interface LifecycleContext {
  /** 生命周期模式：persistent / last-client（ServeLifecycle）。 */
  mode: "persistent" | "last-client";
  /** 当前客户端连接数。 */
  clients: number;
  /** 空闲宽限（毫秒）。 */
  idleGraceMs: number;
  /** ensure 等待是否输出过 Starting host 进度。 */
  progressShown: boolean;
  /** 失败时是否已清行。 */
  lineCleared: boolean;
  /** 最近一次可操作诊断。 */
  diagnostic: string | null;
  /** 替换旧版 Host 的次数（版本旧于客户端）。 */
  replacedOldHosts: number;
  /** 是否向 Host 发送了退出请求以外的杀伤（r1391：恒 false）。 */
  hostKilled: boolean;
  /** 非零退出记录（新版 Host 拒绝 / 确保失败）。 */
  exitCode: number | null;
}

export const initialContext: LifecycleContext = {
  mode: "persistent",
  clients: 0,
  idleGraceMs: DEFAULT_IDLE_GRACE_MS,
  progressShown: false,
  lineCleared: false,
  diagnostic: null,
  replacedOldHosts: 0,
  hostKilled: false,
  exitCode: null,
};

export const meta = defineMeta({
  name: "serve-lifecycle",
  pkg: "protocol-transport",
  purpose: "Serve 进程生命周期：persistent/last-client、空闲宽限、ensure-host 版本决策",
  anchors: [
    "llmanspec/specs/server-core/server-core.feature#@req:r1794",
    "llmanspec/specs/server-core/server-core.feature#@req:r1796",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1391",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r3261",
  ],
  impl: [
    "crates/xylitol-serve/src/lifecycle.rs",
    "crates/xylitol-serve/src/listen.rs",
  ],
});

export const setupMachine = setup({
  types: {
    context: {} as LifecycleContext,
    events: {} as
      | { type: "configureLifecycle"; mode: "persistent" | "last-client" }
      | { type: "ensureRequested"; hostListening: boolean; waitMs: number }
      | { type: "hostReady"; waitMs: number }
      | { type: "ensureTimeout" }
      | { type: "attachRequested"; hostVersion: string | null; clientVersion: string }
      | { type: "newHostReady" }
      | { type: "clientAttached" }
      | { type: "lastClientDetached" }
      | { type: "idleGraceExpired" }
      | { type: "shutdownComplete" },
  },
  guards: {
    hostAlreadyListening: ({ event }) =>
      event.type === "ensureRequested" && event.hostListening,
    hostVersionNewer: ({ event }) =>
      event.type === "attachRequested" &&
      event.hostVersion !== null &&
      compareVersions(event.hostVersion, event.clientVersion) > 0,
    hostVersionOlder: ({ event }) =>
      event.type === "attachRequested" &&
      event.hostVersion !== null &&
      compareVersions(event.hostVersion, event.clientVersion) < 0,
    hostVersionUnknown: ({ event }) =>
      event.type === "attachRequested" && event.hostVersion === null,
    lastClientMode: ({ context }) => context.mode === "last-client",
    persistentMode: ({ context }) => context.mode === "persistent",
  },
  actions: {
    setMode: assign({
      mode: ({ context, event }) =>
        event.type === "configureLifecycle" ? event.mode : context.mode,
    }),
    showEnsureProgress: assign({
      progressShown: ({ event }) =>
        event.type === "ensureRequested" && event.waitMs >= ENSURE_PROGRESS_THRESHOLD_MS,
    }),
    settleHostReady: assign({
      progressShown: ({ event }) =>
        event.type === "hostReady" && event.waitMs >= ENSURE_PROGRESS_THRESHOLD_MS,
    }),
    failEnsure: assign({
      lineCleared: () => true,
      diagnostic: () => "ensure-host-failed-use-xylitol-serve",
      exitCode: () => 1,
    }),
    replaceOldHost: assign({
      replacedOldHosts: ({ context }) => context.replacedOldHosts + 1,
    }),
    refuseNewerHost: assign({
      exitCode: () => 1,
      hostKilled: () => false,
      diagnostic: () => "host-newer-nonzero-exit-no-kill",
    }),
    noteUncomparable: assign({
      diagnostic: () => "version-uncomparable-no-replace",
    }),
    attachClient: assign({
      clients: ({ context }) => context.clients + 1,
    }),
    releaseClients: assign({
      clients: () => 0,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "serve-lifecycle",
  initial: "idle",
  context: initialContext,
  states: {
    idle: {
      on: {
        configureLifecycle: { actions: "setMode" },
        ensureRequested: [
          { guard: "hostAlreadyListening", target: "serving", actions: "attachClient" },
          { target: "ensuring", actions: "showEnsureProgress" },
        ],
      },
    },
    ensuring: {
      on: {
        hostReady: { target: "serving", actions: "settleHostReady" },
        ensureTimeout: { target: "ensureFailed", actions: "failEnsure" },
      },
    },
    serving: {
      on: {
        attachRequested: [
          { guard: "hostVersionNewer", target: "attachRefused", actions: "refuseNewerHost" },
          { guard: "hostVersionOlder", target: "replacing", actions: "replaceOldHost" },
          { guard: "hostVersionUnknown", actions: "noteUncomparable" },
          { actions: "attachClient" },
        ],
        clientAttached: { actions: "attachClient" },
        lastClientDetached: [
          { guard: "lastClientMode", target: "idleGrace", actions: "releaseClients" },
          { guard: "persistentMode", actions: "releaseClients" },
        ],
      },
    },
    idleGrace: {
      on: {
        clientAttached: { target: "serving", actions: "attachClient" },
        idleGraceExpired: { target: "stopping" },
      },
    },
    replacing: {
      on: {
        newHostReady: { target: "serving", actions: "attachClient" },
      },
    },
    stopping: {
      on: {
        shutdownComplete: { target: "stopped" },
      },
    },
    attachRefused: { type: "final" },
    ensureFailed: { type: "final" },
    stopped: { type: "final" },
  },
});

export const scenarioGroups = {
  "last-client 空闲宽限到期停机": [
    { name: "启动时选择 last-client 模式", events: [{ type: "configureLifecycle", mode: "last-client" }], expectStates: ["idle"], expectContext: { mode: "last-client", idleGraceMs: 30000 } },
    { name: "确保等待超 150ms 输出进度", events: [{ type: "ensureRequested", hostListening: false, waitMs: 200 }], expectStates: ["ensuring"], expectContext: { progressShown: true } },
    { name: "Host 就绪并 attach", events: [{ type: "hostReady", waitMs: 200 }], expectStates: ["serving"], expectContext: { progressShown: true } },
    { name: "客户端接入", events: [{ type: "clientAttached" }], expectStates: ["serving"], expectContext: { clients: 1 } },
    { name: "最后一条连接断开进入空闲宽限", events: [{ type: "lastClientDetached" }], expectStates: ["idleGrace"], expectContext: { clients: 0 } },
    { name: "宽限到期进入停机", events: [{ type: "idleGraceExpired" }], expectStates: ["stopping"] },
    { name: "停机完成", events: [{ type: "shutdownComplete" }], expectStates: ["stopped"] },
  ],
  "persistent 无人驻留不退出": [
    { name: "确保等待不足 150ms 不输出进度", events: [{ type: "ensureRequested", hostListening: false, waitMs: 100 }], expectStates: ["ensuring"], expectContext: { progressShown: false } },
    { name: "Host 就绪仍不闪等待面", events: [{ type: "hostReady", waitMs: 100 }], expectStates: ["serving"], expectContext: { progressShown: false } },
    { name: "客户端接入", events: [{ type: "clientAttached" }], expectStates: ["serving"], expectContext: { clients: 1 } },
    { name: "断开后 persistent 模式保持驻留", events: [{ type: "lastClientDetached" }], expectStates: ["serving"], expectContext: { clients: 0, mode: "persistent" } },
  ],
  "宽限内重连续占": [
    { name: "选择 last-client 模式", events: [{ type: "configureLifecycle", mode: "last-client" }], expectStates: ["idle"], expectContext: { mode: "last-client" } },
    { name: "确保并就绪", events: [{ type: "ensureRequested", hostListening: false, waitMs: 200 }, { type: "hostReady", waitMs: 200 }], expectStates: ["serving"] },
    { name: "客户端接入", events: [{ type: "clientAttached" }], expectStates: ["serving"], expectContext: { clients: 1 } },
    { name: "断开进入宽限", events: [{ type: "lastClientDetached" }], expectStates: ["idleGrace"], expectContext: { clients: 0 } },
    { name: "宽限内重连续占未停机", events: [{ type: "clientAttached" }], expectStates: ["serving"], expectContext: { clients: 1, exitCode: null } },
  ],
  "新版 Host 非零退出且不杀": [
    { name: "确保并就绪", events: [{ type: "ensureRequested", hostListening: false, waitMs: 100 }, { type: "hostReady", waitMs: 100 }], expectStates: ["serving"] },
    { name: "目标 Host 版本更新于客户端：非零退出且不杀", events: [{ type: "attachRequested", hostVersion: "2.0.0", clientVersion: "1.0.0" }], expectStates: ["attachRefused"], expectContext: { exitCode: 1, hostKilled: false, replacedOldHosts: 0 } },
  ],
  "旧版 Host 请求替换": [
    { name: "确保并就绪", events: [{ type: "ensureRequested", hostListening: false, waitMs: 200 }, { type: "hostReady", waitMs: 200 }], expectStates: ["serving"] },
    { name: "目标 Host 版本旧于客户端：请求替换", events: [{ type: "attachRequested", hostVersion: "0.9.0", clientVersion: "1.0.0" }], expectStates: ["replacing"], expectContext: { replacedOldHosts: 1, hostKilled: false } },
    { name: "新 Host 就绪后 attach", events: [{ type: "newHostReady" }], expectStates: ["serving"], expectContext: { clients: 1 } },
  ],
  "版本无法比较不得替换": [
    { name: "确保并就绪（恰 150ms 输出进度）", events: [{ type: "ensureRequested", hostListening: false, waitMs: 150 }, { type: "hostReady", waitMs: 150 }], expectStates: ["serving"], expectContext: { progressShown: true } },
    { name: "版本无法比较：不替换只落诊断", events: [{ type: "attachRequested", hostVersion: null, clientVersion: "1.0.0" }], expectStates: ["serving"], expectContext: { diagnostic: "version-uncomparable-no-replace", replacedOldHosts: 0 } },
  ],
  "确保超时清行落诊断": [
    { name: "确保等待输出进度", events: [{ type: "ensureRequested", hostListening: false, waitMs: 300 }], expectStates: ["ensuring"], expectContext: { progressShown: true } },
    { name: "确保失败：清行并落可操作诊断后非零退出", events: [{ type: "ensureTimeout" }], expectStates: ["ensureFailed"], expectContext: { lineCleared: true, diagnostic: "ensure-host-failed-use-xylitol-serve", exitCode: 1 } },
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