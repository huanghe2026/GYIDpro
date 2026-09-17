//! `POST /v1/poh`：RP 凭 challenge_id 取 PoH 证书。
//!
//! 请求 JSON：`{ "challenge_id": "<hex16>" }`。
//! - 已签发（Issued）：200 + `Content-Type: application/cbor`，体为 PoH CBOR；
//! - 待应答（Pending）：202 + JSON 状态（RP 继续轮询）；
//! - 已过期/已消费：410；未知 challenge：404。

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::config::now_unix_secs;
use crate::error::{ServerError, ServerResult};
use crate::handlers::parse_hex16;
use crate::state::{AppState, ChallengeStatus};

#[derive(Debug, Deserialize)]
pub struct PohRequest {
    pub challenge_id: String,
}

pub async fn fetch(
    State(state): State<AppState>,
    Json(req): Json<PohRequest>,
) -> ServerResult<impl IntoResponse> {
    let challenge_id = parse_hex16(&req.challenge_id, "challenge_id")?;

    let record = {
        let challenges = state.inner.challenges.read().await;
        challenges
            .get(&challenge_id)
            .cloned()
            .ok_or_else(|| ServerError::NotFound("unknown challenge_id".into()))?
    };

    // Pending 但已过 deadline：惰性置为 Expired。
    if record.status == ChallengeStatus::Pending && now_unix_secs() > record.deadline {
        state
            .inner
            .challenges
            .write()
            .await
            .entry(challenge_id)
            .and_modify(|r| r.status = ChallengeStatus::Expired);
        return Err(ServerError::Expired("challenge deadline passed".into()));
    }

    match record.status {
        ChallengeStatus::Pending => Ok((
            axum::http::StatusCode::ACCEPTED,
            Json(json!({"status":"pending","expires_at": record.deadline})),
        )
            .into_response()),
        ChallengeStatus::Expired => Err(ServerError::Expired("challenge expired".into())),
        ChallengeStatus::Issued => {
            let cbor = state
                .inner
                .poh
                .read()
                .await
                .get(&challenge_id)
                .cloned()
                .ok_or_else(|| ServerError::Internal("issued challenge missing PoH".into()))?;
            Ok(([(header::CONTENT_TYPE, "application/cbor")], cbor).into_response())
        }
    }
}
