package dev.exact.canvas

import org.junit.Assert.*
import org.junit.Test

class PermissionRequestsTest {
    private class Dialog(val names: List<String>, val code: Int)
    private class Fixture {
        val dialogs = ArrayList<Dialog>()
        val answers = ArrayList<Pair<String, Set<String>>>()
        var available = true
        val requests = PermissionRequests { names, code ->
            if (available) { dialogs += Dialog(names.toList(), code); true } else false
        }
        fun ask(vararg names: String) = requests.ask(arrayOf(*names)) { answers += names.first() to it }
        fun answer(index: Int, names: List<String> = dialogs[index].names) =
            requests.result(dialogs[index].code, names.toTypedArray(), IntArray(names.size) { -1 })
    }

    @Test fun overlappingRequestsKeepTheirOwnPermissionsAndAnswers() {
        val f = Fixture()
        f.requests.resumed()
        f.ask("fine", "coarse")
        f.ask("steps")
        f.requests.paused()
        f.ask("camera")
        assertEquals(listOf(listOf("fine", "coarse")), f.dialogs.map { it.names })
        assertTrue(f.answers.isEmpty())
        f.answer(0)
        assertEquals(listOf("fine" to setOf("fine", "coarse")), f.answers)
        assertEquals(1, f.dialogs.size)
        f.requests.resumed()
        assertEquals(listOf("steps"), f.dialogs[1].names)
        f.answer(1)
        assertEquals(listOf("camera"), f.dialogs[2].names)
        f.answer(2)
        assertEquals(listOf("fine", "steps", "camera"), f.answers.map { it.first })
        assertEquals(setOf("steps"), f.answers[1].second)
        assertEquals(setOf("camera"), f.answers[2].second)
    }

    @Test fun unpresentedRequestsDoNotRecordPermanentDenials() {
        val f = Fixture()
        val denied = HashSet<String>()
        fun ask() = f.requests.ask(arrayOf("steps")) { answered ->
            // No grant and no rationale also describe a permission the person has never seen.
            if ("steps" in answered) denied += "steps"
        }
        ask()
        assertTrue(denied.isEmpty())
        f.requests.resumed()
        f.available = false
        ask()
        assertTrue(denied.isEmpty())
        f.available = true
        ask()
        f.answer(0, emptyList())
        assertTrue(denied.isEmpty())
        ask()
        f.answer(1)
        assertEquals(setOf("steps"), denied)
    }

    @Test fun missingResultsSettleWithoutDenialAndLateResultsDoNotSettleTheNextRequest() {
        val f = Fixture()
        f.requests.resumed()
        f.ask("location")
        f.requests.paused()
        f.ask("steps")
        f.requests.resumed()
        assertEquals(listOf("location" to emptySet<String>()), f.answers)
        assertEquals(listOf("steps"), f.dialogs[1].names)
        f.answer(0)
        assertEquals(1, f.answers.size)
        f.answer(1)
        assertEquals("steps" to setOf("steps"), f.answers.last())
    }

    @Test fun onlyPermissionsWithResultsAreReported() {
        val f = Fixture()
        f.requests.resumed()
        f.ask("fine", "coarse")
        f.requests.result(f.dialogs[0].code, arrayOf("fine", "coarse", "unasked"), intArrayOf(-1))
        assertEquals(listOf("fine" to setOf("fine")), f.answers)
    }

    @Test fun aCallbackCanAskAgainWithoutOvertakingTheQueue() {
        val f = Fixture()
        f.requests.resumed()
        f.requests.ask(arrayOf("location")) { f.ask("camera") }
        f.ask("steps")
        f.answer(0)
        assertEquals(listOf(listOf("location"), listOf("steps")), f.dialogs.map { it.names })
        f.answer(1)
        f.answer(2)
        assertEquals(listOf("steps", "camera"), f.answers.map { it.first })
    }

    @Test fun aRequestChainedFromAResultWaitsForTheActivityToResume() {
        val f = Fixture()
        f.requests.resumed()
        f.requests.ask(arrayOf("fine", "coarse")) { f.ask("background") }
        f.requests.paused()
        f.answer(0)
        assertTrue(f.answers.isEmpty())
        assertEquals(1, f.dialogs.size)
        f.requests.resumed()
        assertEquals(listOf("background"), f.dialogs[1].names)
        f.answer(1)
        assertEquals(listOf("background" to setOf("background")), f.answers)
    }

    @Test fun identicalPermissionSetsShareTheActiveOrQueuedDecision() {
        val f = Fixture()
        f.requests.resumed()
        f.ask("fine", "coarse")
        f.requests.paused()
        f.ask("coarse", "fine")
        f.ask("steps")
        f.ask("steps")
        f.answer(0)
        assertEquals(listOf("fine", "coarse"), f.answers.map { it.first })
        assertTrue(f.answers.all { it.second == setOf("fine", "coarse") })
        f.requests.resumed()
        f.answer(1)
        assertEquals(2, f.dialogs.size)
        assertEquals(listOf("fine", "coarse", "steps", "steps"), f.answers.map { it.first })
    }

}
