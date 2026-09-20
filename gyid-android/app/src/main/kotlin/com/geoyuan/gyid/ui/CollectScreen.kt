package com.geoyuan.gyid.ui

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.geoyuan.gyid.data.CollectBus
import com.geoyuan.gyid.data.IdentityStore
import com.geoyuan.gyid.data.SettingsStore

/**
 * 采集屏：
 * - 运行时权限：精确定位 / 通知 / 后台定位（后台定位只能引到系统设置授予）；
 * - 参数：探索模式（300s）↔ 普通模式（900s），H3 resolution 7..10；
 * - 实时状态：最近定位、H3 cell、本地/待同步计数、事件日志（来自 [CollectBus]）。
 */
@Composable
fun CollectScreen() {
    val context = LocalContext.current
    val identity = remember { IdentityStore(context) }
    val settings = remember { SettingsStore(context) }

    val bus by CollectBus.state.collectAsState()
    var exploration by remember { mutableStateOf(settings.exploration) }
    var resolution by remember { mutableStateOf(settings.h3Resolution) }
    var grantedVersion by remember { mutableStateOf(0) } // 权限回调后触发重组重查

    val fineGranted = ContextCompat.checkSelfPermission(
        context, Manifest.permission.ACCESS_FINE_LOCATION,
    ) == PackageManager.PERMISSION_GRANTED
    val bgGranted = Build.VERSION.SDK_INT < Build.VERSION_CODES.Q ||
        ContextCompat.checkSelfPermission(
            context, Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        ) == PackageManager.PERMISSION_GRANTED
    val notifGranted = Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
        ContextCompat.checkSelfPermission(
            context, Manifest.permission.POST_NOTIFICATIONS,
        ) == PackageManager.PERMISSION_GRANTED

    val locationPermLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { grantedVersion++ }
    val notifPermLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { grantedVersion++ }

    fun ensureNotifThenStart() {
        if (!notifGranted && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            notifPermLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
        val intent = Intent(context, CollectService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.startForegroundService(intent)
        } else {
            context.startService(intent)
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("面包屑采集", style = MaterialTheme.typography.headlineSmall)

        if (!identity.exists()) {
            Text("✗ 请先在「身份」页创建身份", color = MaterialTheme.colorScheme.error)
            return@Column
        }

        // ---- 权限区 ----
        if (!fineGranted) {
            PermissionCard(
                title = "需要精确定位权限",
                detail = "采集 GPS 位置与 H3 cell 用于面包屑签名。",
                actionLabel = "授予定位权限",
            ) {
                locationPermLauncher.launch(
                    arrayOf(
                        Manifest.permission.ACCESS_FINE_LOCATION,
                        Manifest.permission.ACCESS_COARSE_LOCATION,
                    ),
                )
            }
        } else if (!bgGranted) {
            PermissionCard(
                title = "建议授予「始终允许」后台定位",
                detail = "否则切到后台/息屏后系统会很快停止采集。需在系统设置中手动选择。",
                actionLabel = "打开系统设置",
            ) {
                context.startActivity(
                    Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
                        .setData(Uri.fromParts("package", context.packageName, null)),
                )
            }
        }

        // ---- 参数区（采集进行中只读）----
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column {
                        Text("探索模式", style = MaterialTheme.typography.titleSmall)
                        Text(
                            "普通 900s / 探索 300s（§4.2）",
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                    Switch(
                        enabled = !bus.running,
                        checked = exploration,
                        onCheckedChange = {
                            exploration = it
                            settings.exploration = it
                        },
                    )
                }
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text("H3 res：", style = MaterialTheme.typography.titleSmall)
                    (7..10).forEach { r ->
                        FilterChip(
                            enabled = !bus.running,
                            selected = resolution == r,
                            onClick = {
                                resolution = r
                                settings.h3Resolution = r
                            },
                            label = { Text(r.toString()) },
                        )
                    }
                }
            }
        }

        // ---- 启停 ----
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Button(
                enabled = fineGranted && !bus.running,
                onClick = { ensureNotifThenStart() },
                modifier = Modifier.weight(1f),
            ) { Text(if (fineGranted) "开始采集" else "等待定位权限") }
            OutlinedButton(
                enabled = bus.running,
                onClick = { context.stopService(Intent(context, CollectService::class.java)) },
                modifier = Modifier.weight(1f),
            ) { Text("停止采集") }
        }

        // ---- 状态区 ----
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(
                    if (bus.running) "● 采集中" else "○ 已停止",
                    style = MaterialTheme.typography.titleSmall,
                    color = if (bus.running) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text("本地面包屑：${bus.total}    待同步：${bus.pending}")
                Text("最近定位：${bus.lastFix ?: "—"}")
                Text(
                    "H3 cell：${bus.lastCell ?: "—"}",
                    fontFamily = FontFamily.Monospace,
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }

        // ---- 日志 ----
        Text("事件日志", style = MaterialTheme.typography.titleSmall)
        LazyColumn(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = 120.dp)
                .weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            items(bus.logs.reversed()) { line ->
                Text(
                    line,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                )
            }
        }
    }
}

@Composable
private fun PermissionCard(
    title: String,
    detail: String,
    actionLabel: String,
    onClick: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.errorContainer,
        ),
    ) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(detail, style = MaterialTheme.typography.bodySmall)
            Button(onClick = onClick) { Text(actionLabel) }
        }
    }
}
