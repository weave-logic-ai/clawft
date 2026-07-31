package ai.weftos.splatcapture

import ai.weftos.splatcapture.capture.CoverageMap
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class CoverageMapTest {
    @Test
    fun identityQuatMarksABin() {
        val map = CoverageMap(azimuthBins = 16, elevationBins = 8)
        val bin = map.binForQuatWxyz(1.0, 0.0, 0.0, 0.0)
        assertTrue(bin in 0 until map.nBins)
        map.mark(bin)
        assertEquals(1, map.filledCount())
        assertTrue(map.coveragePercent() > 0f)
    }

    @Test
    fun missingBinsPreferHorizon() {
        val map = CoverageMap(azimuthBins = 4, elevationBins = 4)
        val missing = map.missingBins(4)
        assertEquals(4, missing.size)
    }
}
