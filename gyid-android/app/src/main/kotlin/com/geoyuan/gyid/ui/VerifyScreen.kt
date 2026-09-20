package com.geoyuan.gyid.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.geoyuan.gyid.data.CollectBus
import com.geoyuan.gyid.data.IdentityStore
import com.geoyuan.gyid.data.SettingsStore
import com.geoyuan.gyid.data.VerifierApi
import com.geoyuan.gyid.data.hexToBytes
import com.geoyuan.gyid.data.toHex
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.launch
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import org.json.JSONObject
import java.security.SecureRandom
import uniffi.gyid.PohInfo
import uniffi.gyid.signLivenessResponse
import uniffi.gyid.verifyPoh

/**
 * RP 主动验证（draft §5，对齐 Web Verify.tsx 六步）：
 *   1) WS 连 /v1/challenge 等 ready 文本帧；
 *   2) SecureRandom 16 字节 rp_nonce → POST /v1/verify；
 *   3) 收二进制 LivenessChallenge → signLivenessResponse → 二进制回送；
 *   4) 1s 轮询 POST /v1/poh 至签发（或 expires_at+15s 超时）；
 *   5) verifyPoh 校验签名/新鲜度/策略门槛（α≥0.1, trust≥20）。
 *
 * 验证期间必须停止采集：挑战绑定链头，链头变化服务端拒签。
 */
private enum class Step(val label: String) {
    IDLE("待开始"),
    CONNECTING("连接挑战通道…"),
    REQUESTING("提交验证请求…"),
    WAIT_CHALLENGE("等待服务端下发挑战…"),
    RESPONDING("用设备密钥签名存活挑战…"),
    POLLING("等待 Verifier 签发 PoH…"),
    VERIFYING("本地校验 PoH…"),
    DONE("完成"),
    ERROR("失败"),
}

