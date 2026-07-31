package ai.weftos.splatcapture.ui.screens

import android.content.Intent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import ai.weftos.splatcapture.SplatCaptureApp
import ai.weftos.splatcapture.session.SessionSummary
import java.io.File

@Composable
fun SessionsScreen(
    app: SplatCaptureApp,
    onUpload: (String) -> Unit,
    onOpenSog: (String) -> Unit,
) {
    val context = LocalContext.current
    var sessions by remember { mutableStateOf<List<SessionSummary>>(emptyList()) }

    fun refresh() {
        sessions = app.sessions.listSessions()
    }

    LaunchedEffect(Unit) { refresh() }

    Column(
        Modifier
            .fillMaxSize()
            .padding(16.dp),
    ) {
        Text("Sessions", style = MaterialTheme.typography.headlineSmall)
        Text(
            "weft.capture.v1 packages (frames + poses.jsonl + session.json)",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f),
            modifier = Modifier.padding(bottom = 12.dp),
        )
        if (sessions.isEmpty()) {
            Text("No sessions yet. Capture stills, then Stop + ZIP.")
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(sessions, key = { it.id }) { s ->
                    Card(Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(12.dp)) {
                            Text(s.id.take(13) + "…", style = MaterialTheme.typography.titleMedium)
                            Text(
                                "${s.nFrames} frames · ${"%.0f".format(s.coveragePercent)}% coverage",
                                style = MaterialTheme.typography.bodySmall,
                            )
                            Text(s.startedAt, style = MaterialTheme.typography.labelSmall)
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                FilledTonalButton(onClick = {
                                    val zip = s.zipPath?.let { File(it) }
                                        ?: app.sessions.exportZip(s.id)
                                    val uri = FileProvider.getUriForFile(
                                        context,
                                        context.packageName + ".fileprovider",
                                        zip,
                                    )
                                    val send = Intent(Intent.ACTION_SEND).apply {
                                        type = "application/zip"
                                        putExtra(Intent.EXTRA_STREAM, uri)
                                        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                                    }
                                    context.startActivity(Intent.createChooser(send, "Export session ZIP"))
                                    refresh()
                                }) { Text("Share ZIP") }
                                FilledTonalButton(onClick = { onUpload(s.id) }) {
                                    Text("Upload")
                                }
                            }
                        }
                    }
                }
            }
        }
        TextButton(onClick = { refresh() }, modifier = Modifier.padding(top = 8.dp)) {
            Text("Refresh")
        }
    }
}
