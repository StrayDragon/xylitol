# Design: c195-wire-compaction-lifecycle

## Compaction Flow in AgentSession

```
agent_end fires
  → _handlePostAgentRun()
    1. lastAssistant stopReason == "error"? → _prepareRetry()
    2. context tokens > threshold? → _checkCompaction()
       → compact()
         → disconnect from agent
         → abort current operation
         → emit compaction_start
         → prepareCompaction() → find_cut_point()
         → generate_summary() via LLM
         → save CompactionEntry
         → reconnect to agent
         → emit compaction_end
    3. check queues → drain steering/followUp
```
