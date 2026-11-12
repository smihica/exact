package dev.exact.app

import android.content.Context
import android.content.ContextWrapper
import android.content.res.ColorStateList
import android.content.res.Resources
import android.os.Build
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.annotation.RealObject
import org.robolectric.shadow.api.Shadow
import org.robolectric.util.ReflectionHelpers
import org.robolectric.util.ReflectionHelpers.ClassParameter

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [31], shadows = [MaterialTest.AvailableColorStateList::class])
class MaterialTest {
    private val context: Context get() = RuntimeEnvironment.getApplication()
    private val accent = 0xFFD97706.toInt()

    @Test fun missingDynamicColoursUseTheBaseline() {
        val missing = MissingColours(context)
        for (dark in listOf(false, true)) {
            val expected = baseline(dark)
            assertEquals(expected.toString(), pickerColorScheme(missing, dark, null).toString())
            val accented = pickerColorScheme(missing, dark, accent)
            assertEquals(expected.surface, accented.surface)
            assertEquals(expected.secondary, accented.secondary)
            assertEquals(expected.error, accented.error)
            assertEquals(accentRoles(dark), primaryRoles(accented))
        }
        assertTrue("dynamic colours were attempted", missing.reads > 0)
    }

    @Test fun availableDynamicColoursAreKept() {
        for (dark in listOf(false, true)) {
            val expected = if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
            assertEquals(expected.toString(), pickerColorScheme(context, dark, null).toString())
            val accented = pickerColorScheme(context, dark, accent)
            assertEquals(expected.surface, accented.surface)
            assertEquals(expected.secondary, accented.secondary)
            assertEquals(expected.error, accented.error)
            assertEquals(accentRoles(dark), primaryRoles(accented))
        }
    }

    // Simulate the older API surface on the same runtime as the dynamic-colour oracle. Refuse its colour reads
    // and API 31's tone method; this does not establish that the dialogs render on Android 10/11.
    @Test fun android10And11UseTheBaselineAndKeepTheAuthoredAccent() {
        val sdk = Build.VERSION.SDK_INT
        val missing = MissingColours(context)
        try {
            for (oldSdk in listOf(29, 30)) {
                ReflectionHelpers.setStaticField(Build.VERSION::class.java, "SDK_INT", oldSdk)
                for (dark in listOf(false, true)) {
                    val expected = baseline(dark)
                    assertEquals(expected.toString(), pickerColorScheme(missing, dark, null).toString())
                    val accented = pickerColorScheme(missing, dark, accent)
                    assertEquals(expected.surface, accented.surface)
                    assertEquals(expected.secondary, accented.secondary)
                    assertEquals(expected.error, accented.error)
                    assertEquals(accentRoles(dark), primaryRoles(accented))
                }
            }
            assertEquals("older releases never read dynamic colours", 0, missing.reads)
        } finally {
            ReflectionHelpers.setStaticField(Build.VERSION::class.java, "SDK_INT", sdk)
        }
    }

    private fun baseline(dark: Boolean) = if (dark) darkColorScheme() else lightColorScheme()

    private fun primaryRoles(s: ColorScheme) = listOf(
        s.primary, s.onPrimary, s.primaryContainer, s.onPrimaryContainer, s.inversePrimary, s.surfaceTint,
    )

    private fun accentRoles(dark: Boolean): List<Color> {
        // #D97706 at Material's primary-role tones, verified against Android 12's CAM16 conversion.
        // Newer framework withLStar implementations use a different solver from AndroidX Core 1.16.
        val colors = if (dark) {
            listOf(0xFFFFB77A, 0xFF4E2600, 0xFF6F3900, 0xFFFFDCC1, 0xFF914C00, 0xFFFFB77A)
        } else {
            listOf(0xFF914C00, 0xFFFFFFFF, 0xFFFFDCC1, 0xFF2F1500, 0xFFFFB77A, 0xFF914C00)
        }
        return colors.map { Color(it) }
    }

    private class MissingColours(context: Context) : ContextWrapper(context) {
        var reads = 0
        @Suppress("DEPRECATION")
        private val missing = object : Resources(context.assets, context.resources.displayMetrics, context.resources.configuration) {
            override fun getColor(id: Int, theme: Theme?): Int {
                reads++
                throw NotFoundException("No dynamic colour for $id")
            }
        }
        override fun getResources() = missing
    }

    @Implements(ColorStateList::class)
    class AvailableColorStateList {
        @RealObject private lateinit var colors: ColorStateList

        @Implementation fun withLStar(tone: Float): ColorStateList {
            if (Build.VERSION.SDK_INT < 31) throw NoSuchMethodError("ColorStateList.withLStar requires API 31")
            return Shadow.directlyOn(colors, ColorStateList::class.java, "withLStar", ClassParameter.from(Float::class.javaPrimitiveType, tone))
        }
    }
}
