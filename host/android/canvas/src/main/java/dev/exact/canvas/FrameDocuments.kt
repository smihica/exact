package dev.exact.canvas

import android.webkit.WebView
import androidx.webkit.JavaScriptReplyProxy
import androidx.webkit.ScriptHandler
import androidx.webkit.WebMessageCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import org.json.JSONObject
import java.util.UUID
import java.util.WeakHashMap

/** Observe window load on WebView providers that have messaging and no navigation listener. */
internal class FrameDocuments(private val view: WebView, loaded: () -> Unit) {
    private val documentStart = WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)
    private var startObserver: ScriptHandler? = null
    private var token = UUID.randomUUID().toString()
    // Each document gets its own injected object and reply proxy.
    private val reported = WeakHashMap<JavaScriptReplyProxy, Unit>()

    init {
        WebViewCompat.addWebMessageListener(view, "__exactFrameLoad", setOf("*")) { _, message, _, main, document ->
            if (main && message.type == WebMessageCompat.TYPE_STRING && message.data == token &&
                reported.put(document, Unit) == null) loaded()
        }
    }

    fun reset() {
        startObserver?.remove()
        startObserver = null
        token = UUID.randomUUID().toString()
        reported.clear()
    }

    fun observeFromStart() {
        if (!documentStart || startObserver != null) return
        // A guest can navigate from its own load handler before a reader callback runs.
        startObserver = WebViewCompat.addDocumentStartJavaScript(view, script("false", waitForLoad = true), setOf("*"))
    }

    fun observe(url: String, atFinish: Boolean = false) {
        view.evaluateJavascript(script(frameDocumentReady(url), waitForLoad = !atFinish), null)
    }

    private fun script(readiness: String, waitForLoad: Boolean) = """
        (() => {
            if (window !== window.top) return;
            const ready = $readiness;
            if (ready === null) return;
            const bridge = window.__exactFrameLoad;
            if (!bridge) return;
            const report = bridge.postMessage.bind(bridge, "$token");
            if (ready) report();
            else if ($waitForLoad) window.addEventListener('load', event => {
                if (event.isTrusted && event.target === document) report();
            }, {capture: true});
        })()
    """.trimIndent()
}

/** A guest can shadow document properties and global constructors. */
internal fun frameDocumentReady(url: String) = """
    (() => {
        const {getPrototypeOf, getOwnPropertyDescriptor} = ({}).constructor;
        const read = name => {
            for (let proto = getPrototypeOf(document); proto; proto = getPrototypeOf(proto)) {
                const property = getOwnPropertyDescriptor(proto, name);
                if (property && property.get) return property.get.call(document);
            }
        };
        return read('URL') === ${JSONObject.quote(url)} ? read('readyState') === 'complete' : null;
    })()
""".trimIndent()
