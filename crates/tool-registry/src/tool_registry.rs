use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use agent_sdk::SkillManifest;

use crate::registry_error::RegistryError;
use crate::tool_entry::ToolEntry;

pub struct ToolRegistry {
    entries: Arc<RwLock<HashMap<String, ToolEntry>>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, entry: ToolEntry) -> Result<(), RegistryError> {
        let mut map = self.entries.write().unwrap();
        if map.contains_key(&entry.name) {
            return Err(RegistryError::DuplicateEntry { name: entry.name });
        }
        let name = entry.name.clone();
        map.insert(name, entry);
        Ok(())
    }

    pub fn assert_exists(&self, name: &str) -> Result<(), RegistryError> {
        let map = self.entries.read().unwrap();
        if !map.contains_key(name) {
            return Err(RegistryError::ToolNotFound {
                name: name.to_string(),
            });
        }
        Ok(())
    }

    pub fn resolve_for_skill(
        &self,
        manifest: &SkillManifest,
    ) -> Result<Vec<ToolEntry>, RegistryError> {
        let map = self.entries.read().unwrap();
        manifest
            .tools
            .iter()
            .map(|tool_name| {
                map.get(tool_name)
                    .cloned()
                    .ok_or_else(|| RegistryError::ToolNotFound {
                        name: tool_name.clone(),
                    })
            })
            .collect()
    }

    pub fn connect(_url: &str) {
        // TODO: real MCP connection logic in issue #9
    }

    pub fn get(&self, name: &str) -> Option<ToolEntry> {
        let map = self.entries.read().unwrap();
        map.get(name).cloned()
    }
}

impl crate::ToolExists for ToolRegistry {
    fn tool_exists(&self, name: &str) -> bool {
        self.entries.read().unwrap().contains_key(name)
    }
}
