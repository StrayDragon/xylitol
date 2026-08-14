import { parse } from "yaml";
import type { ModulePreview, StateDoc } from "./types";

type Meta = Pick<ModulePreview, "id" | "title">;

const metas = Object.values(
  import.meta.glob("../../modules/*/preview.ts", {
    eager: true,
    import: "preview",
  }),
) as Meta[];

const yamlFiles = import.meta.glob("../../modules/*/states/*.yaml", {
  query: "?raw",
  eager: true,
  import: "default",
}) as Record<string, string>;

function statesFor(id: string): Record<string, StateDoc> {
  const out: Record<string, StateDoc> = {};
  const needle = `/modules/${id}/states/`;
  for (const [path, raw] of Object.entries(yamlFiles)) {
    if (!path.includes(needle)) continue;
    const name = path.split("/").pop()?.replace(/\.yaml$/, "") ?? "";
    out[name] = parse(raw) as StateDoc;
  }
  return out;
}

export function loadModules(): ModulePreview[] {
  const mods = metas.map((meta) => ({
    ...meta,
    states: statesFor(meta.id),
  }));
  mods.sort((a, b) => a.id.localeCompare(b.id));
  return mods;
}
