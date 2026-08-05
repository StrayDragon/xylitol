//! Code-first MCP timing defaults (c1900). Not YAML this wave.

use std::time::Duration;

/// Wall-clock budget for a single MCP server connect attempt.
///
/// One hung SSE/stdio MUST NOT keep bootstrap `Running` forever.
pub const MCP_SERVER_CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
