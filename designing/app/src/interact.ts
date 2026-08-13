import type { ModulePreview } from "./types";

export function cycleState(mod: ModulePreview, stateId: string, dir: 1 | -1): string {
  const keys = Object.keys(mod.states);
  if (!keys.length) return stateId;
  const i = keys.indexOf(stateId);
  const from = i < 0 ? 0 : i;
  return keys[(from + dir + keys.length) % keys.length] ?? stateId;
}
