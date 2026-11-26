/**
 * 生成每个状态机的 mermaid 状态图与索引 README。
 *
 * 用法：cd llmanspec/bloc && bun run scripts/gen-diagrams.ts
 *
 * 数据来源：直接解析各 .ts 文件里的 machine 配置（states 顶层键 + 转移 target），
 * 保证图与代码同源，不手写、不漂移。
 */

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");

interface Transition { from: string; to: string; event: string; guard?: string }
interface MachineDoc {
  pkg: string;
  file: string;
  name: string;
  purpose: string;
  anchors: string[];
  impl: string[];
  states: string[];
  initial: string;
  transitions: Transition[];
}

/**
 * 取状态表正文。两种写法都要支持：
 * - 内联：createMachine({ ..., states: { ... } })
 * - 抽出的常量：const xxxStates = { ... } 然后 states: xxxStates
 * 后者优先取「含 on:/always: 的那个对象」，避免误取其它配置对象。
 */
function pickStateBlock(source: string): string {
  const inline = source.split(/states: \{/);
  if (inline.length > 1) return inline.slice(1).join("states: {");
  const decls = [...source.matchAll(/const\s+[a-zA-Z0-9_]*[Ss]tates\s*=\s*\{/g)];
  const last = decls.at(-1);
  if (last) return source.slice(last.index! + last[0].length);
  return "";
}

function parseFile(pkg: string, entry: string): MachineDoc | null {
  const source = readFileSync(join(ROOT, pkg, entry), "utf8");
  const nameMatch = source.match(/name: "([^"]+)"/);
  const purposeMatch = source.match(/purpose: "([^"]+)"/);
  const initialMatch = source.match(/initial: "([^"]+)"/);
  if (!nameMatch || !purposeMatch || !initialMatch) return null;
  const anchors = [...source.matchAll(/@spec\s+(llmanspec\/specs\/[^\s（]+)/g)].map((m) => m[1]!);
  const impl = [...source.matchAll(/实现落点：([^\n（]+)/g)].map((m) => m[1]!.replace(/\s*\*\/?\s*$/, "").trim());
  const stateBlock = pickStateBlock(source);
  const states: string[] = [];
  for (const m of stateBlock.matchAll(/^    ([a-zA-Z][a-zA-Z0-9]*): \{/gm)) states.push(m[1]!);

  const transitions: Transition[] = [];
  const lines = stateBlock.split("\n");
  let current: string | null = null;
  let pendingEvent: string | null = null;
  for (const line of lines) {
    const stateStart = line.match(/^    ([a-zA-Z][a-zA-Z0-9]*): \{/);
    if (stateStart) { current = stateStart[1]!; pendingEvent = null; continue; }
    if (current === null) continue;
    const alwaysArray = line.match(/^\s{4,}always: \[/);
    if (alwaysArray) { pendingEvent = "always"; continue; }
    const evArray = line.match(/^\s{4,}([a-zA-Z][a-zA-Z0-9]*): \[/);
    if (evArray) { pendingEvent = evArray[1]!; continue; }
    const evSingle = line.match(/^\s{4,}([a-zA-Z][a-zA-Z0-9]*): \{/);
    if (evSingle) { pendingEvent = evSingle[1]!; }
    const target = line.match(/target: "([a-zA-Z][a-zA-Z0-9]*)"/);
    if (target && pendingEvent) {
      const guard = line.match(/guard: "([^"]+)"/);
      transitions.push({ from: current, to: target[1]!, event: pendingEvent, guard: guard ? guard[1] : undefined });
    }
  }
  return { pkg, file: pkg + "/" + entry, name: nameMatch[1]!, purpose: purposeMatch[1]!, anchors, impl, states, initial: initialMatch[1]!, transitions };
}

const TERMINAL = new Set(["finished", "failed", "done", "exited", "returned", "aborted", "exhausted", "shutdown", "formatRejected", "closed", "cancelled", "answered", "skipped", "stopped", "settled"]);

function mermaidOf(doc: MachineDoc): string {
  const out: string[] = ["```mermaid", "stateDiagram-v2"];
  out.push("    [*] --> " + doc.initial);
  const seen = new Set<string>();
  for (const t of doc.transitions) {
    const evName = t.event === "always" ? "ε（无条件）" : t.event;
    const label = t.guard ? evName + " [" + t.guard + "]" : evName;
    const key = t.from + "|" + t.to + "|" + label;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push("    " + t.from + " --> " + t.to + ": " + label);
  }
  for (const s of doc.states) if (TERMINAL.has(s)) out.push("    " + s + " --> [*]");
  out.push("```");
  return out.join("\n");
}

const SKIP = new Set(["node_modules", "shared", "scripts"]);
const pkgDirs = readdirSync(ROOT, { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name)
  .filter((n) => !SKIP.has(n))
  .sort();

const docs: MachineDoc[] = [];
for (const pkg of pkgDirs) {
  for (const entry of readdirSync(join(ROOT, pkg)).sort()) {
    if (!entry.endsWith(".ts")) continue;
    const doc = parseFile(pkg, entry);
    if (doc) docs.push(doc);
  }
}

for (const doc of docs) {
  const body = [
    "# " + doc.name,
    "",
    "> " + doc.purpose,
    "",
    "子包：`" + doc.pkg + "`　代码：`" + doc.file + "`",
    "",
    "## 状态图",
    "",
    mermaidOf(doc),
    "",
    "## 状态清单",
    "",
    ...doc.states.map((s) => "- `" + s + "`"),
    "",
    "## spec 锚点",
    "",
    ...doc.anchors.map((a) => "- `" + a + "`"),
    "",
    "## 实现落点",
    "",
    ...doc.impl.map((i) => "- `" + i + "`"),
    "",
  ];
  writeFileSync(join(ROOT, doc.file.replace(/\.ts$/, ".md")), body.join("\n"));
}

const uniqueAnchors = new Set(docs.flatMap((d) => d.anchors));
const uniqueReqs = new Set(docs.flatMap((d) => d.anchors.map((a) => a.split("#")[1]!)));
const head = [
  "# xylitol 业务状态机梳理",
  "",
  "围绕 `llmanspec/specs/` 与对应 Rust 实现，把每个业务模块写成可运行的 xstate 状态机。",
  "每个模块同时给出：TypeScript 状态机（v5 语法、v6 就绪写法）、mermaid 状态图、bun 可执行验证场景。",
  "",
  "## 约定",
  "",
  "| 项 | 内容 |",
  "|---|---|",
  "| 入口 | `setup({ types, guards, actions }).createMachine({...})`，v5 与 v6 都接受 |",
  "| 禁止 | v4 遗留：`strict` / `schema` / `services` / `cond` / `predictableActionArguments` |",
  "| guard 求值 | 按动作执行前的 context 求值，需要预判的字段在 guard 内自行推算 |",
  "| 保留名 | 状态名避开 xstate v5 保留名（如 `held`），一律用语义名 |",
  "| 场景 | 每组用 `runScenarioGroups` 新建 actor，组间不串状态 |",
  "| 锚点 | 每文件 JSDoc 写 `@spec llmanspec/specs/<cap>/<cap>.feature#@req:<id>` |",
  "",
  "## 校验",
  "",
  "```bash",
  "cd llmanspec/bloc",
  "bun install",
  "bun run scripts/verify.ts          # 全量跑通 + 锚点核对",
  "bun run scripts/gen-diagrams.ts    # 重新生成 mermaid 与索引",
  "bunx tsc --noEmit                  # 类型检查",
  "```",
  "",
  "## 覆盖概览",
  "",
  "共 " + docs.length + " 个状态机，" + pkgDirs.length + " 个子包，引用 " + uniqueAnchors.size + " 个 spec 文件、" + uniqueReqs.size + " 个 @req 锚点。",
  "",
];
const perPkg: string[] = [];
for (const pkg of pkgDirs) {
  const inPkg = docs.filter((d) => d.pkg === pkg);
  perPkg.push("### " + pkg + "（" + inPkg.length + " 个）", "", "| 模块 | 职责 | 锚点数 | 代码 | 图 |", "|---|---|---|---|---|");
  for (const d of inPkg) {
    const short = d.file.split("/")[1]!;
    const md = short.replace(/\.ts$/, ".md");
    perPkg.push("| `" + d.name + "` | " + d.purpose + " | " + d.anchors.length + " | [" + short + "](" + d.file + ") | [" + md + "](" + d.file.replace(/\.ts$/, ".md") + ") |");
  }
  perPkg.push("");
}
writeFileSync(join(ROOT, "README.md"), head.concat(perPkg).join("\n"));

console.log("生成 " + docs.length + " 份状态图文档与 README.md");
console.log("子包 " + pkgDirs.length + "，锚点 " + uniqueReqs.size);