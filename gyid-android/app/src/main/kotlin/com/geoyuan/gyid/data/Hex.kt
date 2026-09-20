package com.geoyuan.gyid.data

/** 小写 hex → 字节数组（奇数长度左侧补 0，与 Web hexToBytes 行为一致）。 */
fun String.hexToBytes(): ByteArray {
    val clean = if (length % 2 == 1) "0$this" else this
    return ByteArray(clean.length / 2) { i ->
        clean.substring(i * 2, i * 2 + 2).toInt(16).toByte()
    }
}

/** 字节数组 → 小写 hex。 */
fun ByteArray.toHex(): String =
    joinToString("") { "%02x".format(it.toInt() and 0xff) }
