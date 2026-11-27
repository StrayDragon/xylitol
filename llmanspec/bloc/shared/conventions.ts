/**
 * 共享约定：所有业务状态机统一使用本文件的写法。
 *
 * 版本策略（v5 语法、v6 就绪）：
 * - 只用 `setup({ types: {...}, guards, actions, actors }).createMachine({...})`，
 *   这是 v5 与 v6 都接受的入口，v6 迁移不需要改调用点。
 * - 状态机一律用 `config` 对象字面量，不用 `createMachine(...)` 的旧式字符串 action。
 * - 不使用 v4 遗留：`strict`、`schema`、`predictableActionArguments`、`services`、`cond`、`target` 字符串数组写法。
 * - action 一律 `({ context, event }) => {}` 具名函数，便于 v6 的 `actions: {}` 注册表复用。
 * - 不用 `state.matches` 之外的隐式副作用；context 只经 `assign` 变更。
 * - 每个机器都导出 `machine`、`initialContext`、事件类型，并提供 `runScenarios` 验证入口。
 *
 * 锚点约定：每个机器的 JSDoc 必须写 `@spec llmanspec/specs/<cap>/<cap>.feature#@req:<id>`。
 */

import { createActor, type AnyActorLogic } from "xstate";

/** 一个状态机场景：发送事件序列后断言到达某状态。 */
export interface ScenarioStep {
  /** 步骤名，用于输出可读报告。 */
  name: string;
  /** 本步发送的事件（可为空表示只断言当前态）。 */
  events?: readonly unknown[];
  /** 断言当前状态路径匹配（任一即通过）。 */
  expectStates?: readonly string[];
  /** 断言 context 上的字段值（浅比较）。 */
  expectContext?: Readonly<Record<string, unknown>>;
  /** 断言机器输出或快照附加事实（可选）。 */
  expect?: (snapshot: unknown) => boolean | string;
}

/** 场景执行结果。 */
export interface ScenarioResult {
  passed: boolean;
  failures: string[];
}

/** 读取 actor 快照的当前状态值（字符串或嵌套对象）。 */
export function statePath(value: unknown): string {
  if (typeof value === "string") return value;
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    return entries.map(([k, v]) => `${k}.${statePath(v)}`).join("|");
  }
  return String(value);
}

/** 判断当前状态是否匹配给定路径（支持 `a.b` 与 `a.b.c`）。 */
export function matchesPath(current: unknown, wanted: string): boolean {
  if (typeof current === "string") {
    return current === wanted;
  }
  const parts = wanted.split(".");
  let cursor: unknown = current;
  for (const part of parts) {
    if (typeof cursor === "string") {
      return false;
    }
    if (cursor && typeof cursor === "object" && part in (cursor as Record<string, unknown>)) {
      cursor = (cursor as Record<string, unknown>)[part];
    } else {
      return false;
    }
  }
  return typeof cursor === "string";
}

/** 浅比较断言 context 字段。 */
function checkContext(actual: Record<string, unknown>, expected: Record<string, unknown>): string[] {
  const failures: string[] = [];
  for (const [key, want] of Object.entries(expected)) {
    const got = actual[key];
    if (JSON.stringify(got) !== JSON.stringify(want)) {
      failures.push(`context.${key} 期望 ${JSON.stringify(want)} 实得 ${JSON.stringify(got)}`);
    }
  }
  return failures;
}

/**
 * 同步驱动一个状态机跑完场景。
 *
 * 所有机器都不依赖异步 actor，因此 createActor + start 后逐个 send 即可同步观察。
 * 每组场景都必须从初始状态起跑：调用方按「组」传入完整事件序列，
 * 不得让上一组遗留状态影响下一组（v5 actor 不会自动重置）。
 */
export function runScenarios(
  logic: AnyActorLogic,
  steps: readonly ScenarioStep[],
): ScenarioResult {
  const actor = createActor(logic);
  actor.start();
  const failures: string[] = [];
  for (const step of steps) {
    for (const event of step.events ?? []) {
      actor.send(event as never);
    }
    const snapshot = actor.getSnapshot() as { value: unknown; context: Record<string, unknown> };
    if (step.expectStates && step.expectStates.length > 0) {
      const current = snapshot.value;
      const ok = step.expectStates.some((want) => matchesPath(current, want));
      if (!ok) {
        failures.push(`[${step.name}] 状态期望 ${step.expectStates.join(" 或 ")}，实得 ${statePath(current)}`);
      }
    }
    if (step.expectContext) {
      failures.push(...checkContext(snapshot.context, step.expectContext).map((m) => `[${step.name}] ${m}`));
    }
    if (step.expect) {
      const outcome = step.expect(snapshot);
      if (outcome !== true) {
        failures.push(`[${step.name}] ${typeof outcome === "string" ? outcome : "自定义断言未通过"}`);
      }
    }
  }
  actor.stop();
  return { passed: failures.length === 0, failures };
}

/**
 * 一次跑多组场景：每组都新建 actor，避免组间状态串味。
 *
 * 返回值按组名索引，供 verify 汇总与失败定位。
 */
export function runScenarioGroups(
  logic: AnyActorLogic,
  groups: Readonly<Record<string, readonly ScenarioStep[]>>,
): Readonly<Record<string, ScenarioResult>> {
  const out: Record<string, ScenarioResult> = {};
  for (const [group, steps] of Object.entries(groups)) {
    out[group] = runScenarios(logic, steps);
  }
  return out;
}

/** 把场景结果转成一行可读文本，供 verify 汇总。 */
export function describeResult(name: string, result: ScenarioResult): string {
  if (result.passed) return `通过 ${name}`;
  return [`失败 ${name}`, ...result.failures.map((f) => `  - ${f}`)].join("\n");
}

/** 业务状态机的元数据：用于 README 索引与覆盖核对。 */
export interface MachineMeta {
  /** 机器稳定名，kebab-case。 */
  name: string;
  /** 所属子包。 */
  pkg: string;
  /** 一句话业务职责。 */
  purpose: string;
  /** 引用的 spec 锚点，形如 `llmanspec/specs/agent-runtime/agent-runtime.feature#@req:r1027`。 */
  anchors: readonly string[];
  /** 对应的 Rust 实现落点（相对仓根），可为空。 */
  impl: readonly string[];
}

/** 声明机器元数据，集中一处便于脚本扫描。 */
export function defineMeta(meta: MachineMeta): MachineMeta {
  return meta;
}
