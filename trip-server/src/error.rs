//! trip-server HTTP 层错误类型。
//!
//! 协议数据本身的错误来自 trip-core 的 [`trip_core::TripError`]，统一映射为
//! 400（客户提交的 evidence/response 不合法）；流程类错误各自映射到
//! 404/410/413/500。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use trip_core::TripError;

/// 服务器错误。
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// trip-core 协议错误（CBOR/签名/链/引擎），一律视为客户输入不合法。
    #[error(transparent)]
    Core(#[from] TripError),

    /// 请求格式/字段不合法。
    #[error("bad request: {0}")]
    BadRequest(String),

    /// 引用的资源不存在（身份无 evidence、challenge 未知）。
    #[error("not found: {0}")]
    NotFound(String),

    /// 请求体超过单批上限。
    #[error("payload too large: {0}")]
    PayloadTooLarge(String),

    /// 挑战已过期或状态不允许该操作。
    #[error("challenge expired or invalid: {0}")]
    Expired(String),

    /// 服务端内部错误（阻塞任务崩溃等）。
    #[error("internal error: {0}")]
    Internal(String),
}

impl ServerError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ServerError::Core(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            ServerError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            ServerError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            ServerError::PayloadTooLarge(m) => (StatusCode::PAYLOAD_TOO_LARGE, m.clone()),
            ServerError::Expired(m) => (StatusCode::GONE, m.clone()),
            ServerError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        tracing::warn!(%status, error = %self, "request failed");
        (status, Json(json!({ "error": message }))).into_response()
    }
}

/// 服务器 Result 别名。
pub type ServerResult<T> = Result<T, ServerError>;
