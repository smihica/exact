package dev.exact.canvas

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import dev.exact.canvas.ExtensionInvokeTest.Host
import exact.calendar.CalendarModule
import exact.contacts.ContactsModule
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [31], manifest = Config.NONE, shadows = [Host::class])
class ContactsCalendarStatusTest {
    @Test fun contactsStatusAloneFollowsSettingsOnForeground() {
        followsSettings("contacts", "contactsStatus", Manifest.permission.READ_CONTACTS, ::ContactsModule)
    }

    @Test fun calendarStatusAloneFollowsSettingsOnForeground() {
        followsSettings("calendar", "calendarStatus", Manifest.permission.READ_CALENDAR, ::CalendarModule)
    }

    private fun followsSettings(
        name: String,
        status: String,
        permission: String,
        make: (ExtensionContext) -> ExactExtension,
    ) {
        val app = RuntimeEnvironment.getApplication()
        val info = app.packageManager.getPackageInfo(app.packageName, PackageManager.GET_PERMISSIONS)
        info.requestedPermissions = arrayOf(permission)
        shadowOf(app.packageManager).installPackage(info)
        val history = app.getSharedPreferences("exact.permissions", Context.MODE_PRIVATE)
        fun deny() {
            shadowOf(app).denyPermissions(permission)
            history.edit().putBoolean(permission, true).commit()
        }
        deny()
        val module = make(ExtensionContext(app, name))
        var shown = JSONObject()
        fun read() { module.later(status, JSONObject()) { shown = it } }
        read()
        assertEquals("denied", shown.getString("state"))
        assertEquals("none", shown.getString("access"))

        val topics = ArrayList<String>()
        Host.onChanged = { topic ->
            topics += topic
            if (topic == "$name.$status") read()
        }
        try {
            for (granted in listOf(true, false, true)) {
                topics.clear()
                val before = shown.toString()
                module.foreground(false)
                if (granted) shadowOf(app).grantPermissions(permission) else deny()
                assertTrue("background changes are not announced", topics.isEmpty())
                assertEquals(before, shown.toString())

                module.foreground(true)
                assertEquals(if (granted) "granted" else "denied", shown.getString("state"))
                assertEquals(if (granted) "full" else "none", shown.getString("access"))
                assertTrue("the status reader is notified", topics.isNotEmpty())
                assertTrue("no list was read", topics.all { it == "$name.$status" })
            }
        } finally {
            Host.onChanged = {}
        }
    }
}
