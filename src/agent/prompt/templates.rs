//! Prompt template metadata for `/template:name` command surfacing.
//!
//! Templates are discovered by the resource loader and registered on the
//! [`Agent`](crate::agent::session::Agent) via `register_prompt_commands`, which
//! surfaces each template as a `/template:name` slash command (see
//! `Agent::get_commands`). The c320 change removed the dead in-Agent dispatch
//! path (`process_prompt`) together with its positional-argument expansion
//! machinery — none of it had a production call site (the application surface
//! owns command dispatch/expansion, not the capability aggregate).

/// Metadata for a loaded prompt template, used to surface `/template:name`
/// slash commands.
#[derive(Debug, Clone)]
pub(crate) struct PromptTemplate {
    /// Template name (derived from filename).
    pub(crate) name: String,
    /// Optional description from frontmatter.
    pub(crate) description: Option<String>,
    /// Provenance info for the template.
    pub(crate) source_info: Option<crate::domain::source_info::SourceInfo>,
}
