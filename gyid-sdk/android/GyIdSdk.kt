package io.gyid.sdk

import org.json.JSONObject
import org.json.JSONArray

/**
 * GyID SDK - Android Kotlin 封装
 *
 * 通过 JNI 调用 Rust 原生库，提供类型安全的 Kotlin API。
 *
 * 使用方法:
 * ```kotlin
 * // 生成 GyID
 * val result = GyIdSdk.generateDefault()
 * println("GyID: ${result.id}")
 *
 * // 验证 GyID
 * val valid = GyIdSdk.verify(result.id)
 * ```
 */
object GyIdSdk {

    init {
        System.loadLibrary("gyid_sdk")
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 数据类
    // ─────────────────────────────────────────────────────────────────────────

    data class GyIdResult(
        val id: String,
        val hash: String,
        val createdAtMs: Long,
        val linkedDevices: Int,
        val platform: String,
        val sdkVersion: String
    ) {
        companion object {
            fun fromJson(json: String): GyIdResult {
                val obj = JSONObject(json)
                return GyIdResult(
                    id = obj.getString("id"),
                    hash = obj.getString("hash"),
                    createdAtMs = obj.getLong("created_at_ms"),
                    linkedDevices = obj.getInt("linked_devices"),
                    platform = obj.getString("platform"),
                    sdkVersion = obj.getString("sdk_version")
                )
            }
        }
    }

    data class GenerateOptions(
        val geoLevel: String = "city",        // "city" | "district" | "exact"
        val withGeo: Boolean = true,
        val avatarPath: String? = null
    ) {
        fun toJson(): String = buildString {
            append("""{"geo_level":"$geoLevel","with_geo":$withGeo""")
            avatarPath?.let { append(""","avatar_path":"$it"""") }
            append("}")
        }
    }

    data class DeviceInfo(
        val linkedGyid: String,
        val linkId: String,
        val linkedAtMs: Long,
        val active: Boolean
    )

    data class DeviceList(
        val masterGyid: String,
        val devices: List<DeviceInfo>
    ) {
        companion object {
            fun fromJson(json: String): DeviceList {
                val obj = JSONObject(json)
                val devicesArr = obj.getJSONArray("devices")
                val devices = (0 until devicesArr.length()).map { i ->
                    val d = devicesArr.getJSONObject(i)
                    DeviceInfo(
                        linkedGyid = d.getString("linked_gyid"),
                        linkId = d.getString("link_id"),
                        linkedAtMs = d.getLong("linked_at_ms"),
                        active = d.getBoolean("active")
                    )
                }
                return DeviceList(
                    masterGyid = obj.getString("master_gyid"),
                    devices = devices
                )
            }
        }
    }

    data class SdkVersion(
        val version: String,
        val platform: String,
        val features: List<String>
    ) {
        companion object {
            fun fromJson(json: String): SdkVersion {
                val obj = JSONObject(json)
                val featuresArr = obj.getJSONArray("features")
                val features = (0 until featuresArr.length()).map { i ->
                    featuresArr.getString(i)
                }
                return SdkVersion(
                    version = obj.getString("version"),
                    platform = obj.getString("platform"),
                    features = features
                )
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 公开 API（在线程上异步调用，避免阻塞主线程）
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * 使用默认配置生成 GyID（阻塞，需在协程/后台线程调用）
     */
    @Throws(GyIdException::class)
    fun generateDefault(): GyIdResult {
        val json = GyIdLib.generateDefault()
        checkError(json)
        return GyIdResult.fromJson(json)
    }

    /**
     * 使用自定义选项生成 GyID（阻塞，需在协程/后台线程调用）
     */
    @Throws(GyIdException::class)
    fun generate(options: GenerateOptions = GenerateOptions()): GyIdResult {
        val json = GyIdLib.generate(options.toJson())
        checkError(json)
        return GyIdResult.fromJson(json)
    }

    /**
     * 验证 GyID 格式（线程安全）
     */
    fun verify(gyid: String): Boolean = GyIdLib.verify(gyid)

    /**
     * 获取关联设备列表（阻塞）
     */
    @Throws(GyIdException::class)
    fun getDevices(masterGyid: String? = null): DeviceList {
        val json = GyIdLib.getDevices(masterGyid ?: "")
        checkError(json)
        return DeviceList.fromJson(json)
    }

    /**
     * 获取 SDK 版本信息（线程安全）
     */
    fun version(): SdkVersion = SdkVersion.fromJson(GyIdLib.getVersion())

    // ─────────────────────────────────────────────────────────────────────────
    // 内部 JNI 接口
    // ─────────────────────────────────────────────────────────────────────────

    private object GyIdLib {
        external fun generateDefault(): String
        external fun generate(optionsJson: String): String
        external fun verify(gyid: String): Boolean
        external fun getDevices(masterGyid: String): String
        external fun getVersion(): String
    }

    private fun checkError(json: String) {
        try {
            val obj = JSONObject(json)
            if (obj.has("error")) {
                throw GyIdException(obj.getString("error"))
            }
        } catch (e: org.json.JSONException) {
            // 非错误 JSON，忽略
        }
    }
}

/**
 * GyID SDK 异常类
 */
class GyIdException(message: String) : Exception(message)
