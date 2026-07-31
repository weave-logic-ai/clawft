package ai.weftos.splatcapture.upload

import ai.weftos.splatcapture.edge.EdgeCore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import org.json.JSONObject
import java.io.File
import java.util.concurrent.TimeUnit

data class JobCreateResult(val jobId: String)

data class JobPollResult(
    val jobId: String,
    val status: String,
    val stage: String,
    val sogUrl: String,
    val rawJson: String,
)

/**
 * HTTPS client for splatd (A2). Mirrors `clawft-android-edge` upload surface
 * so Kotlin can run without the native lib; native path preferred when linked.
 */
class SplatdClient(
    private val client: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.MINUTES)
        .writeTimeout(10, TimeUnit.MINUTES)
        .build(),
) {
    suspend fun health(pair: EdgeCore.HostPair): String = withContext(Dispatchers.IO) {
        val req = requestBuilder(pair, "${pair.baseUrl.trimEnd('/')}/healthz").get().build()
        client.newCall(req).execute().use { resp ->
            val body = resp.body?.string().orEmpty()
            if (!resp.isSuccessful) error("healthz HTTP ${resp.code}: $body")
            body
        }
    }

    suspend fun uploadSessionZip(pair: EdgeCore.HostPair, zip: File): JobCreateResult =
        withContext(Dispatchers.IO) {
            require(zip.isFile) { "zip missing: ${zip.absolutePath}" }
            val body = MultipartBody.Builder()
                .setType(MultipartBody.FORM)
                .addFormDataPart(
                    "session",
                    zip.name,
                    zip.asRequestBody("application/zip".toMediaType()),
                )
                .build()
            val req = requestBuilder(pair, "${pair.baseUrl.trimEnd('/')}/v1/jobs")
                .post(body)
                .build()
            client.newCall(req).execute().use { resp ->
                val text = resp.body?.string().orEmpty()
                if (resp.code != 202 && !resp.isSuccessful) {
                    error("POST /v1/jobs HTTP ${resp.code}: $text")
                }
                val jobId = JSONObject(text).getString("job_id")
                JobCreateResult(jobId)
            }
        }

    suspend fun getJob(pair: EdgeCore.HostPair, jobId: String): JobPollResult =
        withContext(Dispatchers.IO) {
            val url = "${pair.baseUrl.trimEnd('/')}/v1/jobs/$jobId"
            val req = requestBuilder(pair, url).get().build()
            client.newCall(req).execute().use { resp ->
                val text = resp.body?.string().orEmpty()
                if (!resp.isSuccessful) error("GET job HTTP ${resp.code}: $text")
                val o = JSONObject(text)
                val status = o.optString("status", "unknown")
                val stage = o.optString("stage", "")
                val sog = extractSogUrl(pair, o, jobId)
                JobPollResult(
                    jobId = jobId,
                    status = status,
                    stage = stage,
                    sogUrl = sog,
                    rawJson = text,
                )
            }
        }

    private fun requestBuilder(pair: EdgeCore.HostPair, url: String): Request.Builder {
        val b = Request.Builder().url(url)
        if (pair.token.isNotEmpty()) {
            b.header("Authorization", "Bearer ${pair.token}")
        }
        return b
    }

    private fun extractSogUrl(pair: EdgeCore.HostPair, o: JSONObject, jobId: String): String {
        for (key in listOf("sog_url", "result_url", "viewer_url")) {
            val s = o.optString(key, "")
            if (s.isNotEmpty()) return s
        }
        val arts = o.optJSONObject("artifacts")
        if (arts != null) {
            val names = listOf("scene.sog", "output.sog", "sog", "result.sog")
            for (n in names) {
                if (arts.has(n)) {
                    return "${pair.baseUrl.trimEnd('/')}/v1/jobs/$jobId/artifacts/$n"
                }
            }
            val keys = arts.keys()
            while (keys.hasNext()) {
                val k = keys.next()
                if (k.endsWith(".sog")) {
                    return "${pair.baseUrl.trimEnd('/')}/v1/jobs/$jobId/artifacts/$k"
                }
            }
        }
        if (o.optString("status") in listOf("done", "completed")) {
            return "${pair.baseUrl.trimEnd('/')}/v1/jobs/$jobId/artifacts/scene.sog"
        }
        return ""
    }
}
