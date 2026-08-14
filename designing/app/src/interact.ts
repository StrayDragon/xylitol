import type { ModulePreview } from "./types";

/** Product-key toggles that have a pair of designing states. */
const KEY_TOGGLES: Record<string, { alt?: string; ctrl?: string; a: string; b: string }[]> = {
  expandable: [{ alt: "e", a: "tool-collapsed", b: "tool-expanded" }],
  compaction: [{ alt: "e", a: "collapsed", b: "expanded" }],
  "session-resume": [{ ctrl: "u", a: "default", b: "id-on" }],
};

function toggle(stateId: string, a: string, b: string): string | null {
  if (stateId === a) return b;
  if (stateId === b) return a;
  return null;
}

export function cycleState(mod: ModulePreview, stateId: string, dir: 1 | -1): string {
  const keys = Object.keys(mod.states);
  if (!keys.length) return stateId;
  const i = keys.indexOf(stateId);
  const from = i < 0 ? 0 : i;
  return keys[(from + dir + keys.length) % keys.length] ?? stateId;
}

export function stateFromComponentKey(
  mod: ModulePreview,
  stateId: string,
  ev: KeyboardEvent,
): string | null {
  const rows = KEY_TOGGLES[mod.id] ?? [];
  const key = ev.key.length === 1 ? ev.key.toLowerCase() : ev.key.toLowerCase();
  for (const row of rows) {
    if (row.alt && ev.altKey && !ev.ctrlKey && !ev.metaKey && key === row.alt) {
      return toggle(stateId, row.a, row.b);
    }
    if (row.ctrl && ev.ctrlKey && !ev.altKey && !ev.metaKey && key === row.ctrl) {
      return toggle(stateId, row.a, row.b);
    }
  }
  return null;
}
