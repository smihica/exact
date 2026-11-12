package dev.exact.canvas

import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.io.File
import java.nio.file.Files

/** An `iframe`'s and a `video`'s `src` resolve as on Apple: under the asset root, or a URL the element may load. */
class SourcesTest {
    @get:Rule val dir = TemporaryFolder()

    /** `files/app` (the asset root) beside `shared_prefs`, as the app's data directory has them. */
    private fun root(): File {
        val data = dir.root
        File(data, "shared_prefs").mkdirs()
        File(data, "shared_prefs/x.xml").writeText("<map/>")
        val app = File(data, "files/app")
        File(app, "assets/deck").mkdirs()
        File(app, "assets/deck/index.html").writeText("<p>deck</p>")
        File(app, "assets/clip.mp4").writeText("")
        return app
    }

    @Test fun aFrameSrcIsAFileUnderTheAssetRoot() {
        val app = root()
        val page = File(app, "assets/deck/index.html").canonicalFile
        assertEquals(Source.Local(page), Sources.frame("assets/deck/index.html", app))
        // A leading slash is the root's, not the device's; the query and fragment are the page's.
        assertEquals(Source.Local(page, "?slide=2#notes"), Sources.frame("/assets/deck/index.html?slide=2#notes", app))
        assertEquals(Source.Local(page), Sources.frame("assets/deck/../deck/index.html", app))
    }

    @Test fun aFrameSrcOutsideTheAssetRootIsRefused() {
        val app = root()
        assertTrue(Sources.frame("../../shared_prefs/x.xml", app) is Source.Refused)
        assertTrue(Sources.frame(File(dir.root, "shared_prefs/x.xml").path, app) is Source.Refused)
        assertTrue(Sources.frame("assets/missing.html", app) is Source.Refused)
        assertTrue(Sources.frame("assets/deck", app) is Source.Refused)
        assertTrue(Sources.frame("javascript:alert(1)", app) is Source.Refused)
        // A link inside the root that leads out of it.
        Files.createSymbolicLink(File(app, "assets/out").toPath(), File(dir.root, "shared_prefs").toPath())
        assertTrue(Sources.frame("assets/out/x.xml", app) is Source.Refused)
    }

    @Test fun aFrameLoadsAWebUrlAsGiven() {
        val app = root()
        assertEquals(Source.Remote("https://example.com/deck"), Sources.frame("https://example.com/deck", app))
        assertEquals(Source.Remote("HTTP://example.com/"), Sources.frame("HTTP://example.com/", app))
        assertEquals(Source.Remote("https://example.com/deck"), Sources.frame("//example.com/deck", app))
        assertEquals(Source.Remote("data:text/html,hi"), Sources.frame("data:text/html,hi", app))
        assertEquals(Source.Remote("about:blank"), Sources.frame("", app))
    }

    @Test fun aVideoPlaysAWebUrlAnAppFileOrAnAsset() {
        val app = root()
        val kept = File(dir.root, "files/data/episode.mp4")
        val appFile = { path: String -> if (path == "app:/data/episode.mp4") kept else null }
        assertEquals(Source.Remote("https://cdn.example/a.mp4"), Sources.video("https://cdn.example/a.mp4", app, appFile))
        assertEquals(Source.Local(kept), Sources.video("app:/data/episode.mp4", app, appFile))
        assertEquals(Source.Local(File(app, "assets/clip.mp4").canonicalFile), Sources.video("assets/clip.mp4#t=10", app, appFile))
    }

    @Test fun aVideoSrcThatNamesNothingItMayPlayIsRefused() {
        val app = root()
        val appFile = { _: String -> null as File? }
        for (src in listOf("", "app:/data/../../x.mp4", "../../shared_prefs/x.xml", "/sdcard/Movies/a.mp4", "ftp://a/b.mp4")) {
            assertTrue(src, Sources.video(src, app, appFile) is Source.Refused)
        }
    }

    @Test fun aGuestNavigatesOnlyToTheWebOrTheAssetRoot() {
        val app = root()
        assertTrue(Sources.navigable("https", "/next", app))
        assertTrue(Sources.navigable("file", File(app, "assets/deck/index.html").path, app))
        assertFalse(Sources.navigable("file", File(dir.root, "shared_prefs/x.xml").path, app))
        assertFalse(Sources.navigable("file", "${app.path}/../../shared_prefs/x.xml", app))
        assertFalse(Sources.navigable("intent", null, app))
    }

    @Test fun theErrorPageEscapesTheSource() {
        assertFalse(Sources.errorPage("<script>x</script> is not a file under /a").contains("<script>"))
    }
}
