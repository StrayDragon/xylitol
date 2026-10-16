/** Live todo-bar reflow for tui-lab width experiments. Frozen YAML remains the snapshot SSOT. */

export const TODO_BAR_BREAKPOINT = 100;
export const TODO_BAR_MAX_ROWS = 6;
export const TODO_BAR_CONTENT_MAX = 80;

export type TodoStatus = "pending" | "in_progress" | "completed" | "cancelled";
export type TodoRow = { content: string; status: TodoStatus };
export type Fold = "folded" | "open";
export type Span = { text: string; token: string };

export type Scenario = {
  catalog: string;
  doing: Fold;
  past: Fold;
  pending: Fold;
};

const CATALOGS: Record<string, TodoRow[]> = {
  default: [
    { status: "completed", content: "read glossary" },
    { status: "cancelled", content: "skip overlay slot" },
    { status: "in_progress", content: "write todo-bar copy" },
    { status: "pending", content: "paint lab states" },
    { status: "pending", content: "graduate to product" },
    { status: "pending", content: "wire lower fixed zone" },
  ],
  wrap: [
    { status: "completed", content: "read glossary" },
    { status: "cancelled", content: "skip overlay slot" },
    {
      status: "in_progress",
      content:
        "Keep the live checklist in the lower dock and wrap long titles not clipping them",
    },
    { status: "pending", content: "paint lab states" },
    { status: "pending", content: "graduate to product" },
    { status: "pending", content: "wire lower fixed zone" },
  ],
  tall: [
    { status: "completed", content: "read glossary" },
    { status: "in_progress", content: "write todo-bar copy" },
    { status: "pending", content: "paint lab states" },
    { status: "pending", content: "graduate to product" },
    { status: "pending", content: "wire lower fixed zone" },
    { status: "pending", content: "review dock stack" },
    { status: "pending", content: "keep transcript visible" },
  ],
  done: [
    { status: "completed", content: "one" },
    { status: "completed", content: "two" },
    { status: "completed", content: "three" },
    { status: "completed", content: "four" },
    { status: "completed", content: "five" },
  ],
  pendingOnly: [
    { status: "pending", content: "alpha" },
    { status: "pending", content: "beta" },
    { status: "pending", content: "gamma" },
    { status: "pending", content: "delta" },
  ],
  noFocus: [
    { status: "completed", content: "read glossary" },
    { status: "cancelled", content: "skip overlay slot" },
    { status: "pending", content: "paint lab states" },
    { status: "pending", content: "graduate to product" },
    { status: "pending", content: "wire lower fixed zone" },
  ],
};

const SCENARIOS: Record<string, Scenario> = {
  empty: { catalog: "empty", doing: "folded", past: "folded", pending: "folded" },
  focus: { catalog: "default", doing: "open", past: "open", pending: "open" },
  "focus-wide": { catalog: "default", doing: "open", past: "open", pending: "open" },
  "wrap-stack": { catalog: "wrap", doing: "open", past: "folded", pending: "folded" },
  "wrap-wide": { catalog: "wrap", doing: "open", past: "folded", pending: "folded" },
  "past-open": { catalog: "default", doing: "open", past: "open", pending: "folded" },
  "past-open-wide": { catalog: "default", doing: "open", past: "open", pending: "folded" },
  "pending-open": { catalog: "default", doing: "open", past: "folded", pending: "open" },
  "pending-open-wide": { catalog: "default", doing: "folded", past: "folded", pending: "open" },
  "both-open": { catalog: "default", doing: "open", past: "open", pending: "open" },
  "both-open-wide": { catalog: "wrap", doing: "open", past: "open", pending: "open" },
  "no-focus": { catalog: "noFocus", doing: "open", past: "open", pending: "open" },
  "no-focus-wide": { catalog: "noFocus", doing: "open", past: "open", pending: "open" },
  "all-done": { catalog: "done", doing: "open", past: "open", pending: "open" },
  "all-pending": { catalog: "pendingOnly", doing: "open", past: "open", pending: "open" },
  capped: { catalog: "tall", doing: "open", past: "open", pending: "open" },
};

export function todoBarScenario(stateId: string): Scenario | null {
  return SCENARIOS[stateId] ?? null;
}

export function todoBarMaxRows(termRows: number): number {
  return Math.min(TODO_BAR_MAX_ROWS, Math.max(2, Math.floor(termRows / 4)));
}

