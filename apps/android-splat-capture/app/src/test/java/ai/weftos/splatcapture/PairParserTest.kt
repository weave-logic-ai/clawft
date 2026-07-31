package ai.weftos.splatcapture

import ai.weftos.splatcapture.edge.KotlinPairParser
import org.junit.Assert.assertEquals
import org.junit.Test

class PairParserTest {
    @Test
    fun parseWeftosQr() {
        val p = KotlinPairParser.parse(
            "weftos://pair?host=10.0.0.2&port=7860&token=abc&node=dead&v=1",
            null,
        )
        assertEquals("http://10.0.0.2:7860", p.baseUrl)
        assertEquals("abc", p.token)
        assertEquals("dead", p.hostNodeId)
        assertEquals(1, p.pairVersion)
    }

    @Test
    fun parsePlainUrlWithTokenOverride() {
        val p = KotlinPairParser.parse("https://splat.example.com", "tok")
        assertEquals("https://splat.example.com", p.baseUrl)
        assertEquals("tok", p.token)
    }
}
