export const title = "Session occupancy";

export const state = (v) => {
  if (v.phase === "gone") {
    return v.watermark ? "Evicted, sequence kept" : "Evicted, sequence reset";
  }
  if (v.phase === "grace") return "Grace period, lock still held";
  if (v.inflight && v.subs === 0) return "Turn running, no subscribers";
  if (v.inflight) return "Occupied, turn running";
  return "Occupied";
};

export const step = (action) => {
  const labels = {
    Subscribe: "a client subscribes",
    LastClean: "the last client closes cleanly",
    LastUnclean: "the last client drops",
    LastDuringTurn: "the last client leaves during a turn",
    TurnStarts: "a turn starts",
    TurnEnds: "the turn ends",
    LingerExpires: "the grace period ends",
    Resubscribe: "the same process subscribes again",
  };
  return labels[action] ?? null;
};

export const group = (v) => {
  if (v.phase === "gone") return "released";
  if (v.phase === "grace") return "waiting";
  return "occupied";
};

export const colors = { occupied: "teal", waiting: "yellow", released: "tan" };
