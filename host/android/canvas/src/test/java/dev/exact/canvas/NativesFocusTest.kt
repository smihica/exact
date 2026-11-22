package dev.exact.canvas

import android.app.Activity
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.os.SystemClock
import android.view.KeyEvent
import android.view.View
import android.widget.Button
import android.widget.FrameLayout
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.util.ReflectionHelpers
import java.io.File
import java.lang.ref.WeakReference
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.time.Duration

/** A media controller hands the window's focus back to the reader's view when exact2's focus leaves it, by a key
 * the controller does not take or by exact2's own move (LLP 1042.001 D4). */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [31], manifest = Config.NONE)
class NativesFocusTest {
    /** A controller with one button, standing in for Media3's. */
    private class Player(context: android.content.Context, override var backgroundPlayback: Boolean = false) : VideoPlayer {
        var released = false
        var binds = 0
        val button = Button(context).apply { isFocusable = true; isFocusableInTouchMode = true }
        override val view: View = FrameLayout(context).apply { addView(button, 100, 50) }
        override val lendable = false
        override val engaged = false
        override val controller: View get() = view
        override fun bind(sink: MediaSink) { binds++ }
        override fun update(props: JSONObject, width: Int, height: Int, from: MediaRecord.Resume?) {
            if (props.optBoolean("paused")) backgroundPlayback = false
        }
        override fun shown(inView: Boolean) {}
        override fun snapshot(): MediaRecord.Resume? = null
        override fun stop() {}
        override fun release() { released = true }
    }

