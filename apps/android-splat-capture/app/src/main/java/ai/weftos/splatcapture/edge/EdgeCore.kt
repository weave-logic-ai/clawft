package ai.weftos.splatcapture.edge

import android.content.Context
import android.util.Log
import org.json.JSONObject
import java.io.File
import java.security.MessageDigest
import java.security.SecureRandom

/**
 * Android-side edge core bridge (WEFT-707).
 *
 * Production path: UniFFI `clawft_android_edge` native library (Ed25519).
 * Fallback (no NDK `.so` in this tree yet): deterministic app-private
 * node id derived from a persisted 32-byte seed (HMAC-SHA256 key handle).
 * The Rust crate is the source of truth for Ed25519; regenerate Kotlin
 * bindings with the steps in docs/guides/android-splat-capture-edge.md.
 */
class EdgeCore(context: Context) {
    private val dataDir: File =
        File(context.filesDir, "weft-edge").also { it.mkdirs() }

    val nodeIdHex: String by lazy { loadOrCreateIdentity().nodeIdHex }

    data class Identity(val nodeIdHex: String, val publicKeyHex: String)
    data class HostPair(
        val baseUrl: String,
        val token: String,
        val hostNodeId: String?,
        val pairVersion: Int,
        val pairedAtMs: Long,
    )

    fun loadOrCreateIdentity(): Identity {
        // Prefer native UniFFI when shipped.
        nativeLoadIdentity()?.let { return it }

        val f = File(dataDir, "node_identity.json")
        if (f.exists()) {
            val o = JSONObject(f.readText())
            val pub = o.getString("public_key_hex")
            return Identity(nodeIdHex = pub, publicKeyHex = pub)
        }
        val seed = ByteArray(32).also { SecureRandom().nextBytes(it) }
        // Fallback "public" id: SHA-256(seed) hex — stable across restarts,
        // replaced by Ed25519 verifying key when UniFFI is linked.
        val pub = sha256Hex(seed)
        val o = JSONObject()
            .put("seed_hex", seed.toHex())
            .put("public_key_hex", pub)
            .put("version", 1)
            .put("fallback", true)
        f.writeText(o.toString(2))
        Log.i(TAG, "Created fallback edge identity (link clawft_android_edge for Ed25519)")
        return Identity(nodeIdHex = pub, publicKeyHex = pub)
    }

    fun parsePair(input: String, tokenOverride: String?): HostPair {
        nativeParsePair(input, tokenOverride)?.let { return it }
        return KotlinPairParser.parse(input, tokenOverride)
    }

    fun savePair(pair: HostPair) {
        if (nativeSavePair(pair)) return
        val f = File(dataDir, "host_pair.json")
        val o = JSONObject()
            .put("base_url", pair.baseUrl)
            .put("token", pair.token)
            .put("host_node_id", pair.hostNodeId)
            .put("pair_version", pair.pairVersion)
            .put("paired_at_ms", pair.pairedAtMs)
        f.writeText(o.toString(2))
    }

    fun loadPair(): HostPair? {
        nativeLoadPair()?.let { return it }
        val f = File(dataDir, "host_pair.json")
        if (!f.exists()) return null
        val o = JSONObject(f.readText())
        return HostPair(
            baseUrl = o.getString("base_url"),
            token = o.optString("token", ""),
            hostNodeId = o.optString("host_node_id", null).takeIf { !it.isNullOrEmpty() },
            pairVersion = o.optInt("pair_version", 1),
            pairedAtMs = o.optLong("paired_at_ms", 0L),
        )
    }

    fun clearPair() {
        if (nativeClearPair()) return
        File(dataDir, "host_pair.json").delete()
    }

    fun dataDirPath(): String = dataDir.absolutePath

    // ── UniFFI hooks (no-ops until libclawft_android_edge.so is packaged) ──

