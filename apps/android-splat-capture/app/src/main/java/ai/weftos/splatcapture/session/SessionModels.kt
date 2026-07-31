package ai.weftos.splatcapture.session

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/** weft.capture.v1 session.json */
@Serializable
data class SessionJson(
    val protocol: String = "weft.capture.v1",
    val id: String,
    val device: DeviceMeta? = null,
    @SerialName("started_at") val startedAt: String,
    @SerialName("ended_at") val endedAt: String? = null,
    val camera: CameraMeta? = null,
    val coverage: CoverageMeta? = null,
    @SerialName("n_frames") val nFrames: Int = 0,
    val notes: String? = null,
)

@Serializable
data class DeviceMeta(
    val model: String? = null,
    val platform: String = "android",
    @SerialName("node_id") val nodeId: String? = null,
)

@Serializable
data class CameraMeta(
    val width: Int? = null,
    val height: Int? = null,
    @SerialName("focal_hint") val focalHint: Double? = null,
    val model: String = "pinhole",
)

@Serializable
data class CoverageMeta(
    @SerialName("n_bins") val nBins: Int,
    val filled: Int,
)

/** One poses.jsonl line. */
@Serializable
data class PoseLine(
    @SerialName("frame_id") val frameId: Long,
    @SerialName("t_ns") val tNs: Long,
    val path: String,
    @SerialName("quat_wxyz") val quatWxyz: List<Double>,
    @SerialName("coverage_bin") val coverageBin: Int? = null,
)

data class SessionSummary(
    val id: String,
    val dirName: String,
    val nFrames: Int,
    val coveragePercent: Float,
    val startedAt: String,
    val zipPath: String?,
)
