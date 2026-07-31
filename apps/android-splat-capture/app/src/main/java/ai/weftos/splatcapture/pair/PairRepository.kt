package ai.weftos.splatcapture.pair

import android.content.Context
import ai.weftos.splatcapture.edge.EdgeCore

class PairRepository(
    @Suppress("unused") context: Context,
    private val edge: EdgeCore,
) {
    fun current(): EdgeCore.HostPair? = edge.loadPair()

    fun pairFromInput(input: String, token: String?): EdgeCore.HostPair {
        val pair = edge.parsePair(input, token?.ifBlank { null })
        edge.savePair(pair)
        return pair
    }

    fun unpair() = edge.clearPair()

    fun nodeId(): String = edge.nodeIdHex
}
