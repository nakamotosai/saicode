use runtime::PermissionMode;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifestEntry {
    pub name: String,
    pub source: ToolSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    Base,
    Conditional,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolRegistry {
    entries: Vec<ToolManifestEntry>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new(entries: Vec<ToolManifestEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub fn entries(&self) -> &[ToolManifestEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
}

impl ToolSpec {
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self.name {
            "bash" => "Bash",
            "read_file" => "Read",
            "write_file" => "Write",
            "edit_file" => "Edit",
            "glob_search" => "Glob",
            "grep_search" => "Grep",
            other => other,
        }
    }

    #[must_use]
    pub fn selector_aliases(&self) -> &'static [&'static str] {
        match self.name {
            "bash" => &["bash"],
            "read_file" => &["read", "read_file"],
            "write_file" => &["write", "write_file"],
            "edit_file" => &["edit", "edit_file"],
            "glob_search" => &["glob", "glob_search"],
            "grep_search" => &["grep", "grep_search"],
            "ListMcpResources" => &["listmcpresources", "list_mcp_resources"],
            "ReadMcpResource" => &["readmcpresource", "read_mcp_resource"],
            "McpAuth" => &["mcpauth", "mcp_auth"],
            "RemoteTrigger" => &["remotetrigger", "remote_trigger"],
            other => match other {
                "WebFetch" => &["webfetch"],
                "WebSearch" => &["websearch"],
                "TodoWrite" => &["todowrite", "todo_write"],
                "ToolSearch" => &["toolsearch", "tool_search"],
                "NotebookEdit" => &["notebookedit", "notebook_edit"],
                _ => &[],
            },
        }
    }
}
