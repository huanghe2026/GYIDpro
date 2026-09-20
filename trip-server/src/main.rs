//! trip-server 二进制入口。

use std::net::SocketAddr;

use axum::http::{header, HeaderValue, Method};
use axum::serve;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use trip_server::config::load_verifier_key;
use trip_server::{build_router, AppState, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let listen: SocketAddr = config.listen.parse()?;
    let verifier_key = load_verifier_key();
    let verifier_pubkey = verifier_key.public_bytes();

    // CORS：origin 列表含 "*" 时宽松放行；否则按精确匹配。
    let cors = if config.cors_origins.iter().any(|o| o == "*") {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers([header::CONTENT_TYPE])
            .allow_headers(Any)
    } else {
        let origins: Vec<HeaderValue> = config
            .cors_origins
            .iter()
            .filter_map(|o| o.parse::<HeaderValue>().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE])
    };

    let state = AppState::new(verifier_key, config);
    let app = build_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let listener = TcpListener::bind(listen).await?;
    tracing::info!(
        %listen,
        verifier_pubkey = %hex::encode(verifier_pubkey),
        "trip-server listening"
    );
    serve(listener, app).await?;
    Ok(())
}
