# Session occupancy model

TLC checked `SessionSlot.tla` with `SessionSlot.cfg`. It reached 9 states. `TypeOK`, `LockReleased`, `WatermarkKept`, `TurnStays`, and `NoDeadlock` hold. No rule breaks.

The chart draws 5 states and 9 arrows. It hides the wait step, which changes nothing.

The model assumes the grace period is longer than zero. A zero grace period uses the same immediate removal as a clean close. The model leaves out another process holding the kernel lock, and it leaves out the ping timeout. A dead connection enters the existing unclean path. The model does not contain journal bytes, only whether the sequence was kept.

Regenerate from a scratch directory:

```sh
export TLA2TOOLS=/tmp/xylitol-statecharts/tla2tools.jar
export JAVA=/usr/bin/java
statechart SessionSlot.tla --config SessionSlot.cfg --view view.mjs --out /tmp/session-slot-out
```

`statechart` is `skills/statechart/scripts/statechart` in the pstack plugin. The jar is TLA+ 1.7.4, sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`.
