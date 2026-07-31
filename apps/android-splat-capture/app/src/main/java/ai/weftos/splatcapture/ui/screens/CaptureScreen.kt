package ai.weftos.splatcapture.ui.screens

import android.Manifest
import android.util.Log
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.isGranted
import com.google.accompanist.permissions.rememberPermissionState
import ai.weftos.splatcapture.SplatCaptureApp
import ai.weftos.splatcapture.capture.ImuPoseProvider
import ai.weftos.splatcapture.session.ActiveSession
import ai.weftos.splatcapture.ui.components.CameraPreview
import ai.weftos.splatcapture.ui.components.CoverageBinsView
import kotlinx.coroutines.delay
import java.io.File
import java.util.concurrent.Executor

private const val TAG = "CaptureScreen"

@OptIn(ExperimentalPermissionsApi::class)
@Composable
fun CaptureScreen(
    app: SplatCaptureApp,
    onSessionFinished: (String) -> Unit,
) {
    val context = LocalContext.current
    val cameraPerm = rememberPermissionState(Manifest.permission.CAMERA)
    val imu = remember { ImuPoseProvider(context) }
    var imageCapture by remember { mutableStateOf<ImageCapture?>(null) }
    var session by remember { mutableStateOf<ActiveSession?>(null) }
    var frameCount by remember { mutableIntStateOf(0) }
    var liveBin by remember { mutableIntStateOf(0) }
    var status by remember { mutableStateOf("Ready") }
    var capturing by remember { mutableStateOf(false) }

    DisposableEffect(Unit) {
        imu.start()
        onDispose { imu.stop() }
    }

    LaunchedEffect(Unit) {
        while (true) {
            liveBin = imu.liveBin()
            delay(50)
        }
    }

    if (!cameraPerm.status.isGranted) {
        Column(
            Modifier
                .fillMaxSize()
                .padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text("Camera permission is required for splat capture.")
            Spacer(Modifier.height(16.dp))
            Button(onClick = { cameraPerm.launchPermissionRequest() }) {
                Text("Grant camera")
            }
        }
        return
    }

    Column(Modifier.fillMaxSize()) {
        Box(
            Modifier
                .weight(1f)
                .fillMaxWidth(),
        ) {
            CameraPreview(
                modifier = Modifier.fillMaxSize(),
                onImageCaptureReady = { imageCapture = it },
            )
            Column(
                Modifier
                    .align(Alignment.BottomStart)
                    .fillMaxWidth()
                    .background(Color(0xAA0B1020))
                    .padding(12.dp),
            ) {
                Text(
                    "Coverage ${"%.0f".format(imu.coverage.coveragePercent())}% · " +
                        "${imu.coverage.filledCount()}/${imu.coverage.nBins} bins · frames $frameCount",
                    color = Color.White,
                    style = MaterialTheme.typography.bodyMedium,
                )
                if (!imu.hasSensor) {
                    Text("No rotation vector — poses default to identity", color = Color(0xFFFFB74D))
                }
                CoverageBinsView(
                    coverage = imu.coverage,
                    liveBin = liveBin,
                    modifier = Modifier
                        .padding(top = 8.dp)
                        .height(72.dp),
                )
                val missing = imu.coverage.missingBins(4)
                if (missing.isNotEmpty() && session != null) {
                    Text(
                        "Missing: " + missing.joinToString(" · ") { imu.coverage.hintForBin(it) },
                        color = Color(0xFF9AE6B4),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
        }

        Column(
            Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(status, style = MaterialTheme.typography.bodySmall)
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                FilledTonalButton(
                    onClick = {
                        if (session == null) {
                            val node = app.pairs.nodeId()
                            session = app.sessions.startSession(node)
                            frameCount = 0
                            status = "Session ${session!!.id.take(8)}…"
                        }
                    },
                    enabled = session == null,
                ) { Text("New session") }

                Button(
                    onClick = {
                        val cap = imageCapture ?: run {
                            status = "Camera not ready"
                            return@Button
                        }
                        val active = session ?: run {
                            status = "Start a session first"
                            return@Button
                        }
                        if (capturing) return@Button
                        capturing = true
                        takeStill(context.mainExecutor(), cap, active, imu) { ok, msg ->
                            capturing = false
                            if (ok) {
                                frameCount = active.nFrames.toInt()
                                status = msg
                            } else {
                                status = msg
                            }
                        }
                    },
                    enabled = session != null && !capturing,
                    modifier = Modifier.size(72.dp),
                    shape = CircleShape,
                ) {
                    Text("SNAP")
                }

                FilledTonalButton(
                    onClick = {
                        val active = session ?: return@FilledTonalButton
                        active.finish(
                            coverageNBins = imu.coverage.nBins,
                            coverageFilled = imu.coverage.filledCount(),
                        )
                        val zip = app.sessions.exportZip(active.id)
                        status = "Exported ${zip.name}"
                        val id = active.id
                        session = null
                        onSessionFinished(id)
                    },
                    enabled = session != null && frameCount > 0,
                ) { Text("Stop + ZIP") }
            }
            Text(
                "Node ${app.pairs.nodeId().take(16)}… · protocol weft.capture.v1",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
            )
        }
    }
}

private fun android.content.Context.mainExecutor(): Executor =
    ContextCompat.getMainExecutor(this)

private fun takeStill(
    mainExecutor: Executor,
    imageCapture: ImageCapture,
    session: ActiveSession,
    imu: ImuPoseProvider,
    onDone: (Boolean, String) -> Unit,
) {
    val tmp = File.createTempFile("frame", ".jpg", session.framesDir)
    val opts = ImageCapture.OutputFileOptions.Builder(tmp).build()
    // Sample pose at shutter time (before async encode finishes).
    val pose = imu.samplePose()
    imageCapture.takePicture(
        opts,
        mainExecutor,
        object : ImageCapture.OnImageSavedCallback {
            override fun onImageSaved(output: ImageCapture.OutputFileResults) {
                try {
                    val jpeg = tmp.readBytes()
                    tmp.delete()
                    // CameraX default still size often 4:3; store as captured.
                    session.addFrameJpeg(jpeg, pose, width = 1080, height = 1920)
                    onDone(true, "Frame ${session.nFrames} · bin ${pose.coverageBin}")
                } catch (e: Exception) {
                    Log.e(TAG, "save frame", e)
                    onDone(false, "Save failed: ${e.message}")
                }
            }

            override fun onError(exception: ImageCaptureException) {
                tmp.delete()
                Log.e(TAG, "capture", exception)
                onDone(false, "Capture error: ${exception.message}")
            }
        },
    )
}
