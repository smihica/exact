// The synthetic store @exact/purchases keeps under the agent on Android (LLP 1133.015 §9), in the drive's scratch tree,
// behaving as Play does on the agent clock: a purchase's token is kept across renewals and no expiry is reported, so
// a subscription leaves `listPurchases` at the next return to the foreground after its period, which the store keeps
// privately; a pending purchase has its token and is listed `pending`; a purchase unacknowledged three days after it
// became `purchased` is refunded and dropped, and journaled; `consume` removes a product. Times are Unix ms from the
// agent clock's epoch.
package exact.purchases

import org.json.JSONArray
import org.json.JSONObject
import java.io.File

internal class AgentStore(private val file: File, private val log: (String) -> Unit) {
    companion object {
        private const val DAY = 86_400_000.0
        private const val REFUND = 3 * DAY

        /** The agent clock as a Unix time, from the epoch the host reads for the agent's zero. */
        fun unix(now: Double): Double = (System.getenv("EXACT_AGENT_EPOCH")?.toDoubleOrNull() ?: 1_767_225_600_000.0) + now

        /** An ISO 8601 period in ms; months of 30 days, years of 365. */
        fun period(iso: String?): Double? {
            if (iso == null || !iso.startsWith("P") || iso.length < 3) return null
            val n = iso.substring(1, iso.length - 1).toDoubleOrNull() ?: return null
            return when (iso.last()) { 'D' -> n * DAY; 'W' -> n * 7 * DAY; 'M' -> n * 30 * DAY; 'Y' -> n * 365 * DAY; else -> null }
        }

        /** A catalogue item as `ItemDetails`, its absent fields the platform's. */
        fun details(item: JSONObject): JSONObject {
            val out = JSONObject(item.toString())
            if (!out.has("description")) out.put("description", "")
            if (!out.has("iconURLs")) out.put("iconURLs", JSONArray())
            if (!out.has("familyShareable")) out.put("familyShareable", false)
            if (!out.has("displayPrice")) out.optJSONObject("price")?.let { out.put("displayPrice", "${it.optString("currency")} ${it.optString("value")}") }
            for (key in listOf("subscriptionPeriod", "freeTrialPeriod", "introductoryPricePeriod", "introductoryPrice", "introductoryPriceCycles")) if (!out.has(key)) out.put(key, JSONObject.NULL)
            out.remove("group")
            out.put("apple", JSONObject.NULL)
            return out.put("android", JSONObject().put("productType", if (item.optString("type") == "subscription") "subs" else "inapp").put("subscriptionOfferDetails", JSONObject.NULL))
        }
    }

    var catalog = JSONArray()
    var fault: String? = null
    private val state: JSONObject = try { JSONObject(file.readText()) } catch (e: Exception) { JSONObject() }
    private val purchases: JSONArray get() = state.optJSONArray("purchases") ?: JSONArray().also { state.put("purchases", it) }
    private val applied: JSONArray get() = state.optJSONArray("applied") ?: JSONArray().also { state.put("applied", it) }

    /** Makes [change] and writes the store. Where it cannot be written the store is left as it was and the
     * [java.io.IOException] goes on to the call, which answers it as any failure (`NotSupportedError`). */
    private fun <T> kept(change: () -> T): T {
        val before = state.toString()
        val out = change()
        try {
            file.parentFile?.mkdirs()
            dev.exact.canvas.Files.write(file, state.toString().toByteArray())
        } catch (e: java.io.IOException) {
            val was = JSONObject(before)
            state.keys().asSequence().toList().forEach { state.remove(it) }
            was.keys().forEach { state.put(it, was.get(it)) }
            log("purchases: the store was not kept, so nothing changed: $e")
            throw e
        }
        return out
    }
    private fun rows(): List<JSONObject> = (0 until purchases.length()).map { purchases.getJSONObject(it) }

    fun item(id: String): JSONObject? = (0 until catalog.length()).map { catalog.getJSONObject(it) }.firstOrNull { it.optString("itemId") == id }
    private fun consumable(id: String) = item(id)?.optJSONObject("apple")?.optString("type") == "consumable"

    /** Play refunds what stays unacknowledged three days after it became `purchased`. */
    private fun refund(now: Double) {
        val gone = rows().filter { it.optString("state") == "purchased" && !it.optBoolean("acknowledged") && now - it.optDouble("since") >= REFUND }
        if (gone.isEmpty()) return
        kept { state.put("purchases", JSONArray(rows().filter { it !in gone })) }
        gone.forEach { log("purchases: ${it.optString("token")} (${it.optString("itemId")}) refunded: unacknowledged three days after purchase, as Play does") }
    }

