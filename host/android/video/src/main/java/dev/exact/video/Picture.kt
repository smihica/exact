package dev.exact.video

import kotlin.math.max
import kotlin.math.roundToInt

/** A rect in the content box's device pixels. */
internal data class Rect(val left: Float, val top: Float, val width: Float, val height: Float)

/** The shared painter supplies the CSS object rectangle; Media3 caps renditions here (LLP 1042.001 D6). */
internal object Picture {
    /** Convert the shared placement's box fractions to the view's device pixels. */
    fun rect(placed: FloatArray?, bw: Int, bh: Int): Rect {
        if (placed == null || placed.size != 4) return Rect(0f, 0f, bw.toFloat(), bh.toFloat())
        return Rect(placed[0] * bw, placed[1] * bh, placed[2] * bw, placed[3] * bh)
    }

    /**
     * The viewport `DefaultTrackSelector` caps the rendition by (D6), or null for Media3's display default (`none`, or
     * a box with no size yet). `cover` is the whole scaled frame, of which the box shows part; `scale-down` is
     * `contain`'s box, since a rendition larger than `contain` shows no more of itself.
     */
    fun viewport(fit: String, bw: Int, bh: Int, vw: Int, vh: Int): Pair<Int, Int>? {
        if (bw <= 0 || bh <= 0 || fit == "none") return null
        if (fit != "cover" || vw <= 0 || vh <= 0) return bw to bh
        val s = max(bw.toFloat() / vw, bh.toFloat() / vh)
        return (vw * s).roundToInt() to (vh * s).roundToInt()
    }

    /** `preferredMaximumResolution`'s "WxH", or null. */
    fun resolution(text: String?): Pair<Int, Int>? {
        val parts = text?.lowercase()?.split('x') ?: return null
        if (parts.size != 2) return null
        val w = parts[0].trim().toIntOrNull() ?: return null
        val h = parts[1].trim().toIntOrNull() ?: return null
        return if (w > 0 && h > 0) w to h else null
    }
}