    private fun nativeLoadIdentity(): Identity? = null
    private fun nativeParsePair(input: String, token: String?): HostPair? = null
    private fun nativeSavePair(pair: HostPair): Boolean = false
    private fun nativeLoadPair(): HostPair? = null
    private fun nativeClearPair(): Boolean = false

    companion object {
        private const val TAG = "EdgeCore"

        private fun sha256Hex(bytes: ByteArray): String {
            val d = MessageDigest.getInstance("SHA-256").digest(bytes)
            return d.toHex()
        }

        private fun ByteArray.toHex(): String =
            joinToString("") { b -> "%02x".format(b) }
    }
}

/** Mirrors Rust `pair_store::parse_pair_input` for the Kotlin fallback path. */
object KotlinPairParser {
    fun parse(input: String, tokenOverride: String?): EdgeCore.HostPair {
        val trimmed = input.trim()
        require(trimmed.isNotEmpty()) { "empty pair input" }
        val now = System.currentTimeMillis()

        if (trimmed.startsWith("weftos://pair") || trimmed.startsWith("weftos:pair")) {
            val query = trimmed.substringAfter('?', missingDelimiterValue = "")
            require(query.isNotEmpty()) { "weftos pair URL missing query" }
            var host: String? = null
            var port = "7860"
            var token = tokenOverride.orEmpty()
            var node: String? = null
            var version = 1
            var scheme = "http"
            for (part in query.split('&')) {
                val kv = part.split('=', limit = 2)
                if (kv.size != 2) continue
                val k = kv[0]
                val v = UriDecode.decode(kv[1])
                when (k) {
                    "host" -> host = v
                    "port" -> port = v
                    "token" -> if (token.isEmpty()) token = v
                    "node" -> node = v
                    "v" -> version = v.toIntOrNull() ?: 1
                    "scheme", "proto" -> scheme = v
                    "url", "base" -> {
                        return EdgeCore.HostPair(
                            baseUrl = v.trimEnd('/'),
                            token = token,
                            hostNodeId = node,
                            pairVersion = version,
                            pairedAtMs = now,
                        )
                    }
                }
            }
            require(!host.isNullOrEmpty()) { "pair QR missing host" }
            return EdgeCore.HostPair(
                baseUrl = "$scheme://$host:$port",
                token = token,
                hostNodeId = node,
                pairVersion = version,
                pairedAtMs = now,
            )
        }

        var base = if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
            trimmed
        } else {
            "http://$trimmed"
        }
        var token = tokenOverride.orEmpty()
        if ('?' in base) {
            val (head, query) = base.split('?', limit = 2)
            for (part in query.split('&')) {
                val kv = part.split('=', limit = 2)
                if (kv.size == 2 && kv[0] == "token" && token.isEmpty()) {
                    token = UriDecode.decode(kv[1])
                }
            }
            base = head
        }
        base = base.trimEnd('/')
        return EdgeCore.HostPair(
            baseUrl = base,
            token = token,
            hostNodeId = null,
            pairVersion = 1,
            pairedAtMs = now,
        )
    }
}

private object UriDecode {
    fun decode(s: String): String {
        val out = StringBuilder(s.length)
        var i = 0
        val chars = s.toCharArray()
        while (i < chars.size) {
            when (val c = chars[i]) {
                '+' -> {
                    out.append(' ')
                    i++
                }
                '%' -> if (i + 2 < chars.size) {
                    val h = hex(chars[i + 1])
                    val l = hex(chars[i + 2])
                    if (h >= 0 && l >= 0) {
                        out.append(((h shl 4) or l).toChar())
                        i += 3
                    } else {
                        out.append(c)
                        i++
                    }
                } else {
                    out.append(c)
                    i++
                }
                else -> {
                    out.append(c)
                    i++
                }
            }
        }
        return out.toString()
    }

    private fun hex(c: Char): Int = when (c) {
        in '0'..'9' -> c - '0'
        in 'a'..'f' -> c - 'a' + 10
        in 'A'..'F' -> c - 'A' + 10
        else -> -1
    }
}
