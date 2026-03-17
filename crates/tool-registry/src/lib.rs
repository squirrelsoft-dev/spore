mod mcp_handle;
mod registry_error;
mod tool_entry;
mod tool_registry;
mod transport;

pub use mcp_handle::McpHandle;
pub use registry_error::RegistryError;
pub use tool_entry::ToolEntry;
pub use tool_registry::ToolRegistry;

/// Trait for checking whether a tool name is registered.
///
/// Used by the `validate` function to verify that all tool names
/// referenced in a `SkillManifest` actually exist in the runtime.
pub trait ToolExists {
    fn tool_exists(&self, name: &str) -> bool;
}
