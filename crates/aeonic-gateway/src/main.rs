mod routes;
mod state;
mod middleware;
mod error;

use aeonic_providers::{AnthropicProvider, OllamaProvider, OpenAiProvider};
use aeonic_router::AeonicRouter;
use axum::{routing::get, routing::post, Router};
use state::AppState;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,aeonic=debug".into()))
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    info!("Starting Aeonic Gateway v{}", env!("CARGO_PKG_VERSION"));

    let mut router_builder = AeonicRouter::builder().max_fallback_attempts(3);

    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        info!("Registering OpenAI provider");
        router_builder = router_builder.provider(OpenAiProvider::new(key));
    }
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        info!("Registering Anthropic provider");
        router_builder = router_builder.provider(AnthropicProvider::new(key));
    }
    if std::env::var("OLLAMA_ENABLED").as_deref() == Ok("true") {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".into());
        info!("Registering Ollama provider at {base_url}");
        router_builder = router_builder.provider(OllamaProvider::with_base_url(base_url));
    }

    let aeonic_router = Arc::new(router_builder.build());
    let state = Arc::new(AppState::new(aeonic_router));

    let app = Router::new()
        .route("/",          get(routes::dashboard::dashboard))
        .route("/dashboard", get(routes::dashboard::dashboard))
        .route("/health",    get(routes::health::health))
        .route("/v1/models", get(routes::models::list_models))
        .route("/v1/chat/completions", post(routes::chat::chat_completions))
        .route("/aeonic/v1/route",     post(routes::chat::aeonic_route))
        .with_state(state)
        .layer(
            tower::ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                ),
        );

    let host = std::env::var("AEONIC_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("AEONIC_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap_or(8080);

    let addr = SocketAddr::from((host.parse::<std::net::IpAddr>()?, port));
    info!("Aeonic Gateway listening on http://{addr}");
    info!("Dashboard: http://{addr}/dashboard");
    info!("OpenAI-compatible endpoint: http://{addr}/v1/chat/completions");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
