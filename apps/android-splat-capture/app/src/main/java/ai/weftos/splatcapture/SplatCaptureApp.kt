package ai.weftos.splatcapture

import android.app.Application
import ai.weftos.splatcapture.edge.EdgeCore
import ai.weftos.splatcapture.pair.PairRepository
import ai.weftos.splatcapture.session.SessionRepository

class SplatCaptureApp : Application() {
    lateinit var edgeCore: EdgeCore
        private set
    lateinit var sessions: SessionRepository
        private set
    lateinit var pairs: PairRepository
        private set

    override fun onCreate() {
        super.onCreate()
        edgeCore = EdgeCore(this)
        sessions = SessionRepository(this)
        pairs = PairRepository(this, edgeCore)
    }
}

fun Application.asSplatApp(): SplatCaptureApp = this as SplatCaptureApp
