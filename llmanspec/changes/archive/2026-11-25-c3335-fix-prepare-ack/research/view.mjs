export const title = "Prepare acknowledgement";

export const state = (v) => {
  if (v.driverAck !== v.probeAck) return "Old ack still on the driver";
  if (v.driverAck && v.toolsFrozen) return "Acked and tools frozen";
  if (v.driverAck) return "Acked";
  return "Ack reset";
};

export const step = (action) => {
  if (action === "Wait") return null;
  if (action === "FirePrepare") return "prepare fires on the driver";
  if (action === "Settle") return "prepare settles";
  if (action === "ActorSwitch") return "new, fork, or resume";
  return null;
};

export const group = (v) => (v.driverAck === v.probeAck ? "one ack" : "split");

export const colors = { "one ack": "teal", split: "orange" };
