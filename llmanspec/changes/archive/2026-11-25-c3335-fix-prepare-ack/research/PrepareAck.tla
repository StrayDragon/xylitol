---- MODULE PrepareAck ----
\* One acknowledgement of session prepare. The driver field and the actor
\* probe are the two copies in xylitol-runtime. New, fork, and resume clear
\* only the probe.

VARIABLES driverAck, probeAck, toolsFrozen
vars == <<driverAck, probeAck, toolsFrozen>>

Init ==
    /\ driverAck = TRUE
    /\ probeAck = TRUE
    /\ toolsFrozen = FALSE

\* The driver path clears both copies and the tool freeze.
FirePrepare ==
    /\ driverAck' = FALSE
    /\ probeAck' = FALSE
    /\ toolsFrozen' = FALSE

\* Settle and the tool-table freeze write both copies.
Settle ==
    /\ driverAck' = TRUE
    /\ probeAck' = TRUE
    /\ toolsFrozen' = TRUE

\* actor new, fork, and resume. The probe goes false. The driver copy stays.
ActorSwitch ==
    /\ driverAck = TRUE
    /\ probeAck' = FALSE
    /\ toolsFrozen' = FALSE
    /\ UNCHANGED driverAck

Wait == UNCHANGED vars

Next == FirePrepare \/ Settle \/ ActorSwitch \/ Wait

Spec == Init /\ [][Next]_vars

\* agent-runtime r1052: a new session, a fork, and a resume reset the prepare ack.
\* One ack has one value.
CopiesMatch == driverAck = probeAck

====
