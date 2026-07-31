package ai.weftos.splatcapture.capture

import kotlin.math.PI
import kotlin.math.atan2
import kotlin.math.asin
import kotlin.math.max
import kotlin.math.min
import kotlin.math.sqrt

/**
 * Discretized view-direction coverage on a sphere (azimuth × elevation bins).
 *
 * Matches ADR-077 "coverage sphere" MVP: simplified rectangular bins rather
 * than a full icosahedron — enough for missing-patch guidance in A1.
 */
class CoverageMap(
    val azimuthBins: Int = 16,
    val elevationBins: Int = 8,
) {
    val nBins: Int = azimuthBins * elevationBins
    private val counts = IntArray(nBins)

    fun filledCount(): Int = counts.count { it > 0 }

    fun coveragePercent(): Float =
        if (nBins == 0) 0f else filledCount().toFloat() / nBins.toFloat() * 100f

    fun countAt(bin: Int): Int = if (bin in counts.indices) counts[bin] else 0

    fun mark(bin: Int) {
        if (bin in counts.indices) counts[bin] += 1
    }

    fun snapshot(): IntArray = counts.copyOf()

    /**
     * Map unit quaternion (w,x,y,z) — device attitude — to a bin by projecting
     * the camera forward vector (-Z in Android camera coords approximated as
     * device "looking" direction from rotation vector).
     */
    fun binForQuatWxyz(w: Double, x: Double, y: Double, z: Double): Int {
        // Rotate local forward (0,0,-1) by quaternion → world direction.
        val fx = 2.0 * (x * z + w * y)
        val fy = 2.0 * (y * z - w * x)
        val fz = 1.0 - 2.0 * (x * x + y * y)
        // We used +Z from rotation; flip for camera look-along -Z:
        val dx = -fx
        val dy = -fy
        val dz = -fz
        return binForDirection(dx, dy, dz)
    }

    fun binForDirection(dx: Double, dy: Double, dz: Double): Int {
        val len = sqrt(dx * dx + dy * dy + dz * dz).coerceAtLeast(1e-9)
        val x = dx / len
        val y = dy / len
        val z = dz / len
        // Azimuth in [0, 2π): atan2(x, z) so 0 = looking +Z world.
        var az = atan2(x, z)
        if (az < 0) az += 2.0 * PI
        // Elevation in [-π/2, π/2]: asin(y)
        val el = asin(max(-1.0, min(1.0, y)))

        val azIdx = ((az / (2.0 * PI)) * azimuthBins).toInt().coerceIn(0, azimuthBins - 1)
        val elNorm = (el + PI / 2.0) / PI // 0..1
        val elIdx = (elNorm * elevationBins).toInt().coerceIn(0, elevationBins - 1)
        return elIdx * azimuthBins + azIdx
    }

    /** Empty bins sorted by elevation band preference (horizon first). */
    fun missingBins(limit: Int = 12): List<Int> {
        val empty = mutableListOf<Int>()
        // Prefer mid elevations (walkaround height), then extremes.
        val order = (0 until elevationBins).sortedBy { el ->
            val mid = elevationBins / 2
            kotlin.math.abs(el - mid)
        }
        for (el in order) {
            for (az in 0 until azimuthBins) {
                val b = el * azimuthBins + az
                if (counts[b] == 0) empty.add(b)
                if (empty.size >= limit) return empty
            }
        }
        return empty
    }

    fun hintForBin(bin: Int): String {
        if (bin !in 0 until nBins) return "look around"
        val el = bin / azimuthBins
        val az = bin % azimuthBins
        val azDeg = (az + 0.5) * 360.0 / azimuthBins
        val elDeg = (el + 0.5) * 180.0 / elevationBins - 90.0
        val turn = when {
            azDeg < 45 || azDeg >= 315 -> "face forward"
            azDeg < 135 -> "turn right"
            azDeg < 225 -> "turn around"
            else -> "turn left"
        }
        val look = when {
            elDeg > 35 -> "look up"
            elDeg < -35 -> "look down"
            else -> "level"
        }
        return "$turn · $look"
    }
}
