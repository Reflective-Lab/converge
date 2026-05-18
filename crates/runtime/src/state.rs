// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Application state for Converge Runtime.

use std::sync::Arc;

#[cfg(feature = "gcp")]
use crate::db::Database;
use crate::templates::TemplateRegistry;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// Template registry for job templates.
    pub templates: Arc<TemplateRegistry>,

    /// Database connection (when gcp feature is enabled).
    #[cfg(feature = "gcp")]
    pub db: Option<Arc<Database>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("templates", &self.templates)
            .finish_non_exhaustive()
    }
}

impl AppState {
    /// Create new application state without database.
    pub fn new() -> Self {
        Self {
            templates: Arc::new(TemplateRegistry::with_defaults()),
            #[cfg(feature = "gcp")]
            db: None,
        }
    }

    /// Create application state with a custom template registry.
    pub fn with_templates(templates: TemplateRegistry) -> Self {
        Self {
            templates: Arc::new(templates),
            #[cfg(feature = "gcp")]
            db: None,
        }
    }

    /// Create application state with database connection.
    #[cfg(feature = "gcp")]
    pub fn with_database(db: Database) -> Self {
        Self {
            templates: Arc::new(TemplateRegistry::with_defaults()),
            db: Some(Arc::new(db)),
        }
    }

    /// Create application state with both templates and database.
    #[cfg(feature = "gcp")]
    pub fn with_templates_and_database(templates: TemplateRegistry, db: Database) -> Self {
        Self {
            templates: Arc::new(templates),
            db: Some(Arc::new(db)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Unit Tests: AppState creation
    // -------------------------------------------------------------------------

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        // Verify templates are initialized (should have defaults loaded)
        let _ = state.templates.list(); // Just verify it's accessible
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        // Default should be equivalent to new()
        let _ = state.templates.list(); // Just verify it's accessible
    }

    #[test]
    fn test_app_state_with_templates() {
        let registry = TemplateRegistry::new();
        let state = AppState::with_templates(registry);
        // Custom registry should be used
        assert_eq!(state.templates.list().len(), 0);
    }

    #[test]
    fn test_app_state_with_default_templates() {
        let registry = TemplateRegistry::with_defaults();
        let state = AppState::with_templates(registry);
        // Should have default templates loaded
        assert!(state.templates.list().len() > 0);
    }

    // -------------------------------------------------------------------------
    // Unit Tests: AppState Clone
    // -------------------------------------------------------------------------

    #[test]
    fn test_app_state_clone() {
        let state = AppState::new();
        let cloned = state.clone();
        // Both should have same template count
        assert_eq!(state.templates.list().len(), cloned.templates.list().len());
    }

    #[test]
    fn test_app_state_clone_shares_arc() {
        let state = AppState::new();
        let cloned = state.clone();
        // Arc pointers should be the same (shared reference)
        assert!(Arc::ptr_eq(&state.templates, &cloned.templates));
    }

    // -------------------------------------------------------------------------
    // Unit Tests: Template access
    // -------------------------------------------------------------------------

    #[test]
    fn test_app_state_templates_accessible() {
        let state = AppState::new();
        // Templates should be accessible through state
        let templates = state.templates.list();
        // Verify templates list is available (may be empty or have defaults)
        let _ = templates; // Just verify it's accessible without panicking
    }

    #[test]
    fn test_app_state_default_templates_content() {
        let state = AppState::new();
        let templates = state.templates.list();
        // If defaults are loaded, check they have expected structure
        for template in &templates {
            assert!(!template.name.is_empty());
        }
    }

    // -------------------------------------------------------------------------
    // Unit Tests: GCP feature (database)
    // -------------------------------------------------------------------------

    #[cfg(feature = "gcp")]
    #[test]
    fn test_app_state_new_has_no_db() {
        let state = AppState::new();
        assert!(state.db.is_none());
    }

    #[cfg(feature = "gcp")]
    #[test]
    fn test_app_state_with_templates_has_no_db() {
        let registry = TemplateRegistry::new();
        let state = AppState::with_templates(registry);
        assert!(state.db.is_none());
    }

    // -------------------------------------------------------------------------
    // Thread Safety Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_app_state_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AppState>();
    }

    #[test]
    fn test_app_state_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<AppState>();
    }

    #[tokio::test]
    async fn test_app_state_clone_across_tasks() {
        let state = AppState::new();
        let state_clone = state.clone();

        let handle = tokio::spawn(async move {
            // Access templates in spawned task
            let templates = state_clone.templates.list();
            templates.len()
        });

        let count = handle.await.unwrap();
        // Both should be able to access templates
        assert_eq!(count, state.templates.list().len());
    }

    #[tokio::test]
    async fn test_app_state_multiple_clones() {
        let state = AppState::new();

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let s = state.clone();
                tokio::spawn(async move { s.templates.list().len() })
            })
            .collect();

        for handle in handles {
            let count = handle.await.unwrap();
            assert_eq!(count, state.templates.list().len());
        }
    }

    // -------------------------------------------------------------------------
    // Edge Case Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_app_state_empty_registry() {
        let empty_registry = TemplateRegistry::new();
        let state = AppState::with_templates(empty_registry);
        assert_eq!(state.templates.list().len(), 0);
    }

    #[test]
    fn test_app_state_clone_preserves_templates() {
        let registry = TemplateRegistry::with_defaults();
        let original_count = registry.list().len();
        let state = AppState::with_templates(registry);
        let cloned = state.clone();

        assert_eq!(cloned.templates.list().len(), original_count);
    }
}
