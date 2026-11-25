---- MODULE SessionSlot ----
\* Session occupancy with linger greater than zero. Evict paths in the
\* current host both release the write lock and keep the sequence watermark.
\* A turn in flight is not evicted.

VARIABLES phase, inflight, lock, watermark, subs
vars == <<phase, inflight, lock, watermark, subs>>

Phase == {"held", "grace", "gone"}

TypeOK ==
    /\ phase \in Phase
    /\ inflight \in BOOLEAN
    /\ lock \in BOOLEAN
    /\ watermark \in BOOLEAN
    /\ subs \in {0, 1}

Init ==
    /\ phase = "held"
    /\ inflight = FALSE
    /\ lock = TRUE
    /\ watermark = FALSE
    /\ subs = 1

Subscribe ==
    /\ phase = "gone"
    /\ phase' = "held"
    /\ inflight' = FALSE
    /\ lock' = TRUE
    /\ subs' = 1
    /\ UNCHANGED watermark

LastClean ==
    /\ phase = "held"
    /\ inflight = FALSE
    /\ subs = 1
    /\ phase' = "gone"
    /\ inflight' = FALSE
    /\ lock' = FALSE
    /\ watermark' = TRUE
    /\ subs' = 0

LastUnclean ==
    /\ phase = "held"
    /\ inflight = FALSE
    /\ subs = 1
    /\ phase' = "grace"
    /\ lock' = TRUE
    /\ subs' = 0
    /\ UNCHANGED <<inflight, watermark>>

LastDuringTurn ==
    /\ phase = "held"
    /\ inflight = TRUE
    /\ subs = 1
    /\ subs' = 0
    /\ UNCHANGED <<phase, inflight, lock, watermark>>

TurnStarts ==
    /\ phase = "held"
    /\ inflight = FALSE
    /\ inflight' = TRUE
    /\ UNCHANGED <<phase, lock, watermark, subs>>

TurnEnds ==
    /\ inflight = TRUE
    /\ inflight' = FALSE
    /\ IF subs = 0
       THEN /\ phase' = "grace"
            /\ lock' = TRUE
            /\ UNCHANGED <<watermark, subs>>
       ELSE /\ UNCHANGED <<phase, lock, watermark, subs>>

LingerExpires ==
    /\ phase = "grace"
    /\ inflight = FALSE
    /\ subs = 0
    /\ phase' = "gone"
    /\ lock' = FALSE
    /\ watermark' = TRUE
    /\ UNCHANGED <<inflight, subs>>

Resubscribe ==
    /\ phase = "grace"
    /\ phase' = "held"
    /\ subs' = 1
    /\ lock' = TRUE
    /\ UNCHANGED <<inflight, watermark>>

Wait == UNCHANGED vars

Next ==
    \/ Subscribe
    \/ LastClean
    \/ LastUnclean
    \/ LastDuringTurn
    \/ TurnStarts
    \/ TurnEnds
    \/ LingerExpires
    \/ Resubscribe
    \/ Wait

Spec == Init /\ [][Next]_vars

\* server-core r1793: eviction releases the write occupancy.
LockReleased == phase = "gone" => ~lock

\* server-core r1806 and r2932: a rebuilt slot continues from the watermark.
WatermarkKept == phase = "gone" => watermark

\* server-core r2932: a turn in flight is not removed.
TurnStays == inflight => phase = "held"

====
