//! Pack registry for discovering and accessing packs

use converge_pack::Pack;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of available packs
///
/// The registry provides a way to discover and access packs by name.
/// It can be initialized with built-in packs or customized with
/// additional packs.
#[derive(Clone)]
pub struct PackRegistry {
    packs: HashMap<String, Arc<dyn Pack>>,
}

impl PackRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            packs: HashMap::new(),
        }
    }

    /// Create registry with all built-in packs
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(super::meeting_scheduler::MeetingSchedulerPack));
        registry.register(Arc::new(
            super::inventory_rebalancing::InventoryRebalancingPack,
        ));
        registry
    }

    /// Register a pack
    pub fn register(&mut self, pack: Arc<dyn Pack>) {
        self.packs.insert(pack.name().to_string(), pack);
    }

    /// Get a pack by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Pack>> {
        self.packs.get(name)
    }

    /// Check if a pack exists
    pub fn contains(&self, name: &str) -> bool {
        self.packs.contains_key(name)
    }

    /// List all registered pack names
    pub fn list(&self) -> Vec<&str> {
        self.packs.keys().map(|s| s.as_str()).collect()
    }

    /// Get number of registered packs
    pub fn len(&self) -> usize {
        self.packs.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }

    /// Iterate over all packs
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Arc<dyn Pack>)> {
        self.packs.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl Default for PackRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

impl std::fmt::Debug for PackRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackRegistry")
            .field("packs", &self.list())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = PackRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_builtins() {
        let registry = PackRegistry::with_builtins();
        assert!(!registry.is_empty());
        assert!(registry.contains("meeting-scheduler"));
        assert!(registry.contains("inventory-rebalancing"));
    }

    #[test]
    fn test_get_pack() {
        let registry = PackRegistry::with_builtins();
        let pack = registry.get("meeting-scheduler");
        assert!(pack.is_some());
        assert_eq!(pack.unwrap().name(), "meeting-scheduler");
    }

    #[test]
    fn test_list_packs() {
        let registry = PackRegistry::with_builtins();
        let names = registry.list();
        assert!(names.contains(&"meeting-scheduler"));
        assert!(names.contains(&"inventory-rebalancing"));
    }

    #[test]
    fn test_default() {
        let registry = PackRegistry::default();
        assert!(!registry.is_empty());
    }
}
