use aeonic_router::AeonicRouter;
use std::sync::Arc;

/// Shared application state injected into every route handler.
#[derive(Clone)]
pub struct AppState {
    pub router: Arc<AeonicRouter>,
}

impl AppState {
    pub fn new(router: Arc<AeonicRouter>) -> Self {
        Self { router }
    }
}
