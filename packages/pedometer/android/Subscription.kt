package exact.pedometer

/** An existing Recording subscription outlives the process; only a new one has a start we know. */
class Subscription {
    var since: Double? = null
        private set
    private var ready = false
    private var pending = false

    fun begin(): Boolean {
        if (ready || pending) return false
        pending = true
        return true
    }

    fun complete(existing: Boolean, epoch: Double) {
        since = if (existing) null else epoch
        ready = true
        pending = false
    }

    fun failed() {
        pending = false
    }
}
