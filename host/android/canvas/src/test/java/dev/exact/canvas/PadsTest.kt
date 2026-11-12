package dev.exact.canvas

import android.content.Context
import android.hardware.input.InputManager
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.shadows.ShadowInputDevice
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [31], manifest = Config.NONE, shadows = [PadsTest.Host::class])
class PadsTest {
    @Implements(Exact::class, isInAndroidSdk = false)
    class Host {
        @Implementation fun pad(index: Int, connected: Boolean, values: FloatArray) {
            reports += values.toList()
        }
        companion object { val reports = ArrayList<List<Float>>() }
    }

    @Before fun connect() {
        Host.reports.clear()
        val context = RuntimeEnvironment.getApplication()
        val manager = context.getSystemService(Context.INPUT_SERVICE) as InputManager
        val device = ShadowInputDevice.makeInputDeviceNamed("Controller")
        ReflectionHelpers.setField(device, "mId", 7)
        ReflectionHelpers.setField(device, "mSources", InputDevice.SOURCE_GAMEPAD or InputDevice.SOURCE_JOYSTICK)
        ReflectionHelpers.setField(device, "mMotionRanges", ArrayList<InputDevice.MotionRange>())
        shadowOf(manager).addInputDevice(device)
        Pads.attach(context)
    }

    @After fun disconnect() { Pads.onInputDeviceRemoved(7) }

    private fun key(code: Int, action: Int = KeyEvent.ACTION_DOWN, repeat: Int = 0, flags: Int = 0) =
        KeyEvent(0, 0, action, code, repeat, 0, 7, 0, flags, InputDevice.SOURCE_GAMEPAD)

    @Test fun buttonsRemainUnhandledWithoutACanvasButTheirStateIsObserved() {
        assertFalse("B must reach Android's Back fallback", Pads.key(key(KeyEvent.KEYCODE_BUTTON_B), false))
        assertEquals(1f, Pads.values(0)!![1])
        assertEquals(0, Pads.current)
        assertEquals(1f, Host.reports.last()[1])
    }

    @Test fun canvasRoutingControlsButtonsReleasesAndRepeats() {
        val requests = Requests(Robolectric.buildActivity(ExactActivity::class.java).get())
        assertFalse(requests.padInput)
        for (takes in listOf(true, false)) {
            requests.perform(intArrayOf(14, 1, 1, if (takes) 1 else 0))
            for (code in listOf(KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_DPAD_LEFT)) {
                assertEquals(takes, Pads.key(key(code), requests.padInput))
                val count = Host.reports.size
                assertEquals(takes, Pads.key(key(code, repeat = 1), requests.padInput))
                assertEquals("a repeat changes no reading", count, Host.reports.size)
                assertEquals(takes, Pads.key(key(code, action = KeyEvent.ACTION_UP), requests.padInput))
            }
        }
        assertTrue(Pads.values(0)!!.all { it == 0f })
    }

    @Test fun fallbackKeysDoNotBecomePadReadingsEvenIfCanvasFocusReturns() {
        for (takes in listOf(false, true)) for (code in listOf(
            KeyEvent.KEYCODE_DPAD_UP, KeyEvent.KEYCODE_DPAD_DOWN, KeyEvent.KEYCODE_DPAD_LEFT,
            KeyEvent.KEYCODE_DPAD_RIGHT, KeyEvent.KEYCODE_DPAD_CENTER, KeyEvent.KEYCODE_BACK,
        )) for (action in listOf(KeyEvent.ACTION_DOWN, KeyEvent.ACTION_UP)) {
            assertFalse(Pads.key(key(code, action = action, flags = KeyEvent.FLAG_FALLBACK), takes))
        }
        assertTrue(Host.reports.isEmpty())
        assertEquals(-1, Pads.current)
    }

    @Test fun remoteAndKeyboardKeysAreNotPadEvents() {
        val manager = RuntimeEnvironment.getApplication().getSystemService(Context.INPUT_SERVICE) as InputManager
        val device = ShadowInputDevice.makeInputDeviceNamed("Remote or keyboard")
        ReflectionHelpers.setField(device, "mId", 8)
        ReflectionHelpers.setField(device, "mSources", InputDevice.SOURCE_DPAD or InputDevice.SOURCE_KEYBOARD)
        shadowOf(manager).addInputDevice(device)
        for (code in listOf(KeyEvent.KEYCODE_DPAD_UP, KeyEvent.KEYCODE_DPAD_CENTER, KeyEvent.KEYCODE_A)) {
            val event = KeyEvent(0, 0, KeyEvent.ACTION_DOWN, code, 0, 0, 8, 0, 0, InputDevice.SOURCE_DPAD)
            assertFalse(Pads.key(event, true))
        }
        assertTrue(Host.reports.isEmpty())
    }

    @Test fun joystickMotionRemainsUnhandledWithoutACanvas() {
        val coordinates = MotionEvent.PointerCoords().apply {
            setAxisValue(MotionEvent.AXIS_X, 0.75f)
            setAxisValue(MotionEvent.AXIS_HAT_Y, -1f)
        }
        val event = MotionEvent.obtain(0, 0, MotionEvent.ACTION_MOVE, 1,
            arrayOf(MotionEvent.PointerProperties().apply { id = 0 }), arrayOf(coordinates),
            0, 0, 1f, 1f, 7, 0, InputDevice.SOURCE_JOYSTICK, 0)
        try {
            assertFalse("Android must synthesize navigation keys", Pads.motion(event, false))
            assertEquals(0.75f, Pads.values(0)!![17])
            assertEquals(1f, Pads.values(0)!![12])
            assertTrue("a canvas consumes motion even when the reading is unchanged", Pads.motion(event, true))
            assertFalse("leaving a canvas restores navigation", Pads.motion(event, false))
        } finally { event.recycle() }
    }
}
