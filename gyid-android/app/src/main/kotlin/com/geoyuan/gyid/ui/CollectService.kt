package com.geoyuan.gyid.ui

import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.location.Location
import android.os.Build
import android.os.IBinder
import android.os.Looper
import com.geoyuan.gyid.R
import com.geoyuan.gyid.data.ChainStore
import com.geoyuan.gyid.data.CollectBus
import com.geoyuan.gyid.data.IdentityStore
import com.geoyuan.gyid.data.ImuProbe
import com.geoyuan.gyid.data.SettingsStore
import com.geoyuan.gyid.data.VerifierApi
import com.geoyuan.gyid.data.WifiProbe
import com.google.android.gms.location.LocationCallback
import com.google.android.gms.location.LocationRequest
import com.google.android.gms.location.LocationResult
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import uniffi.gyid.breadcrumbBlockHash
import uniffi.gyid.h3ToCell
import uniffi.gyid.signBreadcrumb

/**
 * 持续位置采集前台服务（foregroundServiceType=location）。
 *
 * 流程（draft-04 §4）：
 *   FusedLocation（900s / 探索 300s）→ 间隔闸门（创世块除外）→
 *   WiFi BSSID + IMU 摘要 → signBreadcrumb（index/prev 本地续链）→
 *   ChainStore 持久化 → POST /v1/evidence（失败保留、下次续传）→
 *   通知栏 + [CollectBus] 更新 breadcrumb_count。
 */
