package com.geoyuan.gyid.data

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONObject
import java.util.concurrent.TimeUnit

/**
 * Verifier HTTP/WS 客户端（与 Web 端 verifier.ts、trip-server 端点对齐）。
 *
 * 端点：
 * - POST /v1/evidence       octet-stream（面包屑 CBOR 帧拼接）
 * - POST /v1/verify         JSON {attester, rp_nonce}
 * - WS   /v1/challenge?attester=<hex>
 * - POST /v1/poh            JSON {challenge_id}（200=CBOR / 202=Pending / 410=过期）
 * - GET  /v1/pohs?attester=
 * - GET  /.well-known/verifier.json
 *
 * 所有字节在 Rust 边界用 hex，HTTP 传输层再转回 bytes。
 */
class VerifierApi(private val baseUrl: String) {

    private val http = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    private val jsonMedia = "application/json".toMediaType()
    private val cborMedia = "application/octet-stream".toMediaType()

    /** GET /v1/identity/:pubkey 响应体；404（从未上传）返回 null。 */
    data class IdentityInfo(
        val breadcrumbCount: Long,
        val uniqueCells: Long,
        val chainHead: String,
        val lastTs: Long,
    )

    /** POST /v1/evidence 响应体。 */
    data class UploadInfo(val stored: Long, val uniqueCells: Long, val chainHead: String)

    /** POST /v1/verify 响应体。 */
    data class ChallengeInfo(
        val challengeId: String,
        val expiresAt: Long,
        val delivered: Boolean,
    )

    /** 拉取 attester 公开身份统计；404 返回 null（还没上传过面包屑）。 */
    fun fetchIdentity(attesterHex: String): IdentityInfo? {
        val resp = http.newCall(
            Request.Builder().url("$baseUrl/v1/identity/$attesterHex").build(),
        ).execute()
        if (resp.code == 404) {
            resp.close()
            return null
        }
        return resp.use {
            val j = it.bodyJson()
            IdentityInfo(
                breadcrumbCount = j.getLong("breadcrumb_count"),
                uniqueCells = j.getLong("unique_cells"),
                chainHead = j.getString("chain_head"),
                lastTs = j.getLong("last_ts"),
            )
        }
    }

    /** POST /v1/evidence：上传面包屑 CBOR 帧流（可多帧拼接）。 */
    fun uploadEvidence(cborBytes: ByteArray): UploadInfo {
        val resp = http.newCall(
            Request.Builder()
                .url("$baseUrl/v1/evidence")
                .post(cborBytes.toRequestBody(cborMedia))
                .build(),
        ).execute()
        return resp.use {
            if (!it.isSuccessful) {
                throw IllegalStateException("uploadEvidence HTTP ${it.code}: ${it.body?.string()}")
            }
            val j = it.bodyJson()
            UploadInfo(
                stored = j.optLong("stored"),
                uniqueCells = j.optLong("unique_cells"),
                chainHead = j.optString("chain_head"),
            )
        }
    }

    /** POST /v1/verify：发起 Active Verification。 */
    fun requestChallenge(attesterHex: String, rpNonceHex: String): ChallengeInfo {
        val body = JSONObject()
            .put("attester", attesterHex)
            .put("rp_nonce", rpNonceHex)
            .toString()
        val resp = http.newCall(
            Request.Builder()
                .url("$baseUrl/v1/verify")
                .post(body.toRequestBody(jsonMedia))
                .build(),
        ).execute()
        return resp.use {
            if (!it.isSuccessful) {
                throw IllegalStateException("requestChallenge HTTP ${it.code}: ${it.body?.string()}")
            }
            val j = it.bodyJson()
            ChallengeInfo(
                challengeId = j.getString("challenge_id"),
                expiresAt = j.getLong("expires_at"),
                delivered = j.optBoolean("delivered", false),
            )
        }
    }

    /**
     * POST /v1/poh 轮询。
     * @return [PohFetch.Issued]（CBOR 字节）/ [PohFetch.Pending]（deadline）。
     */
    fun fetchPoh(challengeIdHex: String): PohFetch {
        val body = JSONObject().put("challenge_id", challengeIdHex).toString()
        val resp = http.newCall(
            Request.Builder()
                .url("$baseUrl/v1/poh")
                .post(body.toRequestBody(jsonMedia))
                .build(),
        ).execute()
        return resp.use { r ->
            when (r.code) {
                200 -> PohFetch.Issued(r.body!!.bytes())
                202 -> PohFetch.Pending(r.bodyJson().getLong("expires_at"))
                else -> throw IllegalStateException("fetchPoh HTTP ${r.code}: ${r.body?.string()}")
            }
        }
    }

    /** GET /.well-known/verifier.json：Verifier 公钥 hex（RP 校验 PoH 签名用）。 */
    fun verifierPubkeyHex(): String {
        val resp = http.newCall(
            Request.Builder().url("$baseUrl/.well-known/verifier.json").build(),
        ).execute()
        return resp.use { it.bodyJson().getString("verifier_pubkey") }
    }

    /**
     * 打开 Attester 侧挑战 WebSocket。ready 文本帧与二进制挑战帧
     * 经 [listener] 回调；调用方在 onMessage(ByteString) 里用 Rust
     * 签名后 `webSocket.send(ByteString)` 回送。
     */
    fun openChallengeSocket(attesterHex: String, listener: WebSocketListener): WebSocket {
        val wsBase = baseUrl.replace("http://", "ws://").replace("https://", "wss://")
        val request = Request.Builder()
            .url("$wsBase/v1/challenge?attester=$attesterHex")
            .build()
        return http.newWebSocket(request, listener)
    }

    sealed class PohFetch {
        data class Issued(val cbor: ByteArray) : PohFetch()
        data class Pending(val expiresAt: Long) : PohFetch()
    }

    private fun Response.bodyJson(): JSONObject =
        JSONObject(body?.string() ?: throw IllegalStateException("empty body"))
}
