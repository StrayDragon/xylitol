/**
 * Serve 启动就绪窗口状态机（Serve Readiness）。
 *
 * 业务：Host 监听器提供就绪状态机（starting / ready / stopping / failed）。
 * 绑定端口后装配完成前为 starting：healthz 返回 503 并携带 starting 语义与
 * retry-after，其余 unary 与 WS 升级统一 503 拒绝且不得半执行；
 * 装配完成后 healthz 翻转 200（应答 body 携带本进程 pid 与 version）。
 * 配置端口被占用（EADDRINUSE）须失败返回，不得 port+1 重试，
 * 不得用整机锁文件作单实例闸；--port 0 由 OS 分配并打印实际端口。
 * 装配失败进入 failed 不可重试语义；ready 后原子写注册文件（0600，含 url/pid/version），
 * 周期复读发现字段不全等须自行优雅退出；优雅停机进入 stopping 并删除注册文件。
 *
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1786（启动就绪窗口三态语义）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1787（注册文件发现契约与自检退出）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1797（绑定占用：EADDRINUSE 失败，不得整机锁）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1780（healthz 探针启动后 200）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1801（禁止端口重试）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1393（serve 子命令：--port 0 由 OS 分配并打印实际端口）
 *
 * 实现落点：crates/xylitol-serve/src/listen.rs、crates/xylitol-serve/src/registration.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 默认监听地址端口（r1797：默认 127.0.0.1:18790）。 */
export const DEFAULT_SERVE_PORT = 18790;

/** 就绪窗口上下文：绑定事实、注册文件事实与最近一次探针应答。 */
export interface ReadinessContext {
  /** 配置端口（0 = 交由 OS 分配）。 */
  requestedPort: number;
  /** 实际绑定端口；未绑定为 null。 */
  boundPort: number | null;
  /** 绑定尝试次数（占用时不得重试，恒为 1）。 */
  bindAttempts: number;
  /** 装配完成的进程 pid（healthz body 携带）。 */
  hostPid: number | null;
  /** 装配完成的版本（healthz body 携带）。 */
  hostVersion: string | null;
  /** 注册文件是否已原子写入（ready 时写）。 */
  registrationWritten: boolean;
  /** 注册文件是否已删除（优雅退出时删）。 */
  registrationDeleted: boolean;
  /** 最近一次探针路径。 */
  lastProbePath: "healthz" | "unary" | null;
  /** 最近一次探针 HTTP 状态。 */
  lastProbeStatus: number | null;
  /** 最近一次探针语义（starting / ready / stopping / failed）。 */
  lastProbeSemantic: string;
  /** 最近一次探针 retry-after（秒）。 */
  lastProbeRetryAfter: number;
  /** 自检周期发现注册漂移的次数。 */
  driftDetections: number;
  /** 整机锁文件写入次数（r1797/r1800：恒为 0）。 */
  lockFileWrites: number;
}

export const initialContext: ReadinessContext = {
  requestedPort: DEFAULT_SERVE_PORT,
  boundPort: null,
  bindAttempts: 0,
  hostPid: null,
  hostVersion: null,
  registrationWritten: false,
  registrationDeleted: false,
  lastProbePath: null,
  lastProbeStatus: null,
  lastProbeSemantic: "",
  lastProbeRetryAfter: 0,
  driftDetections: 0,
  lockFileWrites: 0,
};

export const meta = defineMeta({
  name: "serve-readiness",
  pkg: "protocol-transport",
  purpose: "Serve 启动就绪窗口：starting/ready/stopping/failed、绑定占用禁重试、注册文件自检",
  anchors: [
    "llmanspec/specs/server-core/server-core.feature#@req:r1786",
    "llmanspec/specs/server-core/server-core.feature#@req:r1787",
    "llmanspec/specs/server-core/server-core.feature#@req:r1797",
    "llmanspec/specs/server-core/server-core.feature#@req:r1780",
    "llmanspec/specs/server-core/server-core.feature#@req:r1801",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1393",
  ],
  impl: [
    "crates/xylitol-serve/src/listen.rs",
    "crates/xylitol-serve/src/registration.rs",
  ],
});

