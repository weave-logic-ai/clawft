package ai.weftos.splatcapture

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import ai.weftos.splatcapture.ui.SplatNavHost
import ai.weftos.splatcapture.ui.theme.WeftSplatTheme

class MainActivity : ComponentActivity() {
    private var pendingPairUri by mutableStateOf<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        pendingPairUri = intent?.data?.toString()
        setContent {
            WeftSplatTheme {
                SplatNavHost(
                    app = application.asSplatApp(),
                    pendingPairUri = pendingPairUri,
                    onPairUriConsumed = { pendingPairUri = null },
                )
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        pendingPairUri = intent.data?.toString()
    }
}