    /** At a return to the foreground: each subscription whose private period has ended is dropped, as Play's next
     * query would drop it. */
    fun resumed(now: Double) {
        val ended = rows().filter { it.has("end") && it.optDouble("end") <= now }
        if (ended.isEmpty()) return
        kept { state.put("purchases", JSONArray(rows().filter { it !in ended })) }
        ended.forEach { log("purchases: ${it.optString("token")} (${it.optString("itemId")}) ended its period") }
    }

    fun owned(now: Double): List<JSONObject> { refund(now); return rows().filter { it.optString("state") == "purchased" } }
    fun unfinished(now: Double): List<JSONObject> { refund(now); return rows().filter { !it.optBoolean("acknowledged") } }

    fun buy(itemId: String, now: Double, account: String?, pending: Boolean): JSONObject? {
        if (rows().any { it.optString("itemId") == itemId }) return null
        val row = kept {
            val n = state.optInt("counter") + 1
            state.put("counter", n)
            val row = JSONObject().put("token", "agent-play-token-$n").put("itemId", itemId).put("time", now).put("acknowledged", false)
                .put("state", if (pending) "pending" else "purchased").put("since", now).put("account", account ?: JSONObject.NULL)
            AgentStore.period(item(itemId)?.optString("subscriptionPeriod"))?.let { if (item(itemId)?.optString("type") == "subscription") row.put("end", now + it) }
            purchases.put(row)
            row
        }
        log("purchases: ${row.optString("token")} for $itemId, ${row.optString("state")}, unacknowledged")
        return row
    }

    fun finish(token: String, consume: Boolean, now: Double): Boolean {
        val row = owned(now).firstOrNull { it.optString("token") == token } ?: return false
        val done = kept {
            if (consume && consumable(row.optString("itemId"))) {
                state.put("purchases", JSONArray(rows().filter { it.optString("token") != token }))
                "consumed"
            } else if (!row.optBoolean("acknowledged")) {
                row.put("acknowledged", true)
                "acknowledged"
            } else null
        }
        done?.let { log("purchases: $it $token") }
        return true
    }

    /** A change the store makes on its own, applied once by its op and token or item. */
    fun apply(event: JSONObject, now: Double): Boolean {
        val op = event.optString("op")
        val token = event.optString("purchaseToken").ifEmpty { null }
        val key = "$op ${token ?: ""} ${event.optString("itemId")}"
        if ((0 until applied.length()).any { applied.getString(it) == key }) return false
        // The event is recorded as applied in the same write as what it changes, or not at all.
        if (op == "purchase") {
            kept { applied.put(key) }
            return buy(event.optString("itemId"), now, null, false) != null
        }
        val said = kept {
            applied.put(key)
            val row = rows().firstOrNull { it.optString("token") == token }
            when (op) {
                "approve" -> if (row == null || row.optString("state") != "pending") null else { row.put("state", "purchased").put("since", now); "$token approved" }
                // Play keeps the token and its acknowledgement across a renewal; nothing enters the queue.
                "renew" -> { val p = AgentStore.period(item(row?.optString("itemId") ?: "")?.optString("subscriptionPeriod"))
                    if (row == null || p == null) null else { row.put("end", maxOf(row.optDouble("end", now), now) + p); "$token renewed, the same token" } }
                "revoke" -> if (row == null) null else { state.put("purchases", JSONArray(rows().filter { it !== row })); "$token revoked" }
                else -> "storeEvent $op is not approve, renew, revoke or purchase"
            }
        }
        said?.let { log("purchases: $it") }
        return said != null && op in setOf("approve", "renew", "revoke")
    }

    fun details(row: JSONObject): JSONObject {
        val subscription = item(row.optString("itemId"))?.optString("type") == "subscription"
        return JSONObject().put("itemId", row.optString("itemId")).put("purchaseToken", row.optString("token"))
            .put("purchaseState", row.optString("state")).put("purchaseTime", row.optDouble("time")).put("acknowledged", row.optBoolean("acknowledged"))
            .put("willAutoRenew", if (subscription) true else JSONObject.NULL).put("quantity", 1).put("expirationTime", JSONObject.NULL).put("apple", JSONObject.NULL)
            .put("android", JSONObject().put("orderId", JSONObject.NULL).put("originalJson", "{}").put("signature", "").put("packageName", "")
                .put("isAutoRenewing", subscription).put("products", JSONArray().put(row.optString("itemId"))))
    }

    fun disclosed(): String = rows().joinToString("; ") {
        "${it.optString("token")} ${it.optString("itemId")} ${it.optString("state")}${if (it.optBoolean("acknowledged")) " acknowledged" else ""}"
    }.ifEmpty { "empty" }
}
