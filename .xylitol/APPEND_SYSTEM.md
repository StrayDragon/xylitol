<!-- Experiment 3 (c1610): temporary multi-tool policy fragment.
     Measure Ornith same-message multi-tool hit rate; remove or shrink after c1605/c1610 lands. -->

Tool calling policy:
- When several independent read-only tools are needed (read/grep/find/ls), emit ALL of them in the SAME assistant message as multiple tool calls.
- Do NOT narrate "I will call tools in parallel" and then emit only one tool call per turn.
- If you can only emit one tool call this turn, say so explicitly in one short sentence.
- Dependent writes/bash come AFTER the read results of the previous turn.
