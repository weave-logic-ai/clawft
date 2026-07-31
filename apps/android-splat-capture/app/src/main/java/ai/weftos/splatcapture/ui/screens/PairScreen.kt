package ai.weftos.splatcapture.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import ai.weftos.splatcapture.SplatCaptureApp
import ai.weftos.splatcapture.upload.SplatdClient
import kotlinx.coroutines.launch

@Composable
fun PairScreen(
    app: SplatCaptureApp,
    pendingUri: String?,
    onConsumedUri: () -> Unit,
) {
    var hostInput by remember { mutableStateOf("http://192.168.1.10:7860") }
    var token by remember { mutableStateOf("") }
    var status by remember { mutableStateOf("") }
    var current by remember { mutableStateOf(app.pairs.current()) }
    val scope = rememberCoroutineScope()
    val client = remember { SplatdClient() }

    LaunchedEffect(pendingUri) {
        if (pendingUri != null) {
            hostInput = pendingUri
            try {
                val pair = app.pairs.pairFromInput(pendingUri, token.ifBlank { null })
                current = pair
                status = "Paired via deep link → ${pair.baseUrl}"
            } catch (e: Exception) {
                status = "Pair failed: ${e.message}"
            }
            onConsumedUri()
        }
    }

    Column(
        Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Pair with host", style = MaterialTheme.typography.headlineSmall)
        Text(
            "Paste splatd URL or weftos://pair?host=…&port=7860&token=… QR payload. " +
                "Node id is local Ed25519/fallback identity (WEFT-707).",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.75f),
        )
        Text(
            "This node: ${app.pairs.nodeId()}",
            style = MaterialTheme.typography.labelMedium,
        )
        OutlinedTextField(
            value = hostInput,
            onValueChange = { hostInput = it },
            label = { Text("Host URL or weftos://pair…") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = false,
            minLines = 2,
        )
        OutlinedTextField(
            value = token,
            onValueChange = { token = it },
            label = { Text("Capability token (optional)") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
        )
        Button(
            onClick = {
                try {
                    val pair = app.pairs.pairFromInput(hostInput, token.ifBlank { null })
                    current = pair
                    status = "Saved pair → ${pair.baseUrl}"
                } catch (e: Exception) {
                    status = "Error: ${e.message}"
                }
            },
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Save pair") }

        FilledTonalButton(
            onClick = {
                val pair = current ?: run {
                    status = "No pair saved"
                    return@FilledTonalButton
                }
                scope.launch {
                    status = try {
                        val body = client.health(pair)
                        "healthz OK: ${body.take(120)}"
                    } catch (e: Exception) {
                        "healthz failed: ${e.message}"
                    }
                }
            },
            modifier = Modifier.fillMaxWidth(),
            enabled = current != null,
        ) { Text("Probe /healthz") }

        FilledTonalButton(
            onClick = {
                app.pairs.unpair()
                current = null
                status = "Unpaired"
            },
            modifier = Modifier.fillMaxWidth(),
            enabled = current != null,
        ) { Text("Unpair") }

        Spacer(Modifier.height(8.dp))
        if (current != null) {
            Text("Active pair", style = MaterialTheme.typography.titleMedium)
            Text("URL: ${current!!.baseUrl}")
            Text("Token: ${if (current!!.token.isEmpty()) "(none)" else "••••${current!!.token.takeLast(4)}"}")
            current!!.hostNodeId?.let { Text("Host node: $it") }
        }
        if (status.isNotEmpty()) {
            Text(status, style = MaterialTheme.typography.bodyMedium)
        }
    }
}
