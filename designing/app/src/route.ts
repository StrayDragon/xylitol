export type Scheme = "dark" | "light";

export type Route = {
  surface: string;
  id: string;
  state: string;
  frame: number;
  play: boolean;
  scheme: Scheme;
};

const SURFACES = new Set(["tui", "web"]);

function truthy(raw: string | null): boolean {
  if (raw == null || raw === "") return false;
  return raw === "1" || raw === "true" || raw === "yes";
}

export function parseLocation(loc: Location = location): Partial<Route> {
  let pathname = loc.pathname;
  const hash = loc.hash.replace(/^#/, "");
  if ((!pathname || pathname === "/") && hash) {
    pathname = `/${hash.replace(/^\//, "")}`;
  }
  const parts = pathname.split("/").filter(Boolean);
  const surface = parts[0] && SURFACES.has(parts[0]) ? parts[0] : undefined;
  const id = parts[1];
  const state = parts[2];
  const q = new URLSearchParams(loc.search);
  const frameRaw = q.get("frame");
  const frame = frameRaw != null && frameRaw !== "" ? Number(frameRaw) : undefined;
  const schemeRaw = q.get("scheme");
  const scheme =
    schemeRaw === "dark" || schemeRaw === "light" ? schemeRaw : undefined;
  return {
    surface,
    id,
    state,
    frame: Number.isFinite(frame) ? Math.max(0, Math.floor(frame as number)) : undefined,
    play: q.has("play") ? truthy(q.get("play")) : undefined,
    scheme,
  };
}

export function formatPath(route: Pick<Route, "surface" | "id" | "state">): string {
  return `/${route.surface}/${route.id}/${route.state}`;
}

export function formatSearch(route: Pick<Route, "frame" | "play" | "scheme">): string {
  const q = new URLSearchParams();
  if (route.frame > 0) q.set("frame", String(route.frame));
  if (route.play) q.set("play", "1");
  if (route.scheme === "dark") q.set("scheme", "dark");
  const s = q.toString();
  return s ? `?${s}` : "";
}

export function formatUrl(route: Route): string {
  return `${formatPath(route)}${formatSearch(route)}`;
}

export function writeLocation(route: Route, mode: "replace" | "push"): void {
  const url = formatUrl(route);
  const now = `${location.pathname}${location.search}`;
  if (now === url && !location.hash) return;
  if (mode === "push") history.pushState(null, "", url);
  else history.replaceState(null, "", url);
}
