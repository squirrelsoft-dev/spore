use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use agent_sdk::SkillManifest;

use crate::mcp_handle::McpHandle;
use crate::registry_error::RegistryError;
use crate::tool_entry::ToolEntry;
use crate::transport;

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
                    .map(|entry| ToolEntry {
                        name: entry.name.clone(),
                        version: entry.version.clone(),
                        endpoint: entry.endpoint.clone(),
                        action_type: entry.action_type.clone(),
                        handle: entry.handle.clone(),
                    })
                    .ok_or_else(|| RegistryError::ToolNotFound {
                        name: tool_name.clone(),
                    })
            })
            .collect()
    }

    pub async fn connect(&self, name: &str) -> Result<(), RegistryError> {
        let endpoint = {
            let entries = self.entries.read().unwrap();
            let entry = entries.get(name).ok_or_else(|| RegistryError::ToolNotFound {
                name: name.to_string(),
            })?;
            entry.endpoint.clone()
        };

        let service = transport::connect_transport(&endpoint).await?;
        let handle = McpHandle::new(service);

        let mut entries = self.entries.write().unwrap();
        let entry = entries.get_mut(name).ok_or_else(|| RegistryError::ToolNotFound {
            name: name.to_string(),
        })?;
        entry.handle = Some(handle);
        Ok(())
    }

    pub async fn connect_all(&self) -> Result<(), RegistryError> {
        let names: Vec<String> = {
            let entries = self.entries.read().unwrap();
            entries.keys().cloned().collect()
        };

        let results = futures::future::join_all(
            names.iter().map(|name| self.connect(name)),
        )
        .await;

        for result in results {
            result?;
        }
        Ok(())
    }

    pub fn get_handle(&self, name: &str) -> Option<McpHandle> {
        let entries = self.entries.read().unwrap();
        entries.get(name).and_then(|entry| entry.handle.clone())
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