function charCells(ch: string): number {
  const cp = ch.codePointAt(0) ?? 0;
  if (cp <= 0x1f) return 0;
  if (
    (cp >= 0x1100 && cp <= 0x115f) ||
    (cp >= 0x2e80 && cp <= 0xa4cf) ||
    (cp >= 0xac00 && cp <= 0xd7a3) ||
    (cp >= 0xf900 && cp <= 0xfaff) ||
    (cp >= 0xfe10 && cp <= 0xfe19) ||
    (cp >= 0xfe30 && cp <= 0xfe6f) ||
    (cp >= 0xff00 && cp <= 0xff60) ||
    (cp >= 0xffe0 && cp <= 0xffe6) ||
    (cp >= 0x1f300 && cp <= 0x1f64f) ||
    (cp >= 0x1f900 && cp <= 0x1f9ff)
  ) {
    return 2;
  }
  return 1;
}

function cells(s: string): number {
  let n = 0;
  for (const ch of s) n += charCells(ch);
  return n;
}

function pad(s: string, n: number): string {
  const w = cells(s);
  if (w >= n) return s;
  return s + " ".repeat(n - w);
}

function wrapWords(text: string, width: number): string[] {
  const w = Math.max(1, width);
  const words = text.split(" ");
  const lines: string[] = [];
  let cur = "";
  const pushHard = (token: string) => {
    let rest = token;
    while (cells(rest) > w) {
      const chars = [...rest];
      let acc = "";
      let i = 0;
      for (; i < chars.length; i += 1) {
        const next = acc + chars[i];
        if (cells(next) > w) break;
        acc = next;
      }
      if (!acc) {
        acc = chars[0] ?? "";
        i = 1;
      }
      lines.push(acc);
      rest = chars.slice(i).join("");
    }
    cur = rest;
  };
  for (const word of words) {
    const trial = cur ? `${cur} ${word}` : word;
    if (cells(trial) <= w) {
      cur = trial;
    } else {
      if (cur) lines.push(cur);
      if (cells(word) <= w) cur = word;
      else pushHard(word);
    }
  }
  if (cur) lines.push(cur);
  return lines.length ? lines : [""];
}

function mark(status: TodoStatus): string {
  if (status === "completed") return "[x] ";
  if (status === "cancelled") return "[-] ";
  if (status === "in_progress") return "";
  return "[ ] ";
}

function tokenFor(status: TodoStatus): string {
  return status === "in_progress" ? "on-surface" : "muted";
}

function paintItem(content: string, status: TodoStatus, inner: number): Span[][] {
  const tok = tokenFor(status);
  if (status === "in_progress") {
    return wrapWords(content, Math.max(1, inner)).map((line) => [
      { text: line, token: tok },
    ]);
  }
  const wrapped = wrapWords(content, Math.max(1, inner - 4));
  return wrapped.map((line, i) => [
    { text: `${i === 0 ? mark(status) : "    "}${line}`, token: tok },
  ]);
}

function doingHeader(n: number, open: boolean): string | null {
  return `${open ? "▾" : "▸"} ${n} doing`;
}

function pastHeader(items: TodoRow[], open: boolean): string | null {
  const completed = items.filter((i) => i.status === "completed").length;
  const cancelled = items.filter((i) => i.status === "cancelled").length;
  const tri = open ? "▾" : "▸";
  if (cancelled > 0) return `${tri} ${completed} completed · ${cancelled} cancelled`;
  return `${tri} ${completed} completed`;
}

function pendingHeader(items: TodoRow[], open: boolean): string | null {
  const n = items.filter((i) => i.status === "pending").length;
  return `${open ? "▾" : "▸"} ${n} pending`;
}

type ColBlock = Span[][];

function stackCol(
  header: string | null,
  body: TodoRow[],
  open: boolean,
  inner: number,
): ColBlock {
  const rows: Span[][] = [];
  if (header) rows.push([{ text: header, token: "muted" }]);
  if (open) {
    for (const item of body) {
      rows.push(...paintItem(item.content, item.status, inner));
    }
  }
  return rows;
}

