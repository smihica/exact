package dev.exact.video

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** The picture's rect and the rendition's viewport (LLP 1042.001 D5, D6). */
class PictureTest {
    private fun near(want: Rect, got: Rect) {
        assertEquals(want.left, got.left, 0.01f); assertEquals(want.top, got.top, 0.01f)
        assertEquals(want.width, got.width, 0.01f); assertEquals(want.height, got.height, 0.01f)
    }

    @Test fun sharedPlacementScalesToDevicePixels() {
        near(Rect(0f, 60f, 40f, 20f), Picture.rect(floatArrayOf(0f, .75f, .25f, .25f), 160, 80))
        near(Rect(0f, 120f, 80f, 40f), Picture.rect(floatArrayOf(0f, .75f, .25f, .25f), 320, 160))
        near(Rect(-80f, 0f, 320f, 80f), Picture.rect(floatArrayOf(-.5f, 0f, 2f, 1f), 160, 80))
    }

    @Test fun anUnknownSizeIsTheBox() =
        assertEquals(Rect(0f, 0f, 1000f, 1012f), Picture.rect(null, 1000, 1012))

    @Test fun theViewportIsTheBoxButForCoverAndNone() {
        assertEquals(974 to 1090, Picture.viewport("contain", 974, 1090, 1920, 1080))
        assertEquals(974 to 1090, Picture.viewport("scale-down", 974, 1090, 1920, 1080))
        assertEquals(974 to 1090, Picture.viewport("fill", 974, 1090, 1920, 1080))
        assertEquals(1938 to 1090, Picture.viewport("cover", 974, 1090, 1920, 1080))
        assertEquals(974 to 1090, Picture.viewport("cover", 974, 1090, 0, 0))
        assertNull(Picture.viewport("none", 974, 1090, 1920, 1080))
        assertNull(Picture.viewport("contain", 0, 0, 1920, 1080))
    }

    @Test fun resolutionReadsWidthByHeight() {
        assertEquals(960 to 540, Picture.resolution("960x540"))
        assertNull(Picture.resolution("960"))
        assertNull(Picture.resolution(null))
        assertNull(Picture.resolution("0x540"))
    }
}
