use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing with sensible defaults.
/// Call once at application startup.
/// Reads RUST_LOG for log level; defaults to "info,aeonic=debug".
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,aeonic=debug".into());

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
}

/// Initialize tracing with JSON output (for production/log aggregators).
pub fn init_tracing_json() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".into());

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
