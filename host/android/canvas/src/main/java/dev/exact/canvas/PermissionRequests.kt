package dev.exact.canvas

/** Permission dialogs and their pending callbacks; all calls run on the main thread. */
internal class PermissionRequests(private val present: (Array<String>, Int) -> Boolean) {
    private class Request(val names: Array<String>, val code: Int, done: (Set<String>) -> Unit) {
        val callbacks = arrayListOf(done)
    }
    private var foreground = false
    private var active: Request? = null
    private val waiting = ArrayDeque<Request>()
    private var completing = false
    private var serial = 0

    fun ask(names: Array<String>, done: (Set<String>) -> Unit) {
        if (names.isEmpty() || (!foreground && active == null && waiting.isEmpty() && !completing)) return done(emptySet())
        val shared = (listOfNotNull(active) + waiting).firstOrNull { it.names.toSet() == names.toSet() }
        if (shared != null) { shared.callbacks += done; return }
        // Activity results use 0x5000–0x5fff. A distinct code rejects a late result after recreation.
        waiting += Request(names.copyOf(), 0x4000 + (serial++ % 0x1000), done)
        advance()
    }

    private fun advance() {
        if (!foreground || active != null || completing) return
        val next = waiting.removeFirstOrNull() ?: return
        active = next
        if (!present(next.names, next.code)) finish(emptySet())
    }

    private fun finish(answered: Set<String>) {
        val request = active ?: return
        active = null
        completing = true
        try { request.callbacks.forEach { it(answered) } }
        finally { completing = false; advance() }
    }

    fun result(code: Int, names: Array<String>, grants: IntArray) {
        val request = active ?: return
        if (code != request.code) return
        finish(names.take(grants.size).toSet().intersect(request.names.toSet()))
    }

    fun resumed() {
        foreground = true
        // A result lost to recreation supplies no evidence of a refusal. Callers read the current grant.
        if (active != null) finish(emptySet()) else advance()
    }

    fun paused() { foreground = false }
}
