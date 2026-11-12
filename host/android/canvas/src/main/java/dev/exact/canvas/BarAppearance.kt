package dev.exact.canvas

/** Android's LIGHT appearance flags request dark icons, independently for each bar (LLP 1105 D7). */
internal data class BarAppearance(val statusLight: Boolean, val navigationLight: Boolean)

/** Chrome word 5: bit 0 is the dark scheme; bit 1 requests light status bar icons. */
internal fun barAppearance(flags: Int) = BarAppearance(
    statusLight = flags and 2 == 0,
    navigationLight = flags and 1 == 0,
)