export const setupMachine = setup({
  types: {
    context: {} as ReadinessContext,
    events: {} as
      | { type: "bindOk"; requestedPort: number; assignedPort: number }
      | { type: "bindOccupied" }
      | { type: "assemblyComplete"; pid: number; version: string }
      | { type: "assemblyFail" }
      | { type: "probeHealthz" }
      | { type: "probeUnary" }
      | { type: "registrationDrift" }
      | { type: "shutdownRequested" }
      | { type: "shutdownComplete" },
  },
  actions: {
    recordBind: assign({
      requestedPort: ({ context, event }) =>
        event.type === "bindOk" ? event.requestedPort : context.requestedPort,
      boundPort: ({ event }) => (event.type === "bindOk" ? event.assignedPort : null),
      bindAttempts: ({ context }) => context.bindAttempts + 1,
    }),
    recordBindOccupied: assign({
      bindAttempts: ({ context }) => context.bindAttempts + 1,
    }),
    reportStartingHealthz: assign({
      lastProbePath: () => "healthz" as const,
      lastProbeStatus: () => 503,
      lastProbeSemantic: () => "starting",
      lastProbeRetryAfter: () => 1,
    }),
    reportStartingUnary: assign({
      lastProbePath: () => "unary" as const,
      lastProbeStatus: () => 503,
      lastProbeSemantic: () => "starting",
      lastProbeRetryAfter: () => 1,
    }),
    completeAssembly: assign({
      hostPid: ({ event }) => (event.type === "assemblyComplete" ? event.pid : null),
      hostVersion: ({ event }) => (event.type === "assemblyComplete" ? event.version : null),
      registrationWritten: () => true,
    }),
    reportReadyHealthz: assign({
      lastProbePath: () => "healthz" as const,
      lastProbeStatus: () => 200,
      lastProbeSemantic: () => "ready",
      lastProbeRetryAfter: () => 0,
    }),
    dispatchReadyUnary: assign({
      lastProbePath: () => "unary" as const,
      lastProbeStatus: () => 200,
      lastProbeSemantic: () => "ready",
      lastProbeRetryAfter: () => 0,
    }),
    reportFailedProbe: assign({
      lastProbePath: () => "healthz" as const,
      lastProbeStatus: () => 503,
      lastProbeSemantic: () => "failed",
      lastProbeRetryAfter: () => 0,
    }),
    reportStoppingProbe: assign({
      lastProbePath: () => "healthz" as const,
      lastProbeStatus: () => 503,
      lastProbeSemantic: () => "stopping",
      lastProbeRetryAfter: () => 0,
    }),
    deleteRegistration: assign({
      registrationDeleted: () => true,
    }),
    detectDrift: assign({
      driftDetections: ({ context }) => context.driftDetections + 1,
      registrationDeleted: () => true,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "serve-readiness",
  initial: "binding",
  context: initialContext,
  states: {
    binding: {
      on: {
        bindOk: { target: "starting", actions: "recordBind" },
        bindOccupied: { target: "failedBind", actions: "recordBindOccupied" },
      },
    },
    starting: {
      on: {
        probeHealthz: { actions: "reportStartingHealthz" },
        probeUnary: { actions: "reportStartingUnary" },
        assemblyComplete: { target: "ready", actions: "completeAssembly" },
        assemblyFail: { target: "failed" },
      },
    },
    ready: {
      on: {
        probeHealthz: { actions: "reportReadyHealthz" },
        probeUnary: { actions: "dispatchReadyUnary" },
        registrationDrift: { target: "stopping", actions: "detectDrift" },
        shutdownRequested: { target: "stopping", actions: "deleteRegistration" },
      },
    },
    stopping: {
      on: {
        probeHealthz: { actions: "reportStoppingProbe" },
        shutdownComplete: { target: "stopped" },
      },
    },
    failed: {
      on: {
        probeHealthz: { actions: "reportFailedProbe" },
      },
    },
    failedBind: { type: "final" },
    stopped: { type: "final" },
  },
});

export const scenarioGroups = {
  "启动窗口翻转与注册写入": [
    { name: "绑定成功（--port 0 由 OS 分配）", events: [{ type: "bindOk", requestedPort: 0, assignedPort: 51234 }], expectStates: ["starting"], expectContext: { bindAttempts: 1, boundPort: 51234, requestedPort: 0 } },
    { name: "starting 窗口 healthz 503 带 retry-after", events: [{ type: "probeHealthz" }], expectStates: ["starting"], expectContext: { lastProbePath: "healthz", lastProbeStatus: 503, lastProbeSemantic: "starting", lastProbeRetryAfter: 1 } },
    { name: "starting 窗口 unary 同语义 503 拒绝", events: [{ type: "probeUnary" }], expectStates: ["starting"], expectContext: { lastProbePath: "unary", lastProbeStatus: 503, lastProbeSemantic: "starting" } },
    { name: "装配完成写注册文件并翻转 ready", events: [{ type: "assemblyComplete", pid: 4242, version: "0.9.3" }], expectStates: ["ready"], expectContext: { registrationWritten: true, hostPid: 4242, hostVersion: "0.9.3" } },
    { name: "ready 后 healthz 200", events: [{ type: "probeHealthz" }], expectStates: ["ready"], expectContext: { lastProbePath: "healthz", lastProbeStatus: 200, lastProbeSemantic: "ready", lastProbeRetryAfter: 0 } },
  ],
  "绑定占用禁止重试": [
    { name: "端口占用立即失败且不写整机锁", events: [{ type: "bindOccupied" }], expectStates: ["failedBind"], expectContext: { bindAttempts: 1, lockFileWrites: 0, boundPort: null } },
    { name: "失败后不再绑定（不得 port+1）", events: [{ type: "bindOk", requestedPort: 18791, assignedPort: 18791 }], expectStates: ["failedBind"], expectContext: { bindAttempts: 1, boundPort: null } },
  ],
  "装配失败不可重试": [
    { name: "绑定进入 starting", events: [{ type: "bindOk", requestedPort: 18790, assignedPort: 18790 }], expectStates: ["starting"] },
    { name: "装配失败进入 failed", events: [{ type: "assemblyFail" }], expectStates: ["failed"] },
    { name: "failed 携带不可重试语义", events: [{ type: "probeHealthz" }], expectStates: ["failed"], expectContext: { lastProbeStatus: 503, lastProbeSemantic: "failed", lastProbeRetryAfter: 0 } },
    { name: "failed 不再受理装配完成", events: [{ type: "assemblyComplete", pid: 1, version: "0.9.3" }], expectStates: ["failed"], expectContext: { registrationWritten: false } },
  ],
  "注册漂移自检退出": [
    { name: "就绪并写注册文件", events: [{ type: "bindOk", requestedPort: 18790, assignedPort: 18790 }, { type: "assemblyComplete", pid: 100, version: "1.0.0" }], expectStates: ["ready"], expectContext: { registrationWritten: true } },
    { name: "自检发现字段不全等即优雅退出", events: [{ type: "registrationDrift" }], expectStates: ["stopping"], expectContext: { driftDetections: 1, registrationDeleted: true } },
    { name: "停机完成", events: [{ type: "shutdownComplete" }], expectStates: ["stopped"] },
  ],
  "优雅停机与 stopping 探针": [
    { name: "就绪", events: [{ type: "bindOk", requestedPort: 18790, assignedPort: 18790 }, { type: "assemblyComplete", pid: 100, version: "1.0.0" }], expectStates: ["ready"] },
    { name: "优雅停机删除注册文件", events: [{ type: "shutdownRequested" }], expectStates: ["stopping"], expectContext: { registrationDeleted: true } },
    { name: "stopping 窗口 healthz 503", events: [{ type: "probeHealthz" }], expectStates: ["stopping"], expectContext: { lastProbeStatus: 503, lastProbeSemantic: "stopping" } },
    { name: "停机完成后探针不再受理", events: [{ type: "shutdownComplete" }, { type: "probeHealthz" }], expectStates: ["stopped"], expectContext: { lastProbeSemantic: "stopping" } },
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
