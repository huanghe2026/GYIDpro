package com.geoyuan.gyid.data

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import uniffi.gyid.Keypair
import uniffi.gyid.generateKeypair
import uniffi.gyid.pubkeyFromSeed

/**
 * 身份 seed 的本地加密存储（EncryptedSharedPreferences = AES256-GCM + Keystore）。
 *
 * Phase 8：create / load 骨架；UI 交互在 Phase 9 接入。
 */
class IdentityStore(context: Context) {

    private val prefs = EncryptedSharedPreferences.create(
        context,
        PREFS_NAME,
        MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build(),
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
    )

    /** 本地是否已有身份。 */
    fun exists(): Boolean = prefs.getString(KEY_SEED, null) != null

    /** 已存 seed hex；未创建返回 null。seed 永不出 UI 日志。 */
    fun seedHex(): String? = prefs.getString(KEY_SEED, null)

    /** 从 seed 推导的公钥 hex；未创建返回 null。 */
    fun pubkeyHex(): String? = seedHex()?.let { runCatching { pubkeyFromSeed(it) }.getOrNull() }

    /** 调 Rust 生成新身份并加密落盘（已存在时拒绝，避免误覆盖）。 */
    fun createIdentity(): Keypair {
        check(!exists()) { "身份已存在；如需重建请先清除应用数据" }
        val kp = generateKeypair()
        prefs.edit().putString(KEY_SEED, kp.seedHex).apply()
        return kp
    }

    companion object {
        private const val PREFS_NAME = "gyid_identity"
        private const val KEY_SEED = "seed_hex"
    }
}
