package com.geoyuan.gyid.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.geoyuan.gyid.data.IdentityStore
import com.geoyuan.gyid.data.SettingsStore
import com.geoyuan.gyid.data.VerifierApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * 身份屏：
 * - 生成身份后强制展示 64-hex seed（Keystore 加密落盘，仅本次明文展示），
 *   警告离线抄写：设备丢失/清数据即永久不可恢复（draft §3.1 密钥托管警示）；
 * - pubkey / verifier 地址配置；
 * - GET /v1/identity 展示服务端可见统计（404 = 尚无证据，空态）。
 */
@Composable
fun IdentityScreen() {
    val context = LocalContext.current
    val store = remember { IdentityStore(context) }
    val settings = remember { SettingsStore(context) }
    val clipboard = LocalClipboardManager.current

    var hasIdentity by remember { mutableStateOf(store.exists()) }
    // 仅刚生成的当次保留明文 seed；重组/重进页面后不再可读
    var freshSeed by remember { mutableStateOf<String?>(null) }
    var pubkey by remember { mutableStateOf(store.pubkeyHex()) }
    var verifierUrl by remember { mutableStateOf(settings.verifierUrl) }
    var stats by remember { mutableStateOf<VerifierApi.IdentityInfo?>(null) }
    var statsLoading by remember { mutableStateOf(false) }
    var statsError by remember { mutableStateOf<String?>(null) }
    var statsVersion by remember { mutableStateOf(0) }

    LaunchedEffect(hasIdentity, verifierUrl, statsVersion) {
        val pk = pubkey ?: return@LaunchedEffect
        statsLoading = true
        statsError = null
        val result = withContext(Dispatchers.IO) {
            runCatching { VerifierApi(verifierUrl).fetchIdentity(pk) }
        }
        statsLoading = false
        result.onSuccess { stats = it }
            .onFailure { statsError = it.message ?: "请求失败" }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("GyID 身份", style = MaterialTheme.typography.headlineSmall)

        // ---- 无身份：生成 ----
        if (!hasIdentity) {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.errorContainer,
                ),
            ) {
                Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "身份 seed 是设备主密钥的唯一恢复材料，仅由 Android Keystore 加密保存在本机；" +
                            "卸载/清数据/换机后无法找回，请在生成后立即离线抄写保存。",
                        style = MaterialTheme.typography.bodySmall,
                    )
                    Button(onClick = {
                        runCatching { store.createIdentity() }
                            .onSuccess { kp ->
                                freshSeed = kp.seedHex
                                pubkey = kp.pubkeyHex
                                hasIdentity = true
                            }
                    }) { Text("我已知晓，生成身份") }
                }
            }
        }

        // ---- 刚生成：seed 明文抄写 ----
        freshSeed?.let { seed ->
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                ),
            ) {
                Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("⚠ 立即抄写并离线保存（本页离开后不再显示）")
                    SelectionContainer {
                        Text(
                            seed,
                            fontFamily = FontFamily.Monospace,
                            style = MaterialTheme.typography.bodyMedium,
                        )
                    }
                    OutlinedButton(onClick = {
                        clipboard.setText(AnnotatedString(seed))
                    }) { Text("复制 seed") }
                    OutlinedButton(onClick = { freshSeed = null }) { Text("已抄写，隐藏") }
                }
            }
        }

        // ---- pubkey ----
        pubkey?.let { pk ->
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Attester 公钥（hex）", style = MaterialTheme.typography.titleSmall)
                    SelectionContainer {
                        Text(
                            pk,
                            fontFamily = FontFamily.Monospace,
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                    OutlinedButton(onClick = { clipboard.setText(AnnotatedString(pk)) }) {
                        Text("复制公钥")
                    }
                }
            }
        }

        // ---- Verifier 地址 ----
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Verifier 地址", style = MaterialTheme.typography.titleSmall)
                OutlinedTextField(
                    value = verifierUrl,
                    onValueChange = {
                        verifierUrl = it.trimEnd('/')
                        settings.verifierUrl = verifierUrl
                    },
                    singleLine = true,
                    supportingText = {
                        Text("模拟器默认 http://10.0.2.2:8080；真机填局域网 IP")
                    },
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }

        // ---- 服务端身份统计 ----
        if (hasIdentity) {
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text("服务端可见统计", style = MaterialTheme.typography.titleSmall)
                    when {
                        statsLoading -> CircularProgressIndicator()
                        statsError != null -> {
                            Text(
                                "拉取失败：$statsError",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.error,
                            )
                            OutlinedButton(onClick = { statsVersion++ }) { Text("重试") }
                        }
                        stats == null -> Text(
                            "服务端尚无该身份的证据（404）。开始采集并上传面包屑后出现。",
                            style = MaterialTheme.typography.bodySmall,
                        )
                        else -> {
                            val s = stats!!
                            Text("面包屑数：${s.breadcrumbCount}")
                            Text("唯一 H3 cells：${s.uniqueCells}")
                            Text(
                                "链头：${s.chainHead.take(16)}…",
                                fontFamily = FontFamily.Monospace,
                                style = MaterialTheme.typography.bodySmall,
                            )
                            Text(
                                "最近面包屑：${formatTs(s.lastTs)}",
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                    OutlinedButton(onClick = { statsVersion++ }) { Text("刷新") }
                }
            }
        }
    }
}

private fun formatTs(unixSec: Long): String =
    SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.getDefault()).format(Date(unixSec * 1000))
