/**
 * 全量校验脚本：逐个跑通所有子包状态机，核对 spec 锚点真实存在。
 *
 * 用法：cd llmanspec/bloc && bun run scripts/verify.ts
 *
 * 检查项：
 * 1. 每个子包 .ts 文件都能被 bun 实跑并输出「N 组场景通过」，exit=0。
 * 2. 每个文件 JSDoc 里的 @spec 锚点，其 @req 编号在对应 .feature 中真实存在。
 * 3. 统计模块数与子包数，核对是否达到 30 个主业务模块 / 5 个子包的下限。
 */

import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join, basename } from "node:path";

const ROOT = join(import.meta.dir, "..");
const REPO = join(ROOT, "..", "..");
const SPECS = join(REPO, "llmanspec", "specs");

interface FileReport {
  pkg: string;
  file: string;
  ok: boolean;
  output: string;
  anchors: string[];
  badAnchors: string[];
}

/** 读取某 capability 的 .feature 文件里所有 @req 编号。 */
function reqIdsOf(capability: string): Set<string> {
  const dir = join(SPECS, capability);
  const candidates = [join(dir, `${capability}.feature`), join(SPECS, `${capability}.feature`)];
  for (const path of candidates) {
    if (existsSync(path)) {
      const text = readFileSync(path, "utf8");
      const ids = new Set<string>();
      for (const m of text.matchAll(/@req:([a-z0-9]+)/g)) ids.add(m[1]!);
      return ids;
    }
  }
  return new Set();
}

/** 从源文件 JSDoc 抽出 @spec 锚点。 */
function anchorsOf(source: string): string[] {
  const out: string[] = [];
  for (const m of source.matchAll(/@spec\s+(llmanspec\/specs\/([^/]+)\/[^#\s]+#@req:([a-z0-9]+))/g)) {
    out.push(m[1]!);
  }
  return out;
}

/** 校验一个锚点：capability 目录存在且 @req 编号在册。 */
function checkAnchor(anchor: string): boolean {
  const m = anchor.match(/llmanspec\/specs\/([^/]+)\/[^#]+#@req:([a-z0-9]+)/);
  if (!m) return false;
  const [, capability, req] = m;
  return reqIdsOf(capability!).has(req!);
}

const pkgDirs = readdirSync(ROOT, { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name)
  .filter((name) => name !== "node_modules" && name !== "shared" && name !== "scripts");

const reports: FileReport[] = [];
for (const pkg of pkgDirs) {
  const dir = join(ROOT, pkg);
  for (const entry of readdirSync(dir)) {
    if (!entry.endsWith(".ts")) continue;
    const file = join(dir, entry);
    const source = readFileSync(file, "utf8");
    const anchors = anchorsOf(source);
    const badAnchors = anchors.filter((a) => !checkAnchor(a));
    const proc = Bun.spawnSync(["bun", "run", file], { cwd: ROOT });
    const output = (proc.stdout.toString() + proc.stderr.toString()).trim();
    reports.push({
      pkg,
      file: `${pkg}/${entry}`,
      ok: proc.exitCode === 0 && badAnchors.length === 0,
      output,
      anchors,
      badAnchors,
    });
  }
}

const failed = reports.filter((r) => !r.ok);
const totalAnchors = new Set(reports.flatMap((r) => r.anchors)).size;
const uniqueReqs = new Set(reports.flatMap((r) => r.anchors.map((a) => a.split("#")[1]!))).size;

console.log(`子包数：${pkgDirs.length}`);
console.log(`状态机文件数：${reports.length}`);
console.log(`引用的不同 spec 文件数：${totalAnchors}`);
console.log(`引用的不同 @req 锚点数：${uniqueReqs}`);
console.log("");
for (const pkg of pkgDirs) {
  const inPkg = reports.filter((r) => r.pkg === pkg);
  console.log(`## ${pkg}（${inPkg.length} 个）`);
  for (const r of inPkg) {
    const mark = r.ok ? "通过" : "失败";
    console.log(`- ${mark} ${basename(r.file)}：${r.output.split("\n")[0] ?? ""}`);
    for (const bad of r.badAnchors) console.log(`  - 锚点不存在：${bad}`);
    if (!r.ok && r.output.split("\n").length > 1) {
      for (const line of r.output.split("\n").slice(0, 8)) console.log(`  ${line}`);
    }
  }
}

if (failed.length > 0) {
  console.error(`\n失败 ${failed.length} 个文件`);
  process.exit(1);
}
if (reports.length < 30 || pkgDirs.length < 5) {
  console.error(`\n未达下限：需要至少 30 个状态机、5 个子包`);
  process.exit(1);
}
console.log(`\n全部通过：${reports.length} 个状态机、${pkgDirs.length} 个子包、${uniqueReqs} 个锚点`);
