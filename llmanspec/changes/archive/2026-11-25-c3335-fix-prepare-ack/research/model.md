# Prepare acknowledgement model

TLC checked `PrepareAck.tla` with `PrepareAck.cfg`. It reached 4 states. `CopiesMatch` breaks. `NoDeadlock` holds.

The shortest path has 1 step.

0. Acked
1. new, fork, or resume. Old ack still on the driver

That step is `actor.rs` new, fork, and resume. Each sets the probe false and leaves the driver field unchanged. Replay is the three call sites, not a new test yet.

The model leaves out the hub view, the other prepare fields, and `NoActiveSession`. The chart hides the wait step, which changes nothing.

Regenerate from a scratch directory:

```sh
export TLA2TOOLS=/tmp/xylitol-statecharts/tla2tools.jar
export JAVA=/usr/bin/java
statechart PrepareAck.tla --config PrepareAck.cfg --view view.mjs --out /tmp/prepare-ack-out
```

`statechart` is `skills/statechart/scripts/statechart` in the pstack plugin. The jar is TLA+ 1.7.4, sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`.
