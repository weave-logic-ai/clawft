package ai.weftos.splatcapture.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import ai.weftos.splatcapture.SplatCaptureApp
import ai.weftos.splatcapture.upload.SplatdClient
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun UploadScreen(
    app: SplatCaptureApp,
    sessionId: String,
    onOpenSog: (String) -> Unit,
    onDone: () -> Unit,
) {
    val pair = app.pairs.current()
    val client = remember { SplatdClient() }
    val scope = rememberCoroutineScope()
    var phase by remember { mutableStateOf("idle") }
    var jobId by remember { mutableStateOf<String?>(null) }
    var stage by remember { mutableStateOf("") }
    var status by remember { mutableStateOf("") }
    var sogUrl by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }

    fun startUpload() {
        val p = pair ?: run {
            error = "Pair with a host first (Pair tab)."
            return
        }
        scope.launch {
            busy = true
            error = null
            phase = "zip"
            try {
                val zip = app.sessions.exportZip(sessionId)
                phase = "upload"
                status = "Uploading ${zip.name}…"
                val result = client.uploadSessionZip(p, zip)
                jobId = result.jobId
                phase = "poll"
                status = "job ${result.jobId}"
                // Poll until terminal or timeout ~30 min
                repeat(360) {
                    val st = client.getJob(p, result.jobId)
                    stage = st.stage
                    status = st.status
                    if (st.sogUrl.isNotEmpty()) sogUrl = st.sogUrl
                    if (st.status in listOf("done", "completed", "failed", "error")) {
                        phase = st.status
                        return@launch
                    }
                    delay(5_000)
                }
                phase = "timeout"
            } catch (e: Exception) {
                error = e.message
                phase = "error"
            } finally {
                busy = false
            }
        }
    }

    LaunchedEffect(sessionId) {
        // auto-start when pair present
        if (pair != null) startUpload()
    }

    Column(
        Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        horizontalAlignment = Alignment.Start,
    ) {
        Text("Upload session", style = MaterialTheme.typography.headlineSmall)
        Text("Session: $sessionId", style = MaterialTheme.typography.bodySmall)
        if (pair == null) {
            Text("No host pair. Open the Pair tab and save a splatd URL.")
            Button(onClick = onDone) { Text("Back") }
            return
        }
        Text("Host: ${pair.baseUrl}")
        if (busy) {
            LinearProgressIndicator(Modifier.fillMaxWidth())
            CircularProgressIndicator()
        }
        Text("Phase: $phase")
        jobId?.let { Text("job_id: $it") }
        if (stage.isNotEmpty()) Text("stage: $stage")
        if (status.isNotEmpty()) Text("status: $status")
        error?.let { Text("Error: $it", color = MaterialTheme.colorScheme.error) }

        if (!busy && phase == "idle") {
            Button(onClick = { startUpload() }, modifier = Modifier.fillMaxWidth()) {
                Text("Start upload")
            }
        }
        if (sogUrl.isNotEmpty()) {
            Button(
                onClick = { onOpenSog(sogUrl) },
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Open SOG") }
        }
        FilledTonalButton(onClick = onDone, modifier = Modifier.fillMaxWidth()) {
            Text("Close")
        }
    }
}
