use aeonic_router::AeonicRouter;
use aeonic_telemetry::TelemetryRecorder;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub router: Arc<AeonicRouter>,
    pub telemetry: TelemetryRecorder,
}

impl AppState {
    pub fn new(router: Arc<AeonicRouter>) -> Self {
        Self {
            router,
            telemetry: TelemetryRecorder::new(),
        }
    }
}