@Composable
fun VerifyScreen() {
    val context = androidx.compose.ui.platform.LocalContext.current
    val identity = remember { IdentityStore(context) }
    val settings = remember { SettingsStore(context) }
    val bus by CollectBus.state.collectAsState()
    val scope = rememberCoroutineScope()

    var step by remember { mutableStateOf(Step.IDLE) }
    var error by remember { mutableStateOf<String?>(null) }
    var nonceHex by remember { mutableStateOf<String?>(null) }
    var challengeId by remember { mutableStateOf<String?>(null) }
    var poh by remember { mutableStateOf<PohInfo?>(null) }

    suspend fun run() {
        error = null
        poh = null
        val seed = identity.seedHex() ?: run {
            step = Step.ERROR; error = "本地无身份"; return
        }
        val pubkey = identity.pubkeyHex() ?: run {
            step = Step.ERROR; error = "公钥推导失败"; return
        }
        val api = VerifierApi(settings.verifierUrl)

        val ready = CompletableDeferred<Unit>()
        val challenge = CompletableDeferred<ByteArray>()
        var socket: WebSocket? = null
        val listener = object : WebSocketListener() {
            override fun onMessage(webSocket: WebSocket, text: String) {
                // 服务端先发 {"type":"ready",...}；其余回执文本忽略
                if (runCatching { JSONObject(text).optString("type") }.getOrNull() == "ready") {
                    ready.complete(Unit)
                }
            }
            override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
                challenge.complete(bytes.toByteArray())
            }
            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                val e = "WebSocket 失败：${t.message}"
                ready.completeExceptionally(IllegalStateException(e))
                challenge.completeExceptionally(IllegalStateException(e))
            }
        }

        try {
            // 1) 连接 + ready 握手
            step = Step.CONNECTING
            socket = withContext(Dispatchers.IO) { api.openChallengeSocket(pubkey, listener) }
            withTimeoutOrNull(10_000) { ready.await() }
                ?: throw IllegalStateException("挑战通道 ready 超时")

            // 2) rp_nonce + POST /v1/verify
            step = Step.REQUESTING
            val nonce = ByteArray(16).also { SecureRandom().nextBytes(it) }
            nonceHex = nonce.toHex()
            val info = withContext(Dispatchers.IO) {
                api.requestChallenge(pubkey, nonceHex!!)
            }
            challengeId = info.challengeId
            if (!info.delivered) {
                throw IllegalStateException("challenge 未投递（attester 不在线或无面包屑？HTTP delivered=false）")
            }

            // 3) 等二进制挑战
            step = Step.WAIT_CHALLENGE
            val cbBytes = withTimeoutOrNull(info.expiresAt * 1000L - System.currentTimeMillis() + 5_000) {
                challenge.await()
            } ?: throw IllegalStateException("等待挑战帧超时")

            // 4) Rust 签名并回送
            step = Step.RESPONDING
            val respHex = withContext(Dispatchers.IO) {
                signLivenessResponse(seed, cbBytes.toHex())
            }
            socket.send(ByteString.of(*respHex.hexToBytes()))

            // 5) 轮询 PoH
            step = Step.POLLING
            var pohCbor: ByteArray? = null
            val deadline = info.expiresAt + 15
            while (System.currentTimeMillis() / 1000 < deadline) {
                when (val r = withContext(Dispatchers.IO) { api.fetchPoh(info.challengeId) }) {
                    is VerifierApi.PohFetch.Issued -> { pohCbor = r.cbor; break }
                    is VerifierApi.PohFetch.Pending -> delay(1_000)
                }
            }
            val cbor = pohCbor ?: throw IllegalStateException("PoH 签发超时（deadline=${info.expiresAt}）")

            // 6) 本地校验
            step = Step.VERIFYING
            val verifierVk = withContext(Dispatchers.IO) { api.verifierPubkeyHex() }
            val result = withContext(Dispatchers.IO) {
                verifyPoh(
                    cbor.toHex(),
                    verifierVk,
                    nonceHex!!,
                    (System.currentTimeMillis() / 1000).toULong(),
                    MIN_CONFIDENCE,
                    MIN_TRUST,
                )
            }
            poh = result
            step = Step.DONE
        } catch (e: Throwable) {
            error = e.message ?: e.javaClass.simpleName
            step = Step.ERROR
        } finally {
            socket?.close(1000, null)
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("主动验证 / PoH", style = MaterialTheme.typography.headlineSmall)

        if (!identity.exists()) {
            Text("✗ 请先在「身份」页创建身份", color = MaterialTheme.colorScheme.error)
            return@Column
        }

        if (bus.running) {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.errorContainer,
                ),
            ) {
                Text(
                    "采集服务正在运行：挑战绑定当前链头，验证期间新增面包屑会导致拒签，" +
                        "请先到「采集」页停止采集。",
                    modifier = Modifier.padding(12.dp),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Step.entries.forEach { s ->
                    val active = s == step
                    Row(
                        verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Text(
                            when {
                                active -> "●"
                                step != Step.ERROR && s.ordinal < step.ordinal -> "✓"
                                else -> "○"
                            },
                            color = when {
                                s == Step.ERROR && active -> MaterialTheme.colorScheme.error
                                active || (step != Step.ERROR && s.ordinal < step.ordinal) ->
                                    MaterialTheme.colorScheme.primary
                                else -> MaterialTheme.colorScheme.onSurfaceVariant
                            },
                        )
                        Text(
                            s.label,
                            style = MaterialTheme.typography.bodyMedium,
                            color = if (active) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.onSurface,
                        )
                        if (active && step != Step.DONE && step != Step.ERROR) {
                            CircularProgressIndicator(
                                modifier = Modifier.padding(start = 4.dp),
                                strokeWidth = 2.dp,
                            )
                        }
                    }
                }
            }
        }

        nonceHex?.let {
            Text("rp_nonce：$it", fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall)
        }
        challengeId?.let {
            Text("challenge_id：$it", fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall)
        }

        error?.let {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.errorContainer,
                ),
            ) {
                Text("✗ $it", modifier = Modifier.padding(12.dp))
            }
        }

        poh?.let { p ->
            val passed = p.fresh && p.policy
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = if (passed) MaterialTheme.colorScheme.primaryContainer
                    else MaterialTheme.colorScheme.errorContainer,
                ),
            ) {
                Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(
                        if (passed) "✓ PoH 验证通过（FRESH + POLICY PASS）"
                        else "✗ PoH 未通过门槛（fresh=${p.fresh}, policy=${p.policy}）",
                        style = MaterialTheme.typography.titleMedium,
                    )
                    Text("置信度 α = %.4f（门槛 %.1f）".format(p.alpha, MIN_CONFIDENCE))
                    Text("信任度 = %.2f（门槛 %.1f）".format(p.trust, MIN_TRUST))
                    Text("唯一 cells：${p.uniqueCells}")
                    Text("面包屑数：${p.breadcrumbCount}")
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        AssistChip(onClick = {}, label = { Text(if (p.fresh) "FRESH" else "STALE") })
                        AssistChip(onClick = {}, label = { Text(if (p.policy) "POLICY PASS" else "POLICY FAIL") })
                    }
                }
            }
        }

        Button(
            enabled = !bus.running && step !in listOf(
                Step.CONNECTING, Step.REQUESTING, Step.WAIT_CHALLENGE,
                Step.RESPONDING, Step.POLLING, Step.VERIFYING,
            ),
            onClick = {
                // 页面级协程：切 tab 离开 Compose 时随 composition 取消（验证可重发）
                scope.launch { run() }
            },
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(if (step == Step.IDLE) "发起验证" else "重新验证")
        }
    }
}

private const val MIN_CONFIDENCE: Double = 0.1
private const val MIN_TRUST: Double = 20.0
