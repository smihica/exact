package dev.exact.canvas

import android.app.Activity
import android.view.MotionEvent
import android.widget.FrameLayout
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [31], manifest = Config.NONE)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class ScrollingTest {
    private class Fixture(val axis: Int) {
        val activity = Robolectric.buildActivity(Activity::class.java).get()
        val view = ExactView(activity, false) {}
        val parent = FrameLayout(activity).also { it.addView(view) }
        val scrolling = Scrolling(view)
        val scrollers = Scrolling::class.java.getDeclaredField("scrollers").apply { isAccessible = true }
        val indicate = Scrolling::class.java.getDeclaredMethod("indicate", Scrolling.Scroller::class.java).apply { isAccessible = true }
        fun scroller() = (scrollers.get(scrolling) as Map<*, *>)[1] as Scrolling.Scroller
        fun told(flags: Int) = scrolling.told(intArrayOf(1, flags, 0, 0, 0, 0, 0, 500f.toBits(), 500f.toBits()))
        fun down() {
            val e = MotionEvent.obtain(0, 0, MotionEvent.ACTION_DOWN, 100f, 100f, 0)
            scrolling.touch(e); e.recycle()
        }
        fun reply(policy: Int) = intArrayOf(0, 1, 0, 1, axis or policy, 0, 0, 0,
            500f.toBits(), 500f.toBits(), 0, 0, 200f.toBits(), 200f.toBits())
        fun under(reply: IntArray) = scrolling.under(scrolling.serial, reply)
        fun indicate() = indicate.invoke(scrolling, scroller())
    }

    @Test fun hitPolicyIsAFallbackUntilStatementsArrive() {
        for (axis in listOf(1, 2)) {
            val f = Fixture(axis)
            f.down()
            f.under(f.reply(0))
            f.indicate()
            assertEquals(2, f.parent.childCount)
            f.down()
            f.under(f.reply(8))
            assertEquals("none in a hit reply removes an indicator before any statement", 1, f.parent.childCount)
            f.indicate()
            assertEquals(1, f.parent.childCount)
            f.told(axis)
            f.indicate()
            assertEquals(2, f.parent.childCount)
            f.told(axis or 8)
            assertEquals("a live none removes the existing indicator", 1, f.parent.childCount)
            f.told(axis)
            f.indicate()
            assertEquals(2, f.parent.childCount)
        }
    }

    @Test fun anOldAutoReplyCannotOverrideANewerNoneStatement() = oldReply(0, 8)

    @Test fun anOldNoneReplyCannotOverrideANewerAutoStatement() = oldReply(8, 0)

    private fun oldReply(oldPolicy: Int, newPolicy: Int) {
        for (axis in listOf(1, 2)) {
            val f = Fixture(axis)
            f.told(axis or oldPolicy)
            f.down()
            val old = f.reply(oldPolicy)
            f.told(axis or newPolicy)
            f.under(old)
            f.indicate()
            assertEquals("the statement's policy survives an older hit reply", newPolicy != 0, f.scroller().barless)
            assertEquals(if (newPolicy == 0) 2 else 1, f.parent.childCount)
        }
    }
}
