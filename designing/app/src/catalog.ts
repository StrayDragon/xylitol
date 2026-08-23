import { parse } from "yaml";
import type { Alignment, DraftDoc, ModulePreview, StateDoc } from "./types";
import type { Sim } from "./sim";

const drafts = import.meta.glob("../../*/modules/*/draft.yaml", {
  query: "?raw",
  eager: true,
  import: "default",
}) as Record<string, string>;

const sims = import.meta.glob("../../*/modules/*/sim.ts", {
  eager: true,
}) as Record<string, { sim: Sim }>;

const intents = import.meta.glob("../../*/modules/*/intent.md", {
  query: "?raw",
  eager: true,
  import: "default",
}) as Record<string, string>;

const yamlFiles = import.meta.glob("../../*/modules/*/states/*.yaml", {
  query: "?raw",
  eager: true,
  import: "default",
}) as Record<string, string>;

function parseDraftPath(path: string): { surface: string; id: string } {
  const m = path.match(/\/([^/]+)\/modules\/([^/]+)\/draft\.yaml$/);
  return { surface: m?.[1] ?? "tui", id: m?.[2] ?? "" };
}

function statesFor(surface: string, id: string): Record<string, StateDoc> {
  const out: Record<string, StateDoc> = {};
  const needle = `/${surface}/modules/${id}/states/`;
  for (const [path, raw] of Object.entries(yamlFiles)) {
    if (!path.includes(needle)) continue;
    const name = path.split("/").pop()?.replace(/\.yaml$/, "") ?? "";
    out[name] = parse(raw) as StateDoc;
  }
  return out;
}

function parseSimPath(path: string): { surface: string; id: string } {
  const m = path.match(/\/([^/]+)\/modules\/([^/]+)\/sim\.ts$/);
  return { surface: m?.[1] ?? "tui", id: m?.[2] ?? "" };
}

export function simFor(surface: string, id: string): Sim | null {
  for (const [path, mod] of Object.entries(sims)) {
    const key = parseSimPath(path);
    if (key.surface === surface && key.id === id) return mod.sim;
  }
  return null;
}

function intentFor(surface: string, id: string): string {
  const needle = `/${surface}/modules/${id}/intent.md`;
  const hit = Object.entries(intents).find(([path]) => path.includes(needle));
  return hit?.[1] ?? "";
}

function normalizeAlignment(raw: unknown): Alignment {
  if (raw && typeof raw === "object" && !Array.isArray(raw)) {
    return raw as Alignment;
  }
  const out: Alignment = {};
  if (Array.isArray(raw)) {
    for (const item of raw) {
      if (item && typeof item === "object" && !Array.isArray(item)) {
        Object.assign(out, item);
      } else if (typeof item === "string") {
        const i = item.indexOf(":");
        if (i > 0) {
          const key = item.slice(0, i).trim() as keyof Alignment;
          out[key] = item.slice(i + 1).trim();
        }
      }
    }
  }
  return out;
}

export function loadModules(): ModulePreview[] {
  const mods: ModulePreview[] = [];
  for (const [path, raw] of Object.entries(drafts)) {
    const { surface, id } = parseDraftPath(path);
    if (!id || surface === "app" || surface === "generated") continue;
    const draft = parse(raw) as DraftDoc;
    mods.push({
      ...draft,
      id: draft.id || id,
      surface: draft.surface || surface,
      intent: intentFor(surface, id),
      states: statesFor(surface, id),
      alignment: normalizeAlignment(draft.alignment),
      todos: draft.todos ?? [],
      keys: draft.keys ?? [],
      notes: (draft.notes ?? "").trim(),
    });
  }
  mods.sort((a, b) => {
    const s = a.surface.localeCompare(b.surface);
    return s !== 0 ? s : a.id.localeCompare(b.id);
  });
  return mods;
}
