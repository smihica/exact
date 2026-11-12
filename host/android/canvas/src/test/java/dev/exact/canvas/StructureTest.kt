package dev.exact.canvas

import java.nio.IntBuffer
import org.junit.Assert.*
import org.junit.Test

class StructureTest {
    @Test fun clipMaskAndTintRemainInTheDrawingSignature() {
        // A two-child union (each child has its own fill rule), a mask layer,
        // and a transparent tint followed by both image draw forms.
        val union = intArrayOf(45, 10, 2, 0, 2, 0, 0, 0, 3, 1, 1, 3, 6)
        val mask = intArrayOf(7, 1f.toRawBits(), 46, 2, -1) + IntArray(12) + intArrayOf(6, 6)
        val images = intArrayOf(47, 0, 8, 7, 0, 0, 0, 0, 47, -65536, 14, 7) + IntArray(16)
        val words = union + mask + images
        val b = IntBuffer.wrap(words)
        val s = Structure()
        assertEquals(union.size - 1, s.end(b, 0))
        assertArrayEquals(words, s.read(b) { it })
    }

    @Test fun aMaskAndClipUnionDoNotEndTheirContainingOpacityLayer() {
        val words = intArrayOf(7, 1f.toRawBits(), 45, 1, 0, 46, 6, 6, 6)
        val s = Structure()
        val b = IntBuffer.wrap(words)
        assertEquals(8, s.layerEnd(b, 2, words.size))
        assertEquals(7, s.layerEnd(b, 5, words.size))
        assertEquals(6, s.layerEnd(b, 6, words.size))
        assertEquals(-1, s.layerEnd(b, 2, words.size - 1))
    }
}