    /** The reader's view, which takes Tab, as exact2 moves its focus by it, and no arrow, as on a phone. */
    private class Reader(context: android.content.Context) : View(context) {
        val heard = ArrayList<Int>()
        init { isFocusable = true; isFocusableInTouchMode = true }
        override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
            if (keyCode != KeyEvent.KEYCODE_TAB) return false
            heard += keyCode
            return true
        }
    }

    private class Setup(val activity: Activity, val reader: Reader, val natives: Natives, val player: Player)

    private fun setup(): Setup {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        var made: Player? = null
        val natives = Natives(activity, { File(".") }, emptyMap(), { context -> Player(context).also { made = it } })
        val reader = Reader(activity)
        natives.reader = reader
        activity.setContentView(FrameLayout(activity).apply { addView(reader, 400, 400); addView(natives, 400, 400) })
        natives.ensure(1, 0, 100, 50, props(focused = true), 0)
        shadowOf(Looper.getMainLooper()).idle()
        val player = made!!
        player.button.requestFocus()
        return Setup(activity, reader, natives, player)
    }

    private fun props(focused: Boolean) = JSONObject()
        .put("src", "https://example.com/a.mp4").put("controls", true).put("focused", focused)
        .put("generation", 1).put("slot", 1).put("session", 1).toString()

    private fun press(s: Setup, code: Int, meta: Int = 0): Boolean {
        val down = KeyEvent(0, 0, KeyEvent.ACTION_DOWN, code, 0, meta)
        return s.activity.window.decorView.dispatchKeyEvent(down)
    }

    @Test fun tabOutOfTheControllerGoesToTheReader() {
        for (meta in listOf(0, KeyEvent.META_SHIFT_ON)) {
            val s = setup()
            assertTrue(s.player.button.isFocused)
            assertTrue(press(s, KeyEvent.KEYCODE_TAB, meta))
            assertTrue(s.reader.isFocused)
            assertEquals(listOf(KeyEvent.KEYCODE_TAB), s.reader.heard)
        }
    }

    @Test fun anArrowTheReaderDoesNotTakeLeavesTheFocusInTheController() {
        val s = setup()
        assertTrue(press(s, KeyEvent.KEYCODE_DPAD_LEFT))
        assertTrue(s.player.button.isFocused)
        assertEquals(emptyList<Int>(), s.reader.heard)
    }

    @Test fun exact2MovingItsFocusAwayGivesTheReaderTheWindowsFocus() {
        val s = setup()
        s.natives.ensure(1, 0, 100, 50, props(focused = false), 0)
        assertTrue(s.reader.isFocused)
    }

    @Test fun audioControlsWaitForTheirRowWhileBoxlessAudioGetsAPlayerImmediately() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val players = ArrayList<Player>()
        val natives = Natives(activity, { File(".") }, emptyMap(), { context ->
            Player(context).also { players.add(it) }
        })
        val audio = JSONObject(props(focused = false)).put("audio", true).toString()
        try {
            natives.lazy = true
            natives.ensure(1, 0, 100, 50, audio, 5)
            assertTrue(players.isEmpty())
            natives.ensure(2, 0, 0, 0, audio, 6)
            assertEquals(1, players.size)
            natives.near(emptySet(), emptySet(), setOf(5, 6))
            assertEquals(1, players.size)
            natives.near(setOf(5), setOf(5), setOf(5, 6))
            assertEquals(2, players.size)
            assertTrue(!players[0].released)
        } finally {
            natives.destroyAll()
        }
    }

    @Test fun anActiveAudioLoanSurvivesRecreatingTheActivity() {
        val players = ArrayList<Player>()
        fun reader(activity: Activity) = Natives(activity, { File(".") }, emptyMap(), { context ->
            Player(context, backgroundPlayback = players.isEmpty()).also { players.add(it) }
        })
        val first = Robolectric.buildActivity(Activity::class.java).setup()
        val natives = reader(first.get())
        val audio = JSONObject(props(focused = false)).put("audio", true).toString()
        natives.ensure(1, 0, 0, 0, audio, 5)
        natives.ensure(2, 0, 0, 0, audio, 0)
        val recreated = reader(Robolectric.buildActivity(Activity::class.java).setup().get())
        try {
            natives.stopMedia()
            natives.destroyAll()
            first.pause().stop().destroy()
            assertTrue(players[1].released)
            assertTrue(!players[0].released)
            recreated.ensure(1, 0, 0, 0, audio, 8)
            assertEquals(2, players.size)
            assertEquals(1, players[0].binds)
            assertTrue(!players[0].released)
            recreated.rowFreed(8)
            assertTrue(!players[0].released)
            val detached = BackgroundAudio.take(MediaAllocation(1, JSONObject(audio)))
            assertTrue(detached?.player === players[0])
            detached?.player?.release()
        } finally {
            natives.destroyAll()
            recreated.destroyAll()
            BackgroundAudio.take(MediaAllocation(1, JSONObject(audio)))?.player?.release()
        }
    }

    @Implements(Native::class, isInAndroidSdk = false)
    class AudioHost {
        companion object {
            var committed: String? = null
            var pauseAt: Long? = null
            @JvmStatic @Implementation fun __staticInitializer__() {}
            @JvmStatic @Implementation fun timerObservation(a: Boolean, b: Boolean, c: Boolean, d: Boolean) = false
            @JvmStatic @Implementation fun assetRoot(): String? = null
            @JvmStatic @Implementation fun media(id: Int, slot: Int, generation: Int, session: Long, event: String, payload: String, state: String) {
                if (event == "timeupdate") committed = JSONObject(committed!!).put("paused", true).toString()
            }
            @JvmStatic @Implementation fun backgroundAudio(id: Int, slot: Int, generation: Int, session: Long) =
                committed?.toByteArray()
            @JvmStatic @Implementation fun fds() = intArrayOf()
            @JvmStatic @Implementation fun requests(): IntArray? = null
            @JvmStatic @Implementation fun background(applying: Boolean): Double {
                val due = pauseAt?.minus(SystemClock.uptimeMillis()) ?: return -1.0
                if (due > 0) return due.toDouble()
                committed = JSONObject(committed!!).put("paused", true).toString()
                pauseAt = null
                return -1.0
            }
        }
    }

    @Test @Config(shadows = [AudioHost::class])
    fun queuedPaintCannotTakeAnAudioLoanBackBeforeABackgroundPause() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val player = Player(activity, backgroundPlayback = true)
        val natives = Natives(activity, { File(".") }, emptyMap(), { player })
        val owner = Owner(false, false, 0) { "0" }
        val reader = ExactView(activity, false) {}.also { it.started = true; it.owner = owner; it.natives = natives }
        val audio = JSONObject(props(focused = false)).put("audio", true)
        val allocation = MediaAllocation(1, audio)
        val payload = audio.toString().toByteArray()
        val stream = ByteBuffer.allocate((16 + (payload.size + 3) / 4) * 4).order(ByteOrder.LITTLE_ENDIAN)
        stream.putInt(28).putInt(1).putInt(0)
        stream.position(15 * 4)
        stream.putInt(payload.size).put(payload)
        stream.rewind()
        try {
            natives.ensure(1, 0, 0, 0, audio.toString(), 0)
            owner.queue.add(Owner.Pub(1, stream.asIntBuffer(), false, 0, 0, null, null, null, null))
            reader.running(false, host = false)
            natives.stopMedia()
            reader.onPublished()
            assertFalse(player.released)
            AudioHost.committed = audio.put("paused", true).toString()
            BackgroundAudio.reconcile { it() }
            assertTrue("the detached player received the app pause", player.released)
            assertEquals("the stopped reader left the paint queued", 1, owner.queue.size)
        } finally {
            BackgroundAudio.take(allocation)?.player?.release()
            natives.destroyAll()
            val handler = ReflectionHelpers.getField<Handler>(owner, "handler")
            val thread = ReflectionHelpers.getField<HandlerThread>(owner, "thread")
            handler.removeCallbacksAndMessages(null)
            thread.quitSafely()
            thread.join(5000)
            AudioHost.committed = null
        }
    }

    @Test @Config(shadows = [AudioHost::class])
    fun foregroundAudioRemovalSchedulesReconciliationWithoutAnOwner() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup().get()
        val player = Player(activity, backgroundPlayback = true)
        val natives = Natives(activity, { File(".") }, emptyMap(), { player })
        val audio = JSONObject(props(focused = false)).put("audio", true)
        val allocation = MediaAllocation(1, audio)
        try {
            natives.ensure(1, 0, 0, 0, audio.toString(), 0)
            natives.unrowed(emptySet())
            shadowOf(Looper.getMainLooper()).idle()
            assertTrue("the removed player was released", player.released)
        } finally {
            BackgroundAudio.take(allocation)?.player?.release()
            natives.destroyAll()
        }
    }

    @Test @Config(shadows = [AudioHost::class])
    fun aTimeupdateCanPauseDetachedAudioAfterTheMainThreadReaderIsDestroyed() {
        val activity = Robolectric.buildActivity(Activity::class.java).setup()
        val player = Player(activity.get(), backgroundPlayback = true)
        val audio = JSONObject(props(focused = false)).put("audio", true).put("paused", false)
        val allocation = MediaAllocation(1, audio)
        val loan = AudioLoan(MediaRecords.of(allocation), player)
        val previousTurned = Exact.turned
        try {
            AudioHost.committed = audio.toString()
            BackgroundAudio.detach(loan, activity.get().applicationContext)
            shadowOf(Looper.getMainLooper()).idle()
            assertFalse(player.released)
            activity.pause().stop().destroy()
            Exact.turned = {}
            loan.event("timeupdate", "1")
            shadowOf(Looper.getMainLooper()).idle()
            assertTrue("the committed pause reaches the player without a reader", player.released)
        } finally {
            BackgroundAudio.take(allocation)?.player?.release()
            MediaRecords.forget(loan.record)
            AudioHost.committed = null
            Exact.turned = previousTurned
        }
    }

    @Test @Config(shadows = [AudioHost::class])
    fun aScheduledPauseStillRunsAfterTheMainThreadReaderLeaves() {
        val activity = Robolectric.buildActivity(ExactActivity::class.java).get()
        val player = Player(activity, backgroundPlayback = true)
        val audio = JSONObject(props(focused = false)).put("audio", true).put("paused", false)
        val allocation = MediaAllocation(1, audio)
        val loan = AudioLoan(MediaRecords.of(allocation), player)
        val main = shadowOf(Looper.getMainLooper())
        val previousReader = Exact.reader
        val previousLive = Exact.live
        try {
            Exact.live = true
            Exact.reader = WeakReference(activity)
            AudioHost.committed = audio.toString()
            AudioHost.pauseAt = SystemClock.uptimeMillis() + 30_000
            BackgroundAudio.detach(loan, activity.applicationContext)
            Exact.reader = null
            main.idleFor(Duration.ofMillis(29_999))
            assertFalse(player.released)
            main.idleFor(Duration.ofMillis(1))
            assertTrue("the scheduled pause releases the detached player", player.released)
        } finally {
            BackgroundAudio.take(allocation)?.player?.release()
            MediaRecords.forget(loan.record)
            AudioHost.committed = null
            AudioHost.pauseAt = null
            Exact.live = previousLive
            Exact.reader = previousReader
        }
    }
}
