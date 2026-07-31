package ai.weftos.splatcapture.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val WeftDark = darkColorScheme(
    primary = Color(0xFF5B8CFF),
    onPrimary = Color(0xFF0B1020),
    secondary = Color(0xFF9AE6B4),
    background = Color(0xFF0B1020),
    surface = Color(0xFF151B2E),
    onBackground = Color(0xFFE8ECF8),
    onSurface = Color(0xFFE8ECF8),
    error = Color(0xFFFF6B6B),
)

@Composable
fun WeftSplatTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = WeftDark,
        content = content,
    )
}