class CollectService : Service() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private lateinit var fused: com.google.android.gms.location.FusedLocationProviderClient
    private var locationCallback: LocationCallback? = null
    private var imu: ImuProbe? = null

    private lateinit var identity: IdentityStore
    private lateinit var settings: SettingsStore
    private lateinit var chains: ChainStore
    private lateinit var verifier: VerifierApi

    @Volatile private var lastCollectSec: Long = 0

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        identity = IdentityStore(this)
        settings = SettingsStore(this)
        chains = ChainStore(this)
        verifier = VerifierApi(settings.verifierUrl)
        fused = LocationServices.getFusedLocationProviderClient(this)
        createChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        startAsForeground(0)
        val seed = identity.seedHex()
        val pubkey = identity.pubkeyHex()
        if (seed == null || pubkey == null) {
            CollectBus.log("✗ 本地无身份，停止采集")
            stopSelf()
            return START_NOT_STICKY
        }
        lastCollectSec = prefs().getLong(lastTsKey(pubkey), 0L)

        CollectBus.reset(settings.intervalSecs())
        val snap = chains.snapshot(pubkey)
        CollectBus.update(total = snap.total, pending = snap.pendingCount)
        CollectBus.log("采集启动：间隔 ${settings.intervalSecs()}s，res=${settings.h3Resolution}，verifier=${settings.verifierUrl}")
        if (snap.pendingCount > 0) CollectBus.log("有 ${snap.pendingCount} 条待续传")

        // IMU 持续监听（采集时取最新向量）
        imu = ImuProbe(this).also { it.start() }

        requestLocationUpdates()
        // 启动先补传一次历史未同步证据
        scope.launch { uploadPending(pubkey) }
        return START_STICKY
    }

    @SuppressLint("MissingPermission") // 权限由 CollectScreen 在启动前授予
    private fun requestLocationUpdates() {
        val intervalMs = settings.intervalSecs() * 1000L
        val request = LocationRequest.Builder(intervalMs)
            .setMinUpdateIntervalMillis(intervalMs / 2)
            .setPriority(Priority.PRIORITY_HIGH_ACCURACY)
            .build()
        val cb = object : LocationCallback() {
            override fun onLocationResult(result: LocationResult) {
                result.lastLocation?.let { onFix(it) }
            }
        }
        locationCallback = cb
        fused.requestLocationUpdates(request, cb, Looper.getMainLooper())
    }

    private fun onFix(loc: Location) {
        val pubkey = identity.pubkeyHex() ?: return
        val seed = identity.seedHex() ?: return
        val nowSec = System.currentTimeMillis() / 1000

        // 间隔闸门：非创世块须满足 §4.1 900s / §4.2 300s 硬下限
        val snap = chains.snapshot(pubkey)
        if (snap.total > 0 && nowSec - lastCollectSec < settings.intervalSecs()) return

        val fixText = "%.5f, %.5f ±%.0fm".format(loc.latitude, loc.longitude, loc.accuracy)
        CollectBus.update(lastFix = fixText)

        scope.launch {
            try {
                val cellHex = h3ToCell(loc.latitude, loc.longitude, settings.h3Resolution.toUByte())
                val wifi = runCatching { WifiProbe.bestBssidHex(this@CollectService) }.getOrNull()
                val imuHex = runCatching { imu?.digestHex() }.getOrNull()

                val crumbHex = signBreadcrumb(
                    seed,
                    snap.nextIndex.toULong(),
                    nowSec.toULong(),
                    loc.latitude,
                    loc.longitude,
                    settings.h3Resolution.toUByte(),
                    snap.prevHash,
                    settings.exploration,
                    wifi,
                    imuHex,
                )
                val blockHash = breadcrumbBlockHash(crumbHex)
                val appended = chains.append(pubkey, crumbHex, blockHash)
                lastCollectSec = nowSec
                prefs().edit().putLong(lastTsKey(pubkey), nowSec).apply()

                CollectBus.update(
                    total = appended.total,
                    pending = appended.pendingCount,
                    lastCell = cellHex,
                )
                CollectBus.log(
                    "✓ #${appended.index} 已签名 cell=${cellHex.take(12)}…" +
                        " (wifi=${wifi != null}, imu=${imuHex != null})",
                )
                startAsForeground(appended.total)
                uploadPending(pubkey)
            } catch (e: Exception) {
                CollectBus.log("✗ 采集失败：${e.message}")
            }
        }
    }

    /** 续传全部待同步面包屑（单批 ≤ [ChainStore.MAX_BATCH]，顺序提交）。 */
    private suspend fun uploadPending(pubkey: String) {
        while (true) {
            val batch = chains.nextBatch(pubkey, ChainStore.MAX_BATCH)
            if (batch.isEmpty()) return
            try {
                val info = verifier.uploadEvidence(chains.concatFrames(batch))
                chains.markUploaded(pubkey, batch.size)
                val snap = chains.snapshot(pubkey)
                CollectBus.update(total = snap.total, pending = 0)
                CollectBus.log("✓ 上传 ${batch.size} 条（服务端 stored=${info.stored}）")
            } catch (e: Exception) {
                val snap = chains.snapshot(pubkey)
                CollectBus.update(pending = snap.pendingCount)
                CollectBus.log("✗ 上传失败：${e.message}（本地保留，下次续传）")
                return
            }
        }
    }

    override fun onDestroy() {
        locationCallback?.let { fused.removeLocationUpdates(it) }
        locationCallback = null
        imu?.stop()
        imu = null
        CollectBus.log("采集已停止")
        CollectBus.stop()
        super.onDestroy()
    }

    private fun prefs() = getSharedPreferences("gyid_collect", Context.MODE_PRIVATE)
    private fun lastTsKey(pubkey: String) = "last_ts_$pubkey"

    private fun createChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                getString(R.string.collect_channel_name),
                NotificationManager.IMPORTANCE_LOW,
            )
            getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
        }
    }

    private fun startAsForeground(count: Long) {
        val notification = buildNotification(count)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION,
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun buildNotification(count: Long): Notification {
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        return builder
            .setContentTitle(getString(R.string.collect_notification_title))
            .setContentText("面包屑 #$count · 间隔 ${settings.takeIf { ::settings.isInitialized }?.intervalSecs() ?: 900}s")
            .setSmallIcon(R.drawable.ic_launcher)
            .setOngoing(true)
            .build()
    }

    companion object {
        private const val CHANNEL_ID = "gyid_collect"
        private const val NOTIFICATION_ID = 1001
    }
}
