# c365 — Future / Known Issues

## Known bug (deferred): tool-call turn ends the REPL loop

After a turn that involves tool execution (e.g. bash), the REPL appears to
exit instead of looping back to the input prompt. Suspected area: the turn
drain / `TurnEnd` handling in `mod.rs`, or the `Driver` stream ending without
a `TurnEnd` after the last tool call. Not in c365's scope (streaming render);
filed for a follow-up change. Needs a focused investigation with a `FakeModel`
that emits a tool-call sequence, then a fix in the loop/turn-end path.
