use crate::{fallback, registry::ProviderRegistry};
use aeonic_core::{
    error::Result,
    traits::{Provider, Router, StreamResponse},
    types::{Request, Response},
};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::instrument;

/// The central router — receives requests, selects providers,
/// handles fallbacks, and returns responses.
///
/// # Example
/// ```rust,no_run
/// use aeonic_router::AeonicRouter;
/// use aeonic_providers::OpenAiProvider;
///
/// let router = AeonicRouter::builder()
///     .provider(OpenAiProvider::new("sk-..."))
///     .max_fallback_attempts(3)
///     .build();
/// ```
pub struct AeonicRouter {
    registry: Arc<ProviderRegistry>,
    max_fallback_attempts: u32,
}

impl AeonicRouter {
    pub fn builder() -> AeonicRouterBuilder {
        AeonicRouterBuilder::new()
    }

    /// Access the underlying provider registry.
    pub fn registry(&self) -> &Arc<ProviderRegistry> {
        &self.registry
    }

    /// Add a provider at runtime (thread-safe).
    pub fn add_provider(&self, provider: impl Provider) {
        self.registry.register(provider);
    }
}

#[async_trait]
impl Router for AeonicRouter {
    #[instrument(skip(self, request), fields(request_id = %request.id))]
    async fn route(&self, request: Request) -> Result<Response> {
        fallback::execute_with_fallback(
            &request,
            &self.registry,
            self.max_fallback_attempts,
        )
        .await
    }

    #[instrument(skip(self, request), fields(request_id = %request.id))]
    async fn route_stream(&self, request: Request) -> Result<StreamResponse> {
        fallback::stream_with_fallback(
            &request,
            &self.registry,
            self.max_fallback_attempts,
        )
        .await
    }

    fn providers(&self) -> Vec<&dyn Provider> {
        // DashMap doesn't easily yield &dyn Provider references —
        // return an empty vec here; use registry() for direct access.
        vec![]
    }
}

/// Builder for AeonicRouter.
pub struct AeonicRouterBuilder {
    registry: ProviderRegistry,
    max_fallback_attempts: u32,
}

impl AeonicRouterBuilder {
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
            max_fallback_attempts: 3,
        }
    }

    /// Register a provider.
    pub fn provider(self, provider: impl Provider) -> Self {
        self.registry.register(provider);
        self
    }

    /// Set maximum fallback attempts before giving up.
    pub fn max_fallback_attempts(mut self, n: u32) -> Self {
        self.max_fallback_attempts = n;
        self
    }

    /// Build the router.
    pub fn build(self) -> AeonicRouter {
        AeonicRouter {
            registry: Arc::new(self.registry),
            max_fallback_attempts: self.max_fallback_attempts,
        }
    }
}

impl Default for AeonicRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}
