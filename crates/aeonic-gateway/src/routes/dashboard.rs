use axum::{extract::State, response::Html};
use std::sync::Arc;
use crate::state::AppState;

pub async fn dashboard(State(_state): State<Arc<AppState>>) -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}
