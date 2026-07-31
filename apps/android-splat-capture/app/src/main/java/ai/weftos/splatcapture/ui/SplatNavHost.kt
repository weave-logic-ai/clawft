package ai.weftos.splatcapture.ui

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CameraAlt
import androidx.compose.material.icons.filled.Folder
import androidx.compose.material.icons.filled.Link
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import ai.weftos.splatcapture.SplatCaptureApp
import ai.weftos.splatcapture.ui.screens.CaptureScreen
import ai.weftos.splatcapture.ui.screens.PairScreen
import ai.weftos.splatcapture.ui.screens.SessionsScreen
import ai.weftos.splatcapture.ui.screens.SogViewerScreen
import ai.weftos.splatcapture.ui.screens.UploadScreen

private object Routes {
    const val Capture = "capture"
    const val Sessions = "sessions"
    const val Pair = "pair"
    const val Upload = "upload/{sessionId}"
    const val Sog = "sog?url={url}"
    fun upload(sessionId: String) = "upload/$sessionId"
    fun sog(url: String) = "sog?url=${java.net.URLEncoder.encode(url, Charsets.UTF_8.name())}"
}

@Composable
fun SplatNavHost(
    app: SplatCaptureApp,
    pendingPairUri: String?,
    onPairUriConsumed: () -> Unit,
) {
    val nav = rememberNavController()
    val backStack by nav.currentBackStackEntryAsState()
    val route = backStack?.destination?.route

    LaunchedEffect(pendingPairUri) {
        if (pendingPairUri != null) {
            nav.navigate(Routes.Pair) {
                launchSingleTop = true
            }
        }
    }

    Scaffold(
        bottomBar = {
            if (route in listOf(Routes.Capture, Routes.Sessions, Routes.Pair) ||
                route?.startsWith("upload") == true
            ) {
                NavigationBar {
                    NavigationBarItem(
                        selected = route == Routes.Capture,
                        onClick = {
                            nav.navigate(Routes.Capture) {
                                launchSingleTop = true
                                popUpTo(Routes.Capture) { inclusive = false }
                            }
                        },
                        icon = { Icon(Icons.Default.CameraAlt, contentDescription = null) },
                        label = { Text("Capture") },
                    )
                    NavigationBarItem(
                        selected = route == Routes.Sessions,
                        onClick = { nav.navigate(Routes.Sessions) { launchSingleTop = true } },
                        icon = { Icon(Icons.Default.Folder, contentDescription = null) },
                        label = { Text("Sessions") },
                    )
                    NavigationBarItem(
                        selected = route == Routes.Pair,
                        onClick = { nav.navigate(Routes.Pair) { launchSingleTop = true } },
                        icon = { Icon(Icons.Default.Link, contentDescription = null) },
                        label = { Text("Pair") },
                    )
                }
            }
        },
    ) { padding ->
        NavHost(
            navController = nav,
            startDestination = Routes.Capture,
            modifier = Modifier.padding(padding),
        ) {
            composable(Routes.Capture) {
                CaptureScreen(
                    app = app,
                    onSessionFinished = { id ->
                        nav.navigate(Routes.Sessions)
                    },
                )
            }
            composable(Routes.Sessions) {
                SessionsScreen(
                    app = app,
                    onUpload = { id -> nav.navigate(Routes.upload(id)) },
                    onOpenSog = { url -> nav.navigate(Routes.sog(url)) },
                )
            }
            composable(Routes.Pair) {
                PairScreen(
                    app = app,
                    pendingUri = pendingPairUri,
                    onConsumedUri = onPairUriConsumed,
                )
            }
            composable(
                route = Routes.Upload,
                arguments = listOf(navArgument("sessionId") { type = NavType.StringType }),
            ) { entry ->
                val sessionId = entry.arguments?.getString("sessionId") ?: return@composable
                UploadScreen(
                    app = app,
                    sessionId = sessionId,
                    onOpenSog = { url -> nav.navigate(Routes.sog(url)) },
                    onDone = { nav.popBackStack() },
                )
            }
            composable(
                route = Routes.Sog,
                arguments = listOf(
                    navArgument("url") {
                        type = NavType.StringType
                        defaultValue = ""
                        nullable = true
                    },
                ),
            ) { entry ->
                val url = entry.arguments?.getString("url").orEmpty()
                SogViewerScreen(
                    url = url,
                    onBack = { nav.popBackStack() },
                )
            }
        }
    }
}
