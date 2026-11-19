// A development link (LLP 1030.000 §7): consumed by the host executables in
// every build. What it opens is the plan connection, which only a
// development build compiles (LLP 1023 §8); a production build registers no
// development scheme (`build.mjs` bakes one only with `--url`), so there it
// is the no-op at the end of this file, and LLP 1047.001 D6 keeps the rest
// development's.
import Foundation

#if EXACT_INSPECT
/// Explicit development-only opening action (LLP 1030.000 §7). The locator
/// stays an HTTP(S) URL; the outer app-specific scheme only selects the client.
/// The scheme is derived from the public app id and the loader's identity and
/// compatibility checks are computable by anyone, so neither is trust: a link
/// is admitted only for the dev-server origins this build was made against,
/// carrying the random token baked beside them (`host/apple/build.mjs`).
public enum ExactDevelopmentLink {
    public static func page(_ link: URL, scheme: String, origins: [String], token: String) -> URL? {
        guard link.absoluteString.utf8.count <= 8192, validToken(token),
              let c = URLComponents(url: link, resolvingAgainstBaseURL: false),
              c.scheme?.lowercased() == scheme, c.host == "open",
              c.user == nil, c.password == nil, c.port == nil, c.fragment == nil,
              c.path.isEmpty || c.path == "/",
              let items = c.queryItems, items.count == 2, Set(items.map(\.name)) == ["url", "token"],
              let offered = items.first(where: { $0.name == "token" })?.value, equal(offered, token),
              let value = items.first(where: { $0.name == "url" })?.value, let page = URL(string: value),
              ["http", "https"].contains(page.scheme?.lowercased() ?? ""),
              let host = page.host, !host.isEmpty, page.user == nil, page.password == nil,
              origins.contains(where: { URL(string: $0).map { sameOrigin($0, page) } ?? false })
        else { return nil }
        return page
    }

    /// The scheme, origins and token a development build was baked with, or
    /// nil: an installed or production build registers no development link.
    private static var baked: (scheme: String, origins: [String], token: String)? {
        let info = Bundle.main.infoDictionary ?? [:]
        guard let scheme = info["ExactDevelopmentURLScheme"] as? String, !scheme.isEmpty,
              let origins = info["ExactDevelopmentOrigins"] as? [String], !origins.isEmpty,
              let token = info["ExactDevelopmentToken"] as? String, validToken(token) else { return nil }
        return (scheme.lowercased(), origins, token)
    }

    public static func page(_ link: URL) -> URL? {
        guard let baked else { return nil }
        return page(link, scheme: baked.scheme, origins: baked.origins, token: baked.token)
    }

    /// A link in this build's development scheme is consumed here, admitted
    /// or refused; it never becomes an app location (LLP 1038 D11).
    public static func claims(_ link: URL) -> Bool {
        guard let baked else { return false }
        return link.scheme?.lowercased() == baked.scheme
    }

    /// True when the link was this build's to take — connected, or refused
    /// with a line on stderr and every session left as it was.
    @discardableResult
    public static func open(_ link: URL) -> Bool {
        guard claims(link) else { return false }
        guard let page = page(link) else {
            fputs("exact: refused a development link that does not name this build's dev server and token\n", stderr)
            return true
        }
        // The existing loader still checks app identity, pairing, grants and
        // runtime compatibility before changing any session.
        ExactApp.shared.connect(page.absoluteString)
        return true
    }

    private static func validToken(_ token: String) -> Bool {
        token.utf8.count == 64 && token.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
    }

    /// Constant time over the token's length.
    private static func equal(_ a: String, _ b: String) -> Bool {
        let x = Array(a.utf8), y = Array(b.utf8)
        guard x.count == y.count else { return false }
        return zip(x, y).reduce(UInt8(0)) { $0 | ($1.0 ^ $1.1) } == 0
    }

    /// URL.origin, including the scheme's default port.
    private static func sameOrigin(_ a: URL, _ b: URL) -> Bool {
        func port(_ u: URL) -> Int? {
            switch u.scheme?.lowercased() { case "http": return u.port ?? 80; case "https": return u.port ?? 443; default: return nil }
        }
        guard let pa = port(a), let ha = a.host?.lowercased(), !ha.isEmpty else { return false }
        return a.scheme?.lowercased() == b.scheme?.lowercased() && ha == b.host?.lowercased() && pa == port(b)
    }
}
#else
/// A production build's: it has no development scheme, so it admits, claims
/// and opens no link.
public enum ExactDevelopmentLink {
    public static func page(_ link: URL, scheme: String, origins: [String], token: String) -> URL? { nil }
    public static func page(_ link: URL) -> URL? { nil }
    public static func claims(_ link: URL) -> Bool { false }
    @discardableResult public static func open(_ link: URL) -> Bool { false }
}
#endif
