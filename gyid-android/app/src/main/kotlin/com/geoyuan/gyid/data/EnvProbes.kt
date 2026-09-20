package com.geoyuan.gyid.data

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.net.wifi.WifiManager
import java.security.MessageDigest

/**
 * 环境分量探针（draft-04 §2.2 context digest 附加材料）：
 * - Wi-Fi：[bestBssidHex] 取缓存扫描结果中信号最强 AP 的 BSSID（6 字节 MAC hex）。
 *   startScan 在 Android 9+ 被严格节流，故不依赖即时扫描，仅用系统缓存。
 * - IMU：[ImuProbe] 持续监听加速度计 + 陀螺仪，采集时把最新向量串 SHA-256。
 *
 * 两者都是 best-effort：无权限 / 无硬件 / 无读数时返回 null，面包屑仍可签。
 */
object WifiProbe {

    /** 信号最强 AP 的 BSSID hex（12 字符小写）；无缓存结果返回 null。 */
    fun bestBssidHex(context: Context): String? {
        val wifi = context.applicationContext
            .getSystemService(Context.WIFI_SERVICE) as? WifiManager ?: return null
        return runCatching {
            // 触发一次异步扫描（可能被节流，结果体现在后续 getScanResults）
            @Suppress("MissingPermission")
            wifi.startScan()
            wifi.scanResults
                .filter { it.BSSID != null && it.BSSID.matches(Regex("([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}")) }
                // 过滤随机化/全零 MAC；按信号强度降序
                .sortedByDescending { it.level }
                .firstOrNull()
                ?.BSSID
                ?.replace(":", "")
                ?.lowercase()
        }.getOrNull()
    }
}

/**
 * IMU 探针：注册后持续缓存最新加速度 / 角速度，采集时生成
 * SHA-256("ax,ay,az|gx,gy,gz")（64 hex）。服务存活期间注册，销毁时注销。
 */
class ImuProbe(context: Context) : SensorEventListener {

    private val sm = context.getSystemService(Context.SENSOR_SERVICE) as SensorManager
    private val accel = sm.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)
    private val gyro = sm.getDefaultSensor(Sensor.TYPE_GYROSCOPE)

    @Volatile private var accelVec: FloatArray? = null
    @Volatile private var gyroVec: FloatArray? = null

    fun start() {
        accel?.let { sm.registerListener(this, it, SensorManager.SENSOR_DELAY_NORMAL) }
        gyro?.let { sm.registerListener(this, it, SensorManager.SENSOR_DELAY_NORMAL) }
    }

    fun stop() = sm.unregisterListener(this)

    /** 当前 IMU 摘要 hex；两类传感器都尚无读数时返回 null。 */
    fun digestHex(): String? {
        val a = accelVec ?: return null
        val g = gyroVec ?: return null
        fun f(x: Float) = "%.4f".format(x)
        val vec = "${f(a[0])},${f(a[1])},${f(a[2])}|${f(g[0])},${f(g[1])},${f(g[2])}"
        val md = MessageDigest.getInstance("SHA-256")
        return md.digest(vec.toByteArray()).toHex()
    }

    override fun onSensorChanged(event: SensorEvent) {
        when (event.sensor.type) {
            Sensor.TYPE_ACCELEROMETER -> accelVec = event.values.copyOf()
            Sensor.TYPE_GYROSCOPE -> gyroVec = event.values.copyOf()
        }
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) = Unit
}