function zipCols(
  cols: ColBlock[],
  widths: number[],
  gutter: number,
  totalCols: number,
): Span[][] {
  const height = Math.max(1, ...cols.map((c) => c.length));
  const gap: Span = { text: " ".repeat(gutter), token: "muted" };
  const lines: Span[][] = [];
  for (let r = 0; r < height; r += 1) {
    const row: Span[] = [];
    cols.forEach((col, i) => {
      if (i > 0) row.push(gap);
      const cell = col[r]?.[0];
      row.push({
        text: pad(cell?.text ?? "", widths[i] ?? 0),
        token: cell?.token ?? "muted",
      });
    });
    const joined = row.map((s) => s.text).join("");
    if (cells(joined) > totalCols) {
      let used = 0;
      for (const span of row) {
        const room = totalCols - used;
        if (room <= 0) {
          span.text = "";
          continue;
        }
        if (cells(span.text) > room) span.text = pad(span.text, room);
        used += cells(span.text);
      }
    }
    lines.push(row);
  }
  return lines;
}

const MIN_COL = 8;

function columnWidths(inner: number): [number, number, number] {
  const equal = Math.max(MIN_COL, Math.floor(inner / 3));
  const leftover = inner - equal * 3;
  return [equal, equal, equal + leftover];
}

function moreBorder(width: number, up: boolean, count: number): string {
  const arrow = up ? "↑" : "↓";
  const core = ` ${arrow} ${count} more `;
  const w = Math.max(1, width);
  const coreW = cells(core);
  if (coreW >= w) return [...core].slice(0, w).join("");
  const lead = "───";
  const leadW = cells(lead);
  if (leadW + coreW <= w) return `${lead}${core}${"─".repeat(w - leadW - coreW)}`;
  return `${"─".repeat(Math.max(0, w - coreW))}${core}`;
}

function wrapBorders(lines: Span[][], cols: number, hiddenBelow: number): Span[][] {
  const top: Span[] = [{ text: "─".repeat(Math.max(1, cols)), token: "muted" }];
  const bottomText =
    hiddenBelow > 0 ? moreBorder(cols, false, hiddenBelow) : "─".repeat(Math.max(1, cols));
  const bottom: Span[] = [{ text: bottomText, token: "muted" }];
  return [top, ...lines, bottom];
}

export function layoutTodoBar(stateId: string, cols: number, maxRows = TODO_BAR_MAX_ROWS): {
  lines: Span[][];
  cols: number;
  mode: "stack" | "columns";
  rows: number;
  overflow: boolean;
} | null {
  const scenario = todoBarScenario(stateId);
  if (!scenario) return null;
  const items = scenario.catalog === "empty" ? [] : (CATALOGS[scenario.catalog] ?? []);
  if (items.length === 0) {
    return { lines: [[{ text: " ", token: "muted" }]], cols, mode: "stack", rows: 0, overflow: false };
  }
  const pastItems = items.filter(
    (i) => i.status === "completed" || i.status === "cancelled",
  );
  const nowItems = items.filter((i) => i.status === "in_progress");
  const pendingItems = items.filter((i) => i.status === "pending");
  const doingH = doingHeader(nowItems.length, scenario.doing === "open");
  const pendH = pendingHeader(pendingItems, scenario.pending === "open");
  const pastH = pastHeader(pastItems, scenario.past === "open");
  const useCols = cols >= TODO_BAR_BREAKPOINT;
  let lines: Span[][] = [];
  if (!useCols) {
    lines.push(...stackCol(doingH, nowItems, scenario.doing === "open", cols));
    lines.push(
      ...stackCol(pendH, pendingItems, scenario.pending === "open", cols),
    );
    lines.push(...stackCol(pastH, pastItems, scenario.past === "open", cols));
  } else {
    const gutter = 2;
    const inner = cols - gutter * 2;
    const widths = columnWidths(inner);
    const left = stackCol(doingH, nowItems, scenario.doing === "open", widths[0]);
    const mid = stackCol(
      pendH,
      pendingItems,
      scenario.pending === "open",
      widths[1],
    );
    const right = stackCol(pastH, pastItems, scenario.past === "open", widths[2]);
    lines = zipCols([left, mid, right], widths, gutter, cols);
  }
  const total = lines.length;
  const overflow = total > maxRows;
  const hiddenBelow = overflow ? total - maxRows : 0;
  if (overflow) lines = lines.slice(0, maxRows);
  const contentRows = lines.length;
  lines = wrapBorders(lines, cols, hiddenBelow);
  return {
    lines,
    cols,
    mode: useCols ? "columns" : "stack",
    rows: contentRows,
    overflow,
  };
}
