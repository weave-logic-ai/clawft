package ai.weftos.splatcapture.session

import android.content.Context
import android.os.Build
import ai.weftos.splatcapture.capture.ImuPoseProvider
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.BufferedWriter
import java.io.File
import java.io.FileOutputStream
import java.time.Instant
import java.util.UUID
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/**
 * App-private weft.capture.v1 session store:
 * ```
 * files/sessions/session-<uuid>/
 *   session.json
 *   poses.jsonl
 *   frames/000001.jpg
 * ```
 */
class SessionRepository(context: Context) {
    private val root = File(context.filesDir, "sessions").also { it.mkdirs() }
    private val json = Json {
        prettyPrint = true
        encodeDefaults = true
        ignoreUnknownKeys = true
    }

    fun listSessions(): List<SessionSummary> {
        return root.listFiles()
            ?.filter { it.isDirectory && it.name.startsWith("session-") }
            ?.mapNotNull { dir -> summarize(dir) }
            ?.sortedByDescending { it.startedAt }
            .orEmpty()
    }

    fun sessionDir(id: String): File = File(root, "session-$id")

    fun startSession(nodeId: String?): ActiveSession {
        val id = UUID.randomUUID().toString()
        val dir = sessionDir(id)
        val frames = File(dir, "frames").also { it.mkdirs() }
        val poses = File(dir, "poses.jsonl")
        poses.writeText("")
        val started = Instant.now().toString()
        val meta = SessionJson(
            id = id,
            device = DeviceMeta(
                model = listOfNotNull(Build.MANUFACTURER, Build.MODEL).joinToString(" "),
                platform = "android",
                nodeId = nodeId,
            ),
            startedAt = started,
            camera = CameraMeta(width = 1080, height = 1920, model = "pinhole"),
            coverage = CoverageMeta(nBins = 128, filled = 0),
            nFrames = 0,
        )
        File(dir, "session.json").writeText(json.encodeToString(meta))
        return ActiveSession(
            id = id,
            dir = dir,
            framesDir = frames,
            posesFile = poses,
            startedAt = started,
            nodeId = nodeId,
            json = json,
        )
    }

    fun exportZip(sessionId: String): File {
        val dir = sessionDir(sessionId)
        require(dir.isDirectory) { "session not found: $sessionId" }
        val out = File(root, "session-$sessionId.zip")
        ZipOutputStream(FileOutputStream(out)).use { zos ->
            dir.walkTopDown().filter { it.isFile }.forEach { file ->
                val rel = file.relativeTo(dir).path.replace('\\', '/')
                zos.putNextEntry(ZipEntry(rel))
                file.inputStream().use { it.copyTo(zos) }
                zos.closeEntry()
            }
        }
        return out
    }

    private fun summarize(dir: File): SessionSummary? {
        val sessionFile = File(dir, "session.json")
        if (!sessionFile.exists()) return null
        return try {
            val meta = json.decodeFromString<SessionJson>(sessionFile.readText())
            val zip = File(root, "${dir.name}.zip")
            val cov = meta.coverage
            val pct = if (cov != null && cov.nBins > 0) {
                cov.filled.toFloat() / cov.nBins * 100f
            } else 0f
            SessionSummary(
                id = meta.id,
                dirName = dir.name,
                nFrames = meta.nFrames,
                coveragePercent = pct,
                startedAt = meta.startedAt,
                zipPath = zip.takeIf { it.exists() }?.absolutePath,
            )
        } catch (_: Exception) {
            null
        }
    }
}

class ActiveSession internal constructor(
    val id: String,
    val dir: File,
    val framesDir: File,
    private val posesFile: File,
    val startedAt: String,
    private val nodeId: String?,
    private val json: Json,
) {
    private var frameId: Long = 0
    private var posesWriter: BufferedWriter = posesFile.bufferedWriter()

    val nFrames: Long get() = frameId

    @Synchronized
    fun addFrameJpeg(jpeg: ByteArray, pose: ImuPoseProvider.Pose, width: Int, height: Int) {
        frameId += 1
        val name = "%06d.jpg".format(frameId)
        val file = File(framesDir, name)
        file.writeBytes(jpeg)
        val line = PoseLine(
            frameId = frameId,
            tNs = pose.tNs,
            path = "frames/$name",
            quatWxyz = pose.quatWxyz.map { it.toDouble() },
            coverageBin = pose.coverageBin,
        )
        posesWriter.appendLine(json.encodeToString(line))
        posesWriter.flush()
        // lightweight session.json refresh
        writeSessionJson(
            endedAt = null,
            nFrames = frameId.toInt(),
            coverageFilled = null,
            width = width,
            height = height,
        )
    }

    @Synchronized
    fun finish(coverageNBins: Int, coverageFilled: Int) {
        posesWriter.flush()
        posesWriter.close()
        writeSessionJson(
            endedAt = Instant.now().toString(),
            nFrames = frameId.toInt(),
            coverageFilled = coverageFilled,
            coverageNBins = coverageNBins,
            width = null,
            height = null,
        )
    }

    private fun writeSessionJson(
        endedAt: String?,
        nFrames: Int,
        coverageFilled: Int?,
        width: Int?,
        height: Int?,
        coverageNBins: Int = 128,
    ) {
        val existing = try {
            json.decodeFromString<SessionJson>(File(dir, "session.json").readText())
        } catch (_: Exception) {
            SessionJson(id = id, startedAt = startedAt)
        }
        val filled = coverageFilled ?: existing.coverage?.filled ?: 0
        val nBins = existing.coverage?.nBins ?: coverageNBins
        val meta = existing.copy(
            endedAt = endedAt ?: existing.endedAt,
            nFrames = nFrames,
            device = existing.device?.copy(nodeId = nodeId) ?: DeviceMeta(nodeId = nodeId),
            camera = CameraMeta(
                width = width ?: existing.camera?.width,
                height = height ?: existing.camera?.height,
                focalHint = existing.camera?.focalHint,
                model = existing.camera?.model ?: "pinhole",
            ),
            coverage = CoverageMeta(nBins = nBins, filled = filled),
        )
        File(dir, "session.json").writeText(json.encodeToString(meta))
    }
}
