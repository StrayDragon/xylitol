use std::sync::Arc;

use xylitol_plugin_sdk::XyTool;

#[derive(Clone, Default)]
pub struct ToolSet {
    tools: Vec<Arc<dyn XyTool>>,
}

impl ToolSet {
    pub fn empty() -> Self {
        Self { tools: Vec::new() }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I: IntoIterator<Item = Arc<dyn XyTool>>>(tools: I) -> Self {
        let mut set = Self::empty();
        for t in tools {
            set.tools.push(t);
        }
        set
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn XyTool>> {
        self.tools.iter()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn XyTool>> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .cloned()
            .or_else(|| {
                self.tools
                    .iter()
                    .find(|t| xylitol_plugin_sdk::tool_wire::to_wire_tool_name(t.name()) == name)
                    .cloned()
            })
    }

    pub fn plus(mut self, tool: Arc<dyn XyTool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn remove(mut self, name: &str) -> Self {
        self.tools.retain(|t| t.name() != name);
        self
    }

    pub fn merge(mut self, other: ToolSet) -> Self {
        self.tools.extend(other.tools);
        self
    }

    pub fn overlay_by_name(mut self, other: ToolSet) -> Self {
        for tool in other.tools {
            let name = tool.name();
            if let Some(pos) = self.tools.iter().position(|t| t.name() == name) {
                self.tools[pos] = tool;
            } else {
                self.tools.push(tool);
            }
        }
        self
    }

    pub fn rebuild_agent_tools(
        builtins: impl IntoIterator<Item = Arc<dyn XyTool>>,
        mcp: impl IntoIterator<Item = Arc<dyn XyTool>>,
    ) -> Self {
        Self::from_iter(builtins).overlay_by_name(Self::from_iter(mcp))
    }

    pub fn retain<F>(mut self, mut pred: F) -> Self
    where
        F: FnMut(&Arc<dyn XyTool>) -> bool,
    {
        self.tools.retain(|t| pred(t));
        self
    }
}
