// @ref LLP 1126 D8 — the app publishes, an extension pins and only reads.
// The App Group's root and the pinning of a published generation, which the
// app, its extension and the watch read; the publication is the widgets
// family's (ExactWidgetPublication).
import Foundation
import CommonCrypto

public enum WidgetContainer {
    public static var appID: String? { Bundle.main.object(forInfoDictionaryKey: "ExactAppIdentifier") as? String }
    public static var root: URL? {
        if let appID, getenv("EXACT_APP_GROUP_APP").map({ String(cString: $0) }) == appID,
           let path = getenv("EXACT_APP_GROUP_ROOT") { return URL(fileURLWithPath: String(cString: path)) }
        guard let group = Bundle.main.object(forInfoDictionaryKey: "ExactAppGroup") as? String else { return nil }
        return FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group)
    }
    /// Called before any session. Only widget-enabled binaries carry these keys.
    public static func configure() {
        guard let root, let appID else { return }
        setenv("EXACT_APP_GROUP_ROOT", root.path, 1)
        setenv("EXACT_APP_GROUP_APP", appID, 1)
        if Bundle.main.object(forInfoDictionaryKey: "ExactWidgetKinds") != nil {
            setenv("EXACT_WIDGET_WRITER", appID, 1)
        }
    }
    /// The updater holds lifetime file locks; only published generations belong in the App Group.
    public static var updateBase: URL? { FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first }
    /// The baked compatibility id (LLP 1030 D3a): the cohort a generation belongs to.
    package static var compatibility: String {
        let object = (try? JSONSerialization.jsonObject(with: Runtime.bakedCompat())) as? [String: Any]
        return object?["id"] as? String ?? ""
    }
    /// A generation's name: the digest of its cohort and its publication
    /// (a signed bake or accepted delivery identifies the bytes already).
    /// An embedded bundle's is the one its `ExactWidgetGeneration` makes.
    package static func generationDigest(compatibility: String, publicationID: String) -> String {
        let stamp = Data((compatibility + "\n" + publicationID).utf8)
        var hash = [UInt8](repeating: 0, count: Int(CC_SHA256_DIGEST_LENGTH))
        _ = stamp.withUnsafeBytes { CC_SHA256($0.baseAddress, CC_LONG(stamp.count), &hash) }
        return hash.map { String(format: "%02x", $0) }.joined()
    }
    /// No lock, selection, repair or network access: read the pointer once and
    /// keep its immutable directory for this entire invocation. Stage 1 keeps
    /// all published generations; reclamation needs cross-process leases first.
    public static func pin(generation: String? = nil) throws -> URL {
        guard let root else { throw ExactFailure(description: "widget App Group is unavailable") }
        return try pin(root: root, generation: generation, compatibility: compatibility, embedded: embeddedGeneration)
    }
    /// The bundle's own embedded plan, `ExactWidgetGeneration`: the app's
    /// and its extension's are one build's.
    public static var embeddedGeneration: String? { Bundle.main.object(forInfoDictionaryKey: "ExactWidgetGeneration") as? String }
    /// A published generation is pinned only when the binary that
    /// published it is this one (`embedded`): after a binary update the
    /// published plan is the old app's until the new app launches and
    /// publishes, and the extension's embedded copy is the newer, so a
    /// mismatch refuses and the caller falls back to it.
    package static func pin(root: URL, generation: String? = nil, compatibility: String, embedded: String? = nil) throws -> URL {
        let base = root.appendingPathComponent("widget-generations", isDirectory: true)
        let name = try generation ?? String(contentsOf: base.appendingPathComponent("current"), encoding: .utf8)
        guard name.count == 64, name.allSatisfy({ $0.isHexDigit }) else { throw CocoaError(.fileReadCorruptFile) }
        let directory = base.appendingPathComponent(name, isDirectory: true)
        guard try String(contentsOf: directory.appendingPathComponent("compatibility"), encoding: .utf8) == compatibility else {
            throw ExactFailure(description: "widget generation belongs to another compatibility cohort")
        }
        if let embedded, (try? String(contentsOf: directory.appendingPathComponent("embedded"), encoding: .utf8)) != embedded {
            throw ExactFailure(description: "widget generation was published by another binary")
        }
        return directory
    }
}
