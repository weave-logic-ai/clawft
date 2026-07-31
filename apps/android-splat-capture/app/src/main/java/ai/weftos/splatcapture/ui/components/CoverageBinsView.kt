package ai.weftos.splatcapture.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import ai.weftos.splatcapture.capture.CoverageMap

/**
 * Simplified coverage sphere: elevation × azimuth grid of bins.
 * Filled bins glow green; live look-direction highlighted in cyan.
 */
@Composable
fun CoverageBinsView(
    coverage: CoverageMap,
    liveBin: Int,
    modifier: Modifier = Modifier,
) {
    val snap = coverage.snapshot()
    Canvas(
        modifier = modifier
            .fillMaxWidth()
            .aspectRatio(coverage.azimuthBins.toFloat() / coverage.elevationBins.toFloat()),
    ) {
        val cellW = size.width / coverage.azimuthBins
        val cellH = size.height / coverage.elevationBins
        for (el in 0 until coverage.elevationBins) {
            for (az in 0 until coverage.azimuthBins) {
                val bin = el * coverage.azimuthBins + az
                val count = snap.getOrElse(bin) { 0 }
                val fill = when {
                    count == 0 -> Color(0xFF1A2238)
                    count < 3 -> Color(0xFF2F6B4A)
                    else -> Color(0xFF3FA36A)
                }
                val topLeft = Offset(az * cellW, el * cellH)
                drawRect(color = fill, topLeft = topLeft, size = Size(cellW - 1f, cellH - 1f))
                if (bin == liveBin) {
                    drawRect(
                        color = Color(0xFF5B8CFF),
                        topLeft = topLeft,
                        size = Size(cellW - 1f, cellH - 1f),
                        style = Stroke(width = 3f),
                    )
                }
            }
        }
    }
}
