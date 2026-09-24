//! trip-server 二进制入口。

use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use trip_server::config::load_verifier_key;
use trip_server::listen::{serve, ListenTarget};
use trip_server::{build_router, chain::ChainRelay, AppState, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    // `127.0.0.1:8080` 走 TCP；`unix:/run/trip-server.sock` 走 Unix socket（零 TCP 端口）。
    let target = ListenTarget::parse(&config.listen)?;
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

    // 可选链上中继：TRIP_ANCHOR + EVM_PRIVATE_KEY 齐备才启用；否则静默关闭。
    let relay = match &config.anchor {
        Some(anchor) => {
            match ChainRelay::spawn(anchor, config.evm_rpc_url.as_deref(), config.epoch_size) {
                Ok(relay) => Some(relay),
                Err(reason) => {
                    tracing::warn!(
                        %reason,
                        "TRIP_ANCHOR is set but on-chain relay could not start; \
                         anchoring disabled (verification flows unaffected)"
                    );
                    None
                }
            }
        }
        None => {
            tracing::info!("TRIP_ANCHOR not set; on-chain relay disabled");
            None
        }
    };

    let state = AppState::new_with_relay(verifier_key, config, relay);
    let app = build_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    tracing::info!(
        listen = %target.describe(),
        verifier_pubkey = %hex::encode(verifier_pubkey),
        "trip-server starting"
    );
    serve(target, app).await?;
    Ok(())
}
