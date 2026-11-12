package exact.pedometer

/**
 * One reader group's count (D12): a Recording total plus the step counter's increase since that read, or, on the
 * sensor path, the counter's increase from a zero that moves at every return from the background and every reboot.
 * It touches no platform API, so its sums can be checked without a device.
 */
class Count(val start: Double) {
    class Answer(val steps: Double, val since: Double, val at: Double)
    class Range(val from: Double, epoch: Double) {
        val startSeconds = (from / 1000).toLong()
        val endSeconds = (epoch / 1000).toLong() + 1
        val bucketSeconds = maxOf(1, ((epoch - from) / 1000).toInt() + 1)
    }

    companion object {
        /** How long a Recording total stands before a poll reads it again. */
        const val READ_EVERY_MS = 60_000.0
        /** What the Recording API keeps (D12): "up to 10 days". */
        private const val KEPT_MS = 10 * 86_400_000.0
    }

    private var latest: Double? = null
    // The sensor path: its zero, when the zero was taken, and whether the next event takes a new one.
    private var zero: Double? = null
    private var since = start
    private var rezero = false
    // The Recording path: the last total, what it counted from, when it was read, and the counter then.
    private var total: Double? = null
    private var totalFrom = start
    // Refresh intervals use the session clock; boundaries and answer timestamps use epoch milliseconds.
    private var totalAt = Double.NEGATIVE_INFINITY
    private var atRead: Double? = null

    val hasRecording get() = total != null

    /** No request while a clock correction leaves the boundary in the future. */
    fun range(subscribed: Double?, epoch: Double): Range? {
        val from = maxOf(start, subscribed ?: start, epoch - KEPT_MS)
        if (from > epoch) return null
        return Range(from, epoch)
    }

    /** A counter event. A fall is a reboot: both paths count on from the new value. */
    fun counter(value: Double, epoch: Double) {
        val last = latest
        if (last != null && value < last) {
            // The Recording path keeps what the counter added before the fall, so the count never goes back.
            val t = total
            val a = atRead
            if (t != null && a != null && last >= a) total = t + (last - a)
            zero = value
            since = epoch
            atRead = value
        }
        if (zero == null || rezero) {
            if (rezero) since = epoch
            zero = value
            rezero = false
        }
        if (atRead == null) atRead = value
        latest = value
    }

    /** The app left the foreground: no counter event arrives until the return, so neither path trusts the last one. */
    fun hidden() {
        latest = null
        atRead = null
        rezero = true
    }

    /** Whether the next answer reads the Recording API again: none yet, a stale one, or one across a gap. */
    fun due(elapsed: Double): Boolean = total == null || elapsed - totalAt >= READ_EVERY_MS || atRead == null

    /** The sensor path's answer. */
    fun sensor(epoch: Double): Answer {
        val z = zero
        val l = latest
        return Answer(if (z != null && l != null) l - z else 0.0, since, epoch)
    }

    /** A Recording read of [steps] from [from]: the counter counts on from its next event. */
    fun recorded(steps: Double, from: Double, elapsed: Double, epoch: Double): Answer {
        total = steps
        totalFrom = from
        totalAt = elapsed
        atRead = latest
        return Answer(steps, from, epoch)
    }

    /** Between Recording reads: the last total and the counter's increase since. */
    fun between(epoch: Double): Answer {
        val a = atRead
        val l = latest
        return Answer((total ?: 0.0) + if (a != null && l != null && l >= a) l - a else 0.0, totalFrom, epoch)
    }
}
