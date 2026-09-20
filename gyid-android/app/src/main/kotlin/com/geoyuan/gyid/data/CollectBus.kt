package com.geoyuan.gyid.data

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.update

/**
 * CollectService ↔ CollectScreen 的同进程状态总线。
 *
 * 服务只有一个实例；UI 直接 collect [state]。日志环形保留最近 100 条。
 */
object CollectBus {

    data class State(
        val running: Boolean = false,
        val total: Long = 0,
        val pending: Int = 0,
        val intervalSecs: Int = SettingsStore.NORMAL_INTERVAL,
        val lastFix: String? = null,
        val lastCell: String? = null,
        val logs: List<String> = emptyList(),
    )

    private val _state = MutableStateFlow(State())
    val state: StateFlow<State> = _state

    fun reset(intervalSecs: Int) {
        _state.value = State(running = true, intervalSecs = intervalSecs)
    }

    fun update(
        total: Long? = null,
        pending: Int? = null,
        lastFix: String? = null,
        lastCell: String? = null,
    ) {
        _state.update {
            it.copy(
                total = total ?: it.total,
                pending = pending ?: it.pending,
                lastFix = lastFix ?: it.lastFix,
                lastCell = lastCell ?: it.lastCell,
            )
        }
    }

    fun log(msg: String) {
        val t = java.text.SimpleDateFormat("HH:mm:ss", java.util.Locale.getDefault())
            .format(java.util.Date())
        _state.update {
            it.copy(logs = (it.logs + "[$t] $msg").takeLast(100))
        }
    }

    fun stop() {
        _state.update { it.copy(running = false) }
    }
}
