package com.geoyuan.gyid.data

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/**
 * 本地面包屑链（按 pubkey 隔离，filesDir 下 JSON 文件持久化）。
 *
 * 与 Web chain store 对齐：
 * - nextIndex：下一条的链内序号（创世为 0）；
 * - prevHash：链尾 block_hash hex（创世为 null）；
 * - pending：已签名但尚未确认上传的 CBOR hex 帧（失败保留、断点续传）；
 * - total：累计采集条数（含已上传，通知栏展示用）。
 *
 * 证据帧流 = 单帧 CBOR 顺序拼接（服务端 Chain::from_cbor_stream 解析）。
 */
class ChainStore(context: Context) {

    private val dir = File(context.filesDir, "chains").apply { mkdirs() }

    private data class State(
        var nextIndex: Long,
        var prevHash: String?,
        val pending: MutableList<String>,
        var total: Long,
    )

    private fun fileFor(pubkey: String) = File(dir, "$pubkey.json")

    private fun load(pubkey: String): State {
        val f = fileFor(pubkey)
        if (!f.exists()) return State(0, null, mutableListOf(), 0)
        val obj = JSONObject(f.readText())
        val arr = obj.getJSONArray("pending")
        val pending = MutableList(arr.length()) { arr.getString(it) }
        return State(
            nextIndex = obj.getLong("next_index"),
            prevHash = if (obj.isNull("prev_hash")) null else obj.getString("prev_hash"),
            pending = pending,
            total = obj.getLong("total"),
        )
    }

    private fun save(pubkey: String, s: State) {
        val obj = JSONObject()
            .put("next_index", s.nextIndex)
            .put("prev_hash", s.prevHash ?: JSONObject.NULL)
            .put("total", s.total)
            .put("pending", JSONArray(s.pending as List<*>))
        fileFor(pubkey).writeText(obj.toString())
    }

    /** 追加一条已签名面包屑帧，返回续链所需的新状态参数（通知/日志用）。 */
    @Synchronized
    fun append(pubkey: String, crumbHex: String, newBlockHash: String): AppendResult {
        val s = load(pubkey)
        val index = s.nextIndex
        s.pending.add(crumbHex)
        s.nextIndex += 1
        s.total += 1
        s.prevHash = newBlockHash
        save(pubkey, s)
        return AppendResult(index = index, total = s.total, pendingCount = s.pending.size)
    }

    /**
     * 取下一批待上传证据（最多 [maxBatch] 条）。
     * @return 帧 hex 列表（空 = 全部已同步）；调用方上传后须调 [markUploaded]。
     */
    @Synchronized
    fun nextBatch(pubkey: String, maxBatch: Int): List<String> {
        val s = load(pubkey)
        return s.pending.take(maxBatch)
    }

    /** [nextBatch] 对应批次上传成功后出队。 */
    @Synchronized
    fun markUploaded(pubkey: String, count: Int) {
        val s = load(pubkey)
        repeat(count.coerceAtMost(s.pending.size)) { s.pending.removeAt(0) }
        save(pubkey, s)
    }

    @Synchronized
    fun snapshot(pubkey: String): ChainSnapshot {
        val s = load(pubkey)
        return ChainSnapshot(
            nextIndex = s.nextIndex,
            total = s.total,
            pendingCount = s.pending.size,
            prevHash = s.prevHash,
        )
    }

    @Synchronized
    fun reset(pubkey: String) {
        fileFor(pubkey).delete()
    }

    /** 拼接帧列表为单批 evidence body（CBOR 帧流）。 */
    fun concatFrames(frames: List<String>): ByteArray =
        frames.joinToString("").hexToBytes()

    data class AppendResult(val index: Long, val total: Long, val pendingCount: Int)

    data class ChainSnapshot(
        val nextIndex: Long,
        val total: Long,
        val pendingCount: Int,
        val prevHash: String?,
    )

    companion object {
        /** POST /v1/evidence 单批上限（draft-04 §5）。 */
        const val MAX_BATCH = 300
    }
}
