use aeonic_core::{traits::Provider, types::ModelInfo};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

/// Thread-safe registry of all registered providers.
/// Providers are stored as Arc<dyn Provider> so they can be
/// shared across async tasks without cloning.
pub struct ProviderRegistry {
    providers: DashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: DashMap::new(),
        }
    }

    /// Register a provider. Replaces any existing provider with the same id.
    pub fn register(&self, provider: impl Provider) {
        let id = provider.id().to_string();
        info!("Registering provider: {id}");
        self.providers.insert(id, Arc::new(provider));
    }

    /// Register a pre-boxed Arc provider.
    pub fn register_arc(&self, provider: Arc<dyn Provider>) {
        let id = provider.id().to_string();
        info!("Registering provider (arc): {id}");
        self.providers.insert(id, provider);
    }

    /// Get a provider by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(id).map(|p| Arc::clone(&p))
    }

    /// List all registered provider ids.
    pub fn provider_ids(&self) -> Vec<String> {
        self.providers.iter().map(|e| e.key().clone()).collect()
    }

    /// List all models across all registered providers.
    pub fn all_models(&self) -> Vec<ModelInfo> {
        self.providers
            .iter()
            .flat_map(|e| e.value().models())
            .collect()
    }

    /// Find which provider owns a given model id.
    pub fn provider_for_model(&self, model_id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.iter().find_map(|e| {
            let provider = e.value();
            if provider.models().iter().any(|m| m.id == model_id) {
                Some(Arc::clone(provider))
            } else {
                None
            }
        })
    }

    /// Find a ModelInfo by model id across all providers.
    pub fn model_info(&self, model_id: &str) -> Option<ModelInfo> {
        self.providers.iter().find_map(|e| {
            e.value().model(model_id)
        })
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
