package ai.weftos.splatcapture.ui.screens

import android.annotation.SuppressLint
import android.view.ViewGroup
import android.webkit.WebChromeClient
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView

/**
 * SOG review via WebView. Prefer host-served Spark/viewer HTML; raw .sog URLs
 * may need a viewer page that loads the asset (examples/splat-viewer).
 */
@SuppressLint("SetJavaScriptEnabled")
@Composable
fun SogViewerScreen(
    url: String,
    onBack: () -> Unit,
) {
    Column(Modifier.fillMaxSize()) {
        Text(
            "SOG review",
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.padding(12.dp),
        )
        Text(
            url.ifBlank { "(no url)" },
            style = MaterialTheme.typography.labelSmall,
            modifier = Modifier.padding(horizontal = 12.dp),
        )
        if (url.isBlank()) {
            Text("No SOG URL yet.", modifier = Modifier.padding(12.dp))
        } else {
            AndroidView(
                factory = { ctx ->
                    WebView(ctx).apply {
                        layoutParams = ViewGroup.LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT,
                            ViewGroup.LayoutParams.MATCH_PARENT,
                        )
                        settings.javaScriptEnabled = true
                        settings.domStorageEnabled = true
                        settings.cacheMode = WebSettings.LOAD_DEFAULT
                        settings.allowFileAccess = false
                        webViewClient = WebViewClient()
                        webChromeClient = WebChromeClient()
                        // If URL is a bare .sog, open the in-repo viewer with query.
                        val load = if (url.endsWith(".sog")) {
                            // Host may serve a viewer; otherwise show artifact URL.
                            url
                        } else {
                            url
                        }
                        loadUrl(load)
                    }
                },
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth(),
            )
        }
        FilledTonalButton(
            onClick = onBack,
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
        ) { Text("Back") }
    }
}
