package com.geoyuan.gyid.data

import android.content.Context

/**
 * 全局设置（普通 SharedPreferences，不含敏感材料）。
 *
 * - verifierUrl：trip-server 地址。模拟器访问宿主机回环用 10.0.2.2；
 *   真机用局域网 IP（如 http://192.168.x.x:8080）。
 * - h3Resolution：协议允许 7..=10。
 * - exploration：§4.2 探索模式（间隔 300s，否则 900s）。
 */
class SettingsStore(context: Context) {

    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    var verifierUrl: String
        get() = prefs.getString(KEY_VERIFIER_URL, DEFAULT_VERIFIER_URL)!!.trimEnd('/')
        set(value) = prefs.edit().putString(KEY_VERIFIER_URL, value.trimEnd('/')).apply()

    var h3Resolution: Int
        get() = prefs.getInt(KEY_RESOLUTION, 10)
        set(value) = prefs.edit().putInt(KEY_RESOLUTION, value.coerceIn(7, 10)).apply()

    var exploration: Boolean
        get() = prefs.getBoolean(KEY_EXPLORATION, false)
        set(value) = prefs.edit().putBoolean(KEY_EXPLORATION, value).apply()

    /** 当前模式下的采集间隔（秒）：普通 900s（§4.1 硬下限）/ 探索 300s（§4.2）。 */
    fun intervalSecs(): Int = if (exploration) EXPLORE_INTERVAL else NORMAL_INTERVAL

    companion object {
        private const val PREFS = "gyid_settings"
        private const val KEY_VERIFIER_URL = "verifier_url"
        private const val KEY_RESOLUTION = "h3_resolution"
        private const val KEY_EXPLORATION = "exploration"

        const val NORMAL_INTERVAL = 900
        const val EXPLORE_INTERVAL = 300

        /** 模拟器 → 宿主机 loopback 的固定别名。 */
        const val DEFAULT_VERIFIER_URL = "http://10.0.2.2:8080"
    }
}
