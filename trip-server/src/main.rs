//! trip-server 二进制入口。

use std::net::SocketAddr;

use axum::serve;
use tokio::net::TcpListener;
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

    let state = AppState::new(verifier_key, config);
    let app = build_router(state).layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(listen).await?;
    tracing::info!(
        %listen,
        verifier_pubkey = %hex::encode(verifier_pubkey),
        "trip-server listening"
    );
    serve(listener, app).await?;
    Ok(())
}
