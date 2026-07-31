package ai.weftos.splatcapture.capture

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import kotlin.math.sqrt

/**
 * High-rate rotation-vector attitude for pose sidecars + coverage bins.
 *
 * Uses [Sensor.TYPE_ROTATION_VECTOR] when available, else game rotation vector.
 * Quaternion order for export: **w,x,y,z** (weft.capture.v1).
 */
class ImuPoseProvider(context: Context) : SensorEventListener {
    data class Pose(
        val tNs: Long,
        val quatWxyz: FloatArray,
        val coverageBin: Int,
    ) {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (other !is Pose) return false
            return tNs == other.tNs && quatWxyz.contentEquals(other.quatWxyz) &&
                coverageBin == other.coverageBin
        }

        override fun hashCode(): Int {
            var result = tNs.hashCode()
            result = 31 * result + quatWxyz.contentHashCode()
            result = 31 * result + coverageBin
            return result
        }
    }

    private val sensorManager =
        context.getSystemService(Context.SENSOR_SERVICE) as SensorManager
    private val rotationSensor: Sensor? =
        sensorManager.getDefaultSensor(Sensor.TYPE_ROTATION_VECTOR)
            ?: sensorManager.getDefaultSensor(Sensor.TYPE_GAME_ROTATION_VECTOR)

    private val lock = Any()
    private var latestQuat = floatArrayOf(1f, 0f, 0f, 0f)
    private var latestTNs = System.nanoTime()
    val coverage = CoverageMap()

    val hasSensor: Boolean get() = rotationSensor != null

    fun start() {
        val s = rotationSensor ?: return
        sensorManager.registerListener(this, s, SensorManager.SENSOR_DELAY_GAME)
    }

    fun stop() {
        sensorManager.unregisterListener(this)
    }

    /** Snapshot nearest rotation sample for a still capture. */
    fun samplePose(): Pose {
        val q: FloatArray
        val t: Long
        synchronized(lock) {
            q = latestQuat.copyOf()
            t = latestTNs
        }
        normalizeInPlace(q)
        val bin = coverage.binForQuatWxyz(
            q[0].toDouble(),
            q[1].toDouble(),
            q[2].toDouble(),
            q[3].toDouble(),
        )
        coverage.mark(bin)
        return Pose(tNs = t, quatWxyz = q, coverageBin = bin)
    }

    /** Live bin for HUD without marking. */
    fun liveBin(): Int {
        val q: FloatArray
        synchronized(lock) {
            q = latestQuat.copyOf()
        }
        return coverage.binForQuatWxyz(
            q[0].toDouble(),
            q[1].toDouble(),
            q[2].toDouble(),
            q[3].toDouble(),
        )
    }

    fun liveQuat(): FloatArray {
        synchronized(lock) {
            return latestQuat.copyOf()
        }
    }

    override fun onSensorChanged(event: SensorEvent) {
        // Android rotation vector: event.values = [x, y, z, (optional w)]
        val v = event.values
        val q = FloatArray(4)
        if (v.size >= 4) {
            q[1] = v[0]
            q[2] = v[1]
            q[3] = v[2]
            q[0] = v[3]
        } else if (v.size >= 3) {
            q[1] = v[0]
            q[2] = v[1]
            q[3] = v[2]
            val w2 = 1f - (q[1] * q[1] + q[2] * q[2] + q[3] * q[3])
            q[0] = if (w2 > 0f) sqrt(w2) else 0f
        } else {
            return
        }
        normalizeInPlace(q)
        synchronized(lock) {
            latestQuat = q
            latestTNs = event.timestamp
        }
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) = Unit

    private fun normalizeInPlace(q: FloatArray) {
        val n = sqrt(q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3])
        if (n > 1e-9f) {
            q[0] /= n
            q[1] /= n
            q[2] /= n
            q[3] /= n
        }
    }
}
