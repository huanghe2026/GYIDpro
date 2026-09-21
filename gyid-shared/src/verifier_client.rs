//! 异步 Verifier HTTP 客户端（CLI / Android 用 reqwest）。
//!
//! 浏览器端用原生 `fetch` + `WebSocket`，不使用本模块（feature `http-client`）。
//!
//! 端点对应 trip-server 的路由（见 `trip-server/src/handlers/mod.rs`）：
//!
//! | 方法 | 路径 | 客户端方法 |
//! |---|---|---|
//! | POST | `/v1/evidence` | [`upload_evidence`](VerifierClient::upload_evidence) |
//! | POST | `/v1/verify` | [`request_challenge`](VerifierClient::request_challenge) |
//! | POST | `/v1/poh` | [`fetch_poh`](VerifierClient::fetch_poh) |
//! | GET  | `/v1/identity/:hex` | [`get_identity`](VerifierClient::get_identity) |
//! | GET  | `/v1/pohs?attester=` | [`list_pohs`](VerifierClient::list_pohs) |
//! | GET  | `/.well-known/verifier.json` | [`fetch_verifier_pubkey_hex`](VerifierClient::fetch_verifier_pubkey_hex) |

use serde::{Deserialize, Serialize};

use crate::{GyidError, GyidResult};

/// Verifier HTTP 客户端。
#[derive(Debug, Clone)]
pub struct VerifierClient {
    pub base_url: String,
    http: reqwest::Client,
}

/// `/v1/evidence` 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub identity: String,
    pub stored: u64,
    pub unique_cells: u64,
    pub chain_head: String,
}

/// `/v1/identity/:hex` 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityInfo {
    pub attester: String,
    pub breadcrumb_count: u64,
    pub unique_cells: u64,
    pub chain_head: String,
    pub last_ts: u64,
}

/// `/v1/verify` 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeInfo {
    pub challenge_id: String,
    pub expires_at: u64,
    pub delivered: bool,
}

/// `/v1/poh` 轮询结果。
#[derive(Debug, Clone)]
pub enum PohFetch {
    /// 已签发：原始 PoH CBOR 字节（RP 调用 [`crate::verify_poh`] 校验）。
    Issued(Vec<u8>),
    /// Attester 尚未应答；继续轮询。
    Pending { expires_at: u64 },
}

/// `/v1/pohs?attester=` 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PohsList {
    pub attester: String,
    pub challenge_ids: Vec<String>,
    pub count: u64,
}

impl VerifierClient {
    /// 构造客户端。`base_url` 末尾的 `/` 会被剥离。
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client build"),
        }
    }

    /// `POST /v1/evidence`：上传面包屑 CBOR 帧流（多条面包屑顺序拼接）。
    pub async fn upload_evidence(&self, body: Vec<u8>) -> GyidResult<UploadResponse> {
        let url = format!("{}/v1/evidence", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/cbor")
            .body(body)
            .send()
            .await
            .map_err(|e| GyidError::Http(e.to_string()))?;
        Self::decode_json(resp).await
    }

    /// `GET /v1/identity/:hex`：查询某 attester 的链统计。
    pub async fn get_identity(&self, pubkey_hex: &str) -> GyidResult<IdentityInfo> {
        let url = format!("{}/v1/identity/{}", self.base_url, pubkey_hex);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| GyidError::Http(e.to_string()))?;
        Self::decode_json(resp).await
    }

    /// `POST /v1/verify`：RP 发起 Active Verification。
    pub async fn request_challenge(
        &self,
        attester_hex: &str,
        rp_nonce_hex: &str,
    ) -> GyidResult<ChallengeInfo> {
        let url = format!("{}/v1/verify", self.base_url);
        let body = serde_json::json!({
            "attester": attester_hex,
            "rp_nonce": rp_nonce_hex,
        });
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| GyidError::Http(e.to_string()))?;
        Self::decode_json(resp).await
    }

    /// `POST /v1/poh`：RP 凭 `challenge_id` 轮询 PoH 证书。
    pub async fn fetch_poh(&self, challenge_id_hex: &str) -> GyidResult<PohFetch> {
        let url = format!("{}/v1/poh", self.base_url);
        let body = serde_json::json!({
            "challenge_id": challenge_id_hex,
        });
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| GyidError::Http(e.to_string()))?;
        let status = resp.status();
        match status.as_u16() {
            200 => {
                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| GyidError::Http(format!("read poh body: {e}")))?
                    .to_vec();
                Ok(PohFetch::Issued(bytes))
            }
            202 => {
                #[derive(Deserialize)]
                struct PendingBody {
                    expires_at: u64,
                }
                let b: PendingBody = Self::decode_json(resp).await?;
                Ok(PohFetch::Pending {
                    expires_at: b.expires_at,
                })
            }
            code => {
                let text = resp.text().await.unwrap_or_default();
                Err(GyidError::Verifier(code, text))
            }
        }
    }

    /// `GET /v1/pohs?attester=`：列出某 attester 已签发的 PoH challenge_id。
    pub async fn list_pohs(&self, attester_hex: &str) -> GyidResult<PohsList> {
        let url = format!("{}/v1/pohs?attester={}", self.base_url, attester_hex);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| GyidError::Http(e.to_string()))?;
        Self::decode_json(resp).await
    }

    /// `GET /.well-known/verifier.json`：取 Verifier 公钥 hex（64 字符）。
    pub async fn fetch_verifier_pubkey_hex(&self) -> GyidResult<String> {
        let url = format!("{}/.well-known/verifier.json", self.base_url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| GyidError::Http(e.to_string()))?;
        #[derive(Deserialize)]
        struct WellKnown {
            verifier_pubkey: String,
        }
        let v: WellKnown = Self::decode_json(resp).await?;
        Ok(v.verifier_pubkey)
    }

    /// 通用：HTTP 响应 → JSON（失败时把 status+body 转成 [`GyidError::Verifier`]）。
    async fn decode_json<T: for<'de> Deserialize<'de>>(resp: reqwest::Response) -> GyidResult<T> {
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GyidError::Verifier(status.as_u16(), text));
        }
        resp.json()
            .await
            .map_err(|e| GyidError::Http(format!("decode response: {e}")))
    }
}
