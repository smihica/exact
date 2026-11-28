// A received share's claim (LLP 1133.024 D2–D4): the Rust host claims it,
// sets its tokens, delivers its location at the navigation root and lands
// its files in `app:/tmp/shared/`. A share comes from the share extension's
// inbox in the App Group, claimed at launch, at each foreground and on the
// extension's Darwin notification; under the agent the driver's `share:`
// form arrives here too (D9). A host never takes `share:` from outside it.
#if canImport(UIKit)
import UIKit
#else
import AppKit
#endif
import Darwin
import Foundation
import ExactKit

/// The share extension's inbox (D4): `<group>/shares/<uuid>/`, each complete
/// once its `share.json` is there and its writer has released its publication lock.
struct ShareInbox {
    /// The app's `share_target`, which the bake wrote into the Info.plist.
    let target: Any
    let directory: URL
    /// Private writer staging directory (`<group>/staging/`) when using the real
    /// App Group container; nil for agent overrides so drives never sweep outside
    /// their fixture directory.
    let stagingDirectory: URL?

    init(target: Any, directory: URL, stagingDirectory: URL? = nil) {
        self.target = target
        self.directory = directory
        self.stagingDirectory = stagingDirectory
    }

    /// The app's inbox, when it declares `share_target` and is entitled to
    /// the group (`ExactShareGroup` is written only then). Under the agent,
    /// `EXACT_AGENT_SHARE_INBOX` names another directory: a drive's own,
    /// apart from the shares a person left (and, on an ad-hoc Mac build,
    /// where there is no group).
    static var main: ShareInbox? {
        guard let json = Bundle.main.object(forInfoDictionaryKey: "ExactShareTarget") as? String,
              let target = try? JSONSerialization.jsonObject(with: Data(json.utf8)) else { return nil }
        if ExactEnv.agentMode, let path = ExactEnv.environment["EXACT_AGENT_SHARE_INBOX"] {
            return ShareInbox(target: target, directory: URL(fileURLWithPath: path, isDirectory: true), stagingDirectory: nil)
        }
        guard let group = Bundle.main.object(forInfoDictionaryKey: "ExactShareGroup") as? String,
              let container = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group) else { return nil }
        return ShareInbox(target: target, directory: container.appendingPathComponent("shares", isDirectory: true),
                          stagingDirectory: container.appendingPathComponent("staging", isDirectory: true))
    }

    /// The complete, unlocked shares, oldest first (D8's order); a directory without
    /// `share.json` or still locked by `ShareInboxWriter` or another consumer is skipped.
    func complete() -> [URL] {
        completeChecked().entries
    }

    /// Scans `directory` in FIFO order, reporting `hadTransientError` if reading the
    /// inbox or stating an entry's `share.json` failed transiently so callers retry
    /// instead of letting newer shares overtake an unreadable older share.
    func completeChecked() -> (entries: [URL], hadTransientError: Bool) {
        let dirs: [URL]
        do {
            dirs = try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
        } catch let err as NSError {
            if err.domain == NSCocoaErrorDomain && err.code == NSFileReadNoSuchFileError {
                return ([], false)
            }
            if err.domain == NSPOSIXErrorDomain && err.code == Int(ENOENT) {
                return ([], false)
            }
            return ([], true)
        }
        var dated: [(URL, TimeInterval)] = []
        for dir in dirs {
            let record = dir.appendingPathComponent("share.json")
            var st = stat()
            let rc: Int32 = record.withUnsafeFileSystemRepresentation { path in
                guard let path else { errno = EINVAL; return -1 }
                while true {
                    let r = lstat(path, &st)
                    if r < 0 && errno == EINTR { continue }
                    return r
                }
            }
            if rc != 0 {
                let err = errno
                if err == ENOENT || err == ENOTDIR { continue }
                // A transient stat failure on an entry could belong to the oldest share;
                // stop scanning rather than letting newer entries overtake it.
                return ([], true)
            }
            switch Self.probeLock(dir) {
            case .unlocked:
                let mtime = Double(st.st_mtimespec.tv_sec) + Double(st.st_mtimespec.tv_nsec) / 1e9
                dated.append((dir, mtime))
            case .locked:
                continue
            case .transientError:
                return ([], true)
            }
        }
        return (dated.sorted { $0.1 < $1.1 }.map(\.0), false)
    }

    /// Upper bound on `share.json` size read from the inbox (4 MB).
    static let recordCap = 4 << 20

    /// Outcome of reading a locked inbox directory's `share.json`.
    enum RequestRead {
        case ok([String: Any])
        /// Corrupt, symlink, or oversized record: caller must retire it (lock still held).
        case corrupt
        /// `share.json` is absent (`ENOENT`): skip and continue scanning.
        case missing
        /// Transient I/O error: keep intact and retry later without letting newer shares overtake.
        case transientError
    }

    /// Pending retirement whose lock stays open for retry without re-claiming.
    enum PendingRetire {
        /// Entry is still live in `shares/` (`moveItem` and fallback unlink/sync failed).
        case live(dir: URL, lockFd: Int32)
        /// Entry was moved to `staging/done-*`, awaiting durable parent-directory syncs and removal.
        case moved(retired: URL, lockFd: Int32)

        var lockFd: Int32 {
            switch self {
            case .live(_, let fd), .moved(_, let fd): return fd
            }
        }
    }

    /// Outcome of acquiring a non-blocking exclusive `flock` on an inbox directory.
    enum LockOutcome {
        case acquired(Int32)
        /// Held by another consumer/writer or already removed: safe to skip.
        case busy
        /// Transient I/O/descriptor error: stop scanning and retry to preserve FIFO.
        case transientError
    }

    private enum LockProbe {
        case unlocked, locked, transientError
    }

    private enum InodeMatch {
        case matched, mismatched, transientError
    }

    private static func probeLock(_ dir: URL) -> LockProbe {
        switch tryAcquireLock(dir) {
        case .acquired(let fd):
            close(fd)
            return .unlocked
        case .busy:
            return .locked
        case .transientError:
            return .transientError
        }
    }

    /// Acquires a non-blocking exclusive `flock` on `dir` so only one consumer owns
    /// the directory from claim through retirement; returns nil on contention or error.
    static func acquireLock(_ dir: URL) -> Int32? {
        if case .acquired(let fd) = tryAcquireLock(dir) { return fd }
        return nil
    }

    static func tryAcquireLock(_ dir: URL) -> LockOutcome {
        let fd = dir.withUnsafeFileSystemRepresentation { path -> Int32 in
            guard let path else { errno = EINVAL; return -1 }
            while true {
                let opened = open(path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
                if opened < 0 && errno == EINTR { continue }
                return opened
            }
        }
        guard fd >= 0 else {
            let err = errno
            return (err == ENOENT || err == ENOTDIR || err == ELOOP) ? .busy : .transientError
        }
        while true {
            let rc = flock(fd, LOCK_EX | LOCK_NB)
            if rc == 0 { break }
            if errno == EINTR { continue }
            let err = errno
            close(fd)
            return (err == EWOULDBLOCK || err == EAGAIN) ? .busy : .transientError
        }
        switch Self.matchInode(fd, url: dir) {
        case .matched:
            return .acquired(fd)
        case .mismatched:
            close(fd)
            return .busy
        case .transientError:
            close(fd)
            return .transientError
        }
    }

    /// Verifies `fd` still refers to the directory inode currently linked at `url`,
    /// distinguishing confirmed missing/replaced entries from transient I/O errors.
    private static func matchInode(_ fd: Int32, url: URL) -> InodeMatch {
        var fdStat = stat()
        while true {
            if fstat(fd, &fdStat) == 0 { break }
            if errno == EINTR { continue }
            return .transientError
        }
        guard (fdStat.st_mode & S_IFMT) == S_IFDIR else { return .mismatched }
        var pathStat = stat()
        let rc: Int32 = url.withUnsafeFileSystemRepresentation { path in
            guard let path else { errno = EINVAL; return -1 }
            while true {
                let r = lstat(path, &pathStat)
                if r < 0 && errno == EINTR { continue }
                return r
            }
        }
        if rc != 0 {
            let err = errno
            return (err == ENOENT || err == ENOTDIR) ? .mismatched : .transientError
        }
        return (fdStat.st_dev == pathStat.st_dev && fdStat.st_ino == pathStat.st_ino)
            ? .matched
            : .mismatched
    }

    /// The bridge's request for the share in `dir`, assuming the caller already holds
    /// an exclusive lock on `dir`. Opens `share.json` via `openat(lockFd, ...)` with
    /// `O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC`, validates regular-file type and `recordCap`,
    /// includes the locked directory's `(dev, ino)` so Rust binds attachment opens to the
    /// same inode, reports `.corrupt` for permanent defects (leaving `lockFd` open for the
    /// caller to retire), and flags transient I/O errors for retry.
    func requestLocked(_ dir: URL, lockFd: Int32) -> RequestRead {
        var dirStat = stat()
        guard fstat(lockFd, &dirStat) == 0, (dirStat.st_mode & S_IFMT) == S_IFDIR else {
            return .transientError
        }
        let fd: Int32 = {
            while true {
                let opened = openat(lockFd, "share.json", O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC)
                if opened < 0 && errno == EINTR { continue }
                return opened
            }
        }()
        if fd < 0 {
            if errno == ELOOP { return .corrupt }
            if errno == ENOENT { return .missing }
            return .transientError
        }
        defer { close(fd) }
        var st = stat()
        guard fstat(fd, &st) == 0 else { return .transientError }
        guard (st.st_mode & S_IFMT) == S_IFREG, st.st_size <= Self.recordCap else {
            return .corrupt
        }
        var data = Data()
        var buf = [UInt8](repeating: 0, count: 16 * 1024)
        while true {
            let n = buf.withUnsafeMutableBytes { ptr -> ssize_t in
                while true {
                    let r = read(fd, ptr.baseAddress, ptr.count)
                    if r < 0 && errno == EINTR { continue }
                    return r
                }
            }
            if n == 0 { break }
            if n < 0 { return .transientError }
            data.append(buf, count: Int(n))
            if data.count > Self.recordCap { return .corrupt }
        }
        guard let record = try? JSONSerialization.jsonObject(with: data) else {
            return .corrupt
        }
        return .ok([
            "target": target,
            "share": record,
            "inbox": dir.path,
            "inboxInode": [UInt64(dirStat.st_dev), UInt64(dirStat.st_ino)],
        ])
    }

    /// Test-friendly helper that acquires a temporary lock and reads `share.json`.
    func request(_ dir: URL) -> [String: Any]? {
        guard let fd = Self.acquireLock(dir) else { return nil }
        switch requestLocked(dir, lockFd: fd) {
        case .ok(let req):
            close(fd)
            return req
        case .corrupt:
            retire(dir, lockFd: fd)
            return nil
        case .missing, .transientError:
            close(fd)
            return nil
        }
    }

    /// Atomically retires a delivered (or corrupt) share out of `shares/` and releases
    /// `lockFd`. Returns `nil` once retirement is durable (lock closed), or a
    /// `PendingRetire` retaining `lockFd` if moving or syncing failed transiently.
    func retireOrDefer(_ dir: URL, lockFd: Int32) -> PendingRetire? {
        guard lockFd >= 0 else { return nil }
        switch Self.matchInode(lockFd, url: dir) {
        case .matched: break
        case .mismatched:
            close(lockFd)
            return nil
        case .transientError:
            return .live(dir: dir, lockFd: lockFd)
        }
        let fm = FileManager.default
        if let staging = stagingDirectory {
            try? fm.createDirectory(at: staging, withIntermediateDirectories: true)
            let retired = staging.appendingPathComponent("done-\(UUID().uuidString)-\(dir.lastPathComponent)", isDirectory: true)
            if (try? fm.moveItem(at: dir, to: retired)) != nil {
                return finishMovedRetire(retired, lockFd: lockFd)
            }
        }
        // Fallback (e.g. agent fixture without stagingDirectory, or move failure):
        // unlink `share.json` via `unlinkat(lockFd)` and sync `lockFd` directly;
        // refuse to delete attachments if unlinking or syncing the directory fails.
        while true {
            if unlinkat(lockFd, "share.json", 0) == 0 { break }
            if errno == EINTR { continue }
            if errno == ENOENT { break }
            fputs("exact: share: could not unlink share.json in \(dir.lastPathComponent)\n", stderr)
            return .live(dir: dir, lockFd: lockFd)
        }
        guard Self.syncFd(lockFd) else {
            fputs("exact: share: could not sync \(dir.lastPathComponent) after unlinking share.json\n", stderr)
            return .live(dir: dir, lockFd: lockFd)
        }
        if Self.matchInode(lockFd, url: dir) == .matched {
            try? fm.removeItem(at: dir)
            if !Self.syncBestEffort(directory) {
                fputs("exact: share: could not sync inbox after removing \(dir.lastPathComponent)\n", stderr)
            }
        }
        close(lockFd)
        return nil
    }

    /// Retries a deferred retirement, returning `nil` once durable (lock closed) or
    /// the remaining `PendingRetire` with `lockFd` still held.
    func retirePending(_ pending: PendingRetire) -> PendingRetire? {
        switch pending {
        case .live(let dir, let lockFd):
            return retireOrDefer(dir, lockFd: lockFd)
        case .moved(let retired, let lockFd):
            return finishMovedRetire(retired, lockFd: lockFd)
        }
    }

    private func finishMovedRetire(_ retired: URL, lockFd: Int32) -> PendingRetire? {
        let syncedShares = Self.syncBestEffort(directory)
        let syncedStaging = stagingDirectory.map { Self.syncBestEffort($0) } ?? true
        guard syncedShares && syncedStaging else {
            fputs("exact: share: could not sync inbox after retiring \(retired.lastPathComponent)\n", stderr)
            return .moved(retired: retired, lockFd: lockFd)
        }
        try? FileManager.default.removeItem(at: retired)
        if lockFd >= 0 { close(lockFd) }
        return nil
    }

    /// Retires `dir` and always closes `lockFd` (for tests and session teardown).
    @discardableResult
    func retire(_ dir: URL, lockFd: Int32) -> Bool {
        guard let deferred = retireOrDefer(dir, lockFd: lockFd) else { return true }
        if deferred.lockFd >= 0 { close(deferred.lockFd) }
        return false
    }

    @discardableResult
    private static func syncFd(_ fd: Int32) -> Bool {
        guard fd >= 0 else { return false }
        while true {
            if fcntl(fd, F_FULLFSYNC) == 0 { return true }
            if errno == EINTR { continue }
            if errno == ENOTSUP || errno == EINVAL {
                while true {
                    if fsync(fd) == 0 { return true }
                    if errno == EINTR { continue }
                    return false
                }
            }
            return false
        }
    }

    @discardableResult
    private static func syncBestEffort(_ url: URL, isDirectory: Bool = true) -> Bool {
        let flags: Int32 = O_RDONLY | O_NONBLOCK | O_NOFOLLOW | O_CLOEXEC | (isDirectory ? O_DIRECTORY : 0)
        let fd = url.withUnsafeFileSystemRepresentation { path -> Int32 in
            guard let path else { return -1 }
            while true {
                let opened = open(path, flags)
                if opened < 0 && errno == EINTR { continue }
                return opened
            }
        }
        guard fd >= 0 else { return false }
        defer { close(fd) }
        return syncFd(fd)
    }

    /// Removes abandoned share staging directories (`share-*` / `done-*` or containing
    /// `share.json.tmp`) and unpublished `shares/` directories (`share.json.tmp` without
    /// `share.json`) untouched for at least `age` seconds. Skips any directory with a live
    /// lock or unreadable metadata, and never touches unrelated subdirectories in `staging/`.
    func sweepIncomplete(olderThan age: TimeInterval = 3600, now: Date = Date()) {
        guard let staging = stagingDirectory else { return }
        let fm = FileManager.default
        let keys: [URLResourceKey] = [.isDirectoryKey, .contentModificationDateKey, .creationDateKey]
        sweepDirectory(staging, requireTempMarker: false, keys: keys, olderThan: age, now: now, fm: fm)
        sweepDirectory(directory, requireTempMarker: true, keys: keys, olderThan: age, now: now, fm: fm)
    }

    private func sweepDirectory(_ parent: URL, requireTempMarker: Bool, keys: [URLResourceKey], olderThan age: TimeInterval, now: Date, fm: FileManager) {
        guard let dirs = try? fm.contentsOfDirectory(at: parent, includingPropertiesForKeys: keys) else { return }
        for dir in dirs {
            guard (try? dir.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true else { continue }
            guard let fd = Self.acquireLock(dir) else { continue }
            defer { close(fd) }
            var st = stat()
            let hasRecord = fstatat(fd, "share.json", &st, AT_SYMLINK_NOFOLLOW) == 0
            let hasTemp = fstatat(fd, "share.json.tmp", &st, AT_SYMLINK_NOFOLLOW) == 0
            if requireTempMarker {
                guard !hasRecord, hasTemp else { continue }
            } else {
                let name = dir.lastPathComponent
                guard name.hasPrefix("share-") || name.hasPrefix("done-") || hasTemp else { continue }
            }
            guard let latest = Self.latestModification(in: dir, keys: keys), now.timeIntervalSince(latest) >= age else { continue }
            guard Self.matchInode(fd, url: dir) == .matched else { continue }
            try? fm.removeItem(at: dir)
        }
    }

    /// Latest timestamp across `dir` and all immediate children; returns nil if
    /// enumeration fails or any entry's timestamp is unreadable.
    private static func latestModification(in dir: URL, keys: [URLResourceKey]) -> Date? {
        let dateOf: (URL) -> Date? = { url in
            let v = try? url.resourceValues(forKeys: Set(keys))
            return v?.contentModificationDate ?? v?.creationDate
        }
        guard var latest = dateOf(dir) else { return nil }
        guard let children = try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: keys) else { return nil }
        for child in children {
            guard let d = dateOf(child) else { return nil }
            if d > latest { latest = d }
        }
        return latest
    }
}

/// The inbox's Darwin notification, `<app id>.shares`, which the extension
/// posts after writing a share: while the app runs it claims at once.
final class ShareInboxPing {
    static let shared = ShareInboxPing()
    /// The sessions that claim the inbox, in the order they began to.
    private var sessions: [WeakSession] = []
    private var listening = false

    /// The session a ping goes to: the newest not destroyed. One session
    /// claims, so each share is delivered once.
    var recipient: ExactSession? {
        sessions.removeAll { $0.session == nil || $0.session?.state == .destroyed }
        return sessions.last?.session
    }

    func remove(_ session: ExactSession) {
        sessions.removeAll { $0.session == nil || $0.session === session }
    }

    func listen(_ name: String, for session: ExactSession) {
        sessions.removeAll { $0.session == nil || $0.session === session }
        sessions.append(WeakSession(session))
        guard !listening else { return }
        listening = true
        let center = CFNotificationCenterGetDarwinNotifyCenter()
        CFNotificationCenterAddObserver(center, Unmanaged.passUnretained(self).toOpaque(), { _, observer, _, _, _ in
            guard let observer else { return }
            let ping = Unmanaged<ShareInboxPing>.fromOpaque(observer).takeUnretainedValue()
            DispatchQueue.main.async { ping.recipient?.claimInbox() }
        }, name as CFString, nil, .deliverImmediately)
    }
}

/// Per-session ownership of an unlanded share claim (cold boot or warm delivery
/// whose `Navigate` succeeded while attachment landing failed) or a delivered share
/// whose inbox retirement failed transiently, plus deferral, in-flight batch, and
/// bounded retry timer state.
private final class ShareSessionState {
    var pendingLand: (dir: URL, lockFd: Int32)?
    var pendingRetire: ShareInbox.PendingRetire?
    var deferred = false
    var claiming = false
    var waitingForBatch = false
    var retryWork: DispatchWorkItem?
    var retryAttempt = 0

    func cancelRetry() {
        retryWork?.cancel()
        retryWork = nil
    }

    @discardableResult
    func releasePendingLand() -> Bool {
        guard let claim = pendingLand else { return false }
        pendingLand = nil
        if claim.lockFd >= 0 { close(claim.lockFd) }
        return true
    }

    @discardableResult
    func releasePendingRetire() -> Bool {
        guard let pending = pendingRetire else { return false }
        pendingRetire = nil
        if pending.lockFd >= 0 { close(pending.lockFd) }
        return true
    }

    deinit {
        cancelRetry()
        releasePendingLand()
        releasePendingRetire()
    }
}

/// Weak-key table mapping each `ExactSession` to its own share claim state (main-thread only).
nonisolated(unsafe) private let shareSessionStates = NSMapTable<ExactSession, ShareSessionState>.weakToStrongObjects()

extension ExactSession {
    private var shareState: ShareSessionState {
        if let existing = shareSessionStates.object(forKey: self) { return existing }
        let created = ShareSessionState()
        shareSessionStates.setObject(created, forKey: self)
        return created
    }

    /// Test helper: attaches a held lock to this session's `pendingLand` slot.
    func setPendingLandForTests(_ dir: URL, lockFd: Int32) {
        let st = shareState
        st.releasePendingLand()
        st.pendingLand = (dir: dir, lockFd: lockFd)
    }

    /// Releases observers, retry timers, and held inbox locks on session destruction,
    /// waking any surviving session if a held inbox claim became free.
    func destroyShareState() {
        if let observer = shareInboxObserver {
            NotificationCenter.default.removeObserver(observer)
            shareInboxObserver = nil
        }
        ShareInboxPing.shared.remove(self)
        let releasedLock: Bool
        if let st = shareSessionStates.object(forKey: self) {
            st.cancelRetry()
            let releasedLand = st.releasePendingLand()
            let releasedRetire = st.releasePendingRetire()
            releasedLock = releasedLand || releasedRetire
            shareSessionStates.removeObject(forKey: self)
        } else {
            releasedLock = false
        }
        if releasedLock, let survivor = ShareInboxPing.shared.recipient, survivor !== self {
            DispatchQueue.main.async { [weak survivor] in survivor?.claimInbox() }
        }
    }

    /// Schedules a bounded backoff retry for a recoverable share failure without
    /// setting `deferred`, so unrelated UI batch completions do not defeat backoff.
    private func scheduleInboxRetry(_ st: ShareSessionState) {
        guard state != .destroyed, clock == nil, !ExactEnv.agentFreezes else { return }
        guard st.retryWork == nil else { return }
        let delay = min(0.1 * pow(2.0, Double(min(st.retryAttempt, 6))), 5.0)
        st.retryAttempt = min(st.retryAttempt + 1, 6)
        let work = DispatchWorkItem { [weak self] in
            guard let self, self.state != .destroyed else { return }
            let current = self.shareState
            current.retryWork = nil
            self.claimInbox()
        }
        st.retryWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: work)
    }

    /// The agent's `share` op: `{"id": <navigation root>, "target", "share"}`.
    func receiveShare(_ req: [String: Any]) -> [String: Any] {
        guard booted, let root = navigationRoot else { return ["error": "share: the app has no navigation root to deliver it to"] }
        var request = req.filter { ["target", "share"].contains($0.key) }
        request["root"] = Int(root.id)
        let batch = deliverShare(request)
        if let error = batch?.error { return ["error": error] }
        if batch?.held == true || batch?.busy == true { return ["error": "share: runtime is held over a reset"] }
        return batch == nil ? ["error": "share: the share is malformed"] : ["typed": Int(root.id), "delivery": "recognized"]
    }

    private func deliverShare(_ request: [String: Any], onLanded: (() -> Void)? = nil, onDropped: (() -> Void)? = nil) -> Batch? {
        guard let data = try? JSONSerialization.data(withJSONObject: request) else {
            onDropped?()
            return nil
        }
        let batch = runtime.share(String(decoding: data, as: UTF8.self), now: now())
        if !batch.held, !batch.busy {
            apply(batch, barrier: true, onLanded: onLanded, onDropped: onDropped)
        } else {
            onDropped?()
        }
        return batch
    }

    /// Before the boot: the driver's cold share (`EXACT_AGENT_SHARE`), else
    /// the inbox's oldest complete unlocked share, becomes the launch location and
    /// its tokens. Holds an exclusive lock on the claimed directory under this
    /// session until landing finishes so no second window can steal or delete it.
    func claimColdShare() {
        guard !booted else { return }
        if ExactEnv.agentMode, let json = ExactEnv.environment["EXACT_AGENT_SHARE"] {
            _ = runtime.share(json, now: 0)
            return
        }
        guard let inbox = ShareInbox.main else { return }
        let st = shareState
        let scan = inbox.completeChecked()
        guard !scan.hadTransientError else { return }
        for dir in scan.entries {
            let lockFd: Int32
            switch ShareInbox.tryAcquireLock(dir) {
            case .acquired(let fd): lockFd = fd
            case .busy: continue
            case .transientError: return
            }
            let request: [String: Any]
            switch inbox.requestLocked(dir, lockFd: lockFd) {
            case .ok(let req): request = req
            case .corrupt:
                if let deferred = inbox.retireOrDefer(dir, lockFd: lockFd) {
                    st.releasePendingRetire()
                    st.pendingRetire = deferred
                    return
                }
                continue
            case .missing: close(lockFd); continue
            case .transientError: close(lockFd); return
            }
            guard let data = try? JSONSerialization.data(withJSONObject: request) else {
                close(lockFd)
                return
            }
            let batch = runtime.share(String(decoding: data, as: UTF8.self), now: 0)
            if !batch.held, !batch.busy, batch.error == nil {
                st.releasePendingLand()
                st.pendingLand = (dir: dir, lockFd: lockFd)
                return
            }
            if batch.error?.hasPrefix("share: invalid: ") == true {
                if let deferred = inbox.retireOrDefer(dir, lockFd: lockFd) {
                    st.releasePendingRetire()
                    st.pendingRetire = deferred
                    return
                }
                continue
            }
            // Preserve FIFO order on transient claim errors (stat/I/O/contention).
            close(lockFd)
            return
        }
    }

    /// Registers foreground and Darwin-ping observers before scanning the inbox so
    /// shares published during startup cannot be missed, lands this session's cold
    /// share first, and then drains any remaining warm inbox entries.
    func landColdShare() {
        guard state != .destroyed else { return }
        installInboxObserversIfNeeded()
        if tryLandPendingShare() {
            claimInbox()
        }
    }

    private func installInboxObserversIfNeeded() {
        guard ShareInbox.main != nil, shareInboxObserver == nil else { return }
        #if canImport(UIKit)
        let foreground = UIApplication.willEnterForegroundNotification
        #else
        let foreground = NSApplication.didBecomeActiveNotification
        #endif
        shareInboxObserver = NotificationCenter.default.addObserver(forName: foreground, object: nil, queue: .main) { [weak self] _ in
            guard let self, self.state != .destroyed else { return }
            if ShareInboxPing.shared.recipient == nil
                || ShareInboxPing.shared.recipient === self
                || self.shareState.pendingLand != nil
                || self.shareState.pendingRetire != nil {
                self.claimInbox()
            }
        }
        if let id = Bundle.main.object(forInfoDictionaryKey: "ExactAppIdentifier") as? String ?? Bundle.main.bundleIdentifier {
            ShareInboxPing.shared.listen("\(id).shares", for: self)
        }
    }

    /// Attempts to land this session's pending unlanded share (cold claim or warm
    /// share whose `Navigate` already committed). Returns true only when warm
    /// inbox draining may proceed immediately.
    @discardableResult
    private func tryLandPendingShare() -> Bool {
        let st = shareState
        let hasAgentShare = ExactEnv.agentMode && ExactEnv.environment["EXACT_AGENT_SHARE"] != nil
        guard hasAgentShare || st.pendingLand != nil else { return true }
        let batch = runtime.landShare()
        guard let claim = st.pendingLand else {
            if !batch.held, !batch.busy { apply(batch, barrier: true) }
            return !batch.held && !batch.busy
        }
        if batch.held || batch.busy {
            st.deferred = true
            scheduleInboxRetry(st)
            return false
        }
        if batch.error == "share: no cold share claimed" {
            // A fresh-state reboot cleared the bridge's pending share state without
            // deleting the inbox; release the lock and let `claimInbox()` re-claim it.
            st.releasePendingLand()
            return true
        }
        if batch.error != nil {
            // Keep `st.pendingLand` (and its lock) plus `Bridge.cold_share` intact so
            // a later retry lands against the original delivery without re-navigating.
            // Wait for any visual ops in `batch` to reveal before retrying so an empty
            // retry cannot race ahead of an earlier staged attachment batch.
            if !batch.ops.isEmpty {
                st.waitingForBatch = true
                var finished = false
                let done: () -> Void = { [weak self] in
                    guard !finished else { return }
                    finished = true
                    guard let self, self.state != .destroyed, let cur = shareSessionStates.object(forKey: self) else { return }
                    cur.waitingForBatch = false
                    self.scheduleInboxRetry(cur)
                }
                apply(batch, barrier: true, onLanded: done, onDropped: done)
            } else {
                scheduleInboxRetry(st)
                apply(batch, barrier: true)
            }
            return false
        }
        guard let inbox = ShareInbox.main else {
            st.releasePendingLand()
            apply(batch, barrier: true)
            return true
        }
        // Only retire once the landed batch actually applies (`onLanded`); if it is
        // dropped before reveal or during session destruction, release `lockFd` without
        // deleting the inbox directory so a surviving session or next launch recovers it.
        st.pendingLand = nil
        st.waitingForBatch = true
        var settledSync = false
        var finished = false
        var retiredOk = false
        let onLanded: () -> Void = { [weak self] in
            guard !finished else { return }
            finished = true
            guard let self, self.state != .destroyed, let cur = shareSessionStates.object(forKey: self) else {
                inbox.retire(claim.dir, lockFd: claim.lockFd)
                return
            }
            cur.waitingForBatch = false
            if let deferred = inbox.retireOrDefer(claim.dir, lockFd: claim.lockFd) {
                cur.releasePendingRetire()
                cur.pendingRetire = deferred
                self.scheduleInboxRetry(cur)
            } else {
                retiredOk = true
                cur.retryAttempt = 0
                if !settledSync {
                    DispatchQueue.main.async { self.claimInbox() }
                }
            }
        }
        let onDropped: () -> Void = { [weak self] in
            guard !finished else { return }
            finished = true
            if claim.lockFd >= 0 { close(claim.lockFd) }
            guard let self else { return }
            if self.state == .destroyed {
                if let survivor = ShareInboxPing.shared.recipient, survivor !== self {
                    DispatchQueue.main.async { [weak survivor] in survivor?.claimInbox() }
                }
                return
            }
            guard let cur = shareSessionStates.object(forKey: self) else { return }
            cur.waitingForBatch = false
            self.scheduleInboxRetry(cur)
        }
        apply(batch, barrier: true, onLanded: onLanded, onDropped: onDropped)
        settledSync = true
        return finished && retiredOk
    }

    /// Retries a share claim that was deferred by a reset hold, contention, or
    /// pending `needs_apply` change receipt once the session settles.
    func claimDeferredInbox() {
        guard state != .destroyed, let st = shareSessionStates.object(forKey: self) else { return }
        guard st.deferred, !st.claiming, !st.waitingForBatch else { return }
        st.deferred = false
        claimInbox()
    }

    /// Each complete, unlocked share in the inbox, oldest first, delivered at the
    /// live navigation root. File shares navigate immediately (`pending > 0`) per
    /// LLP 1133.024 D3 and land attachments on the follow-up `landShare()` turn;
    /// the inbox stays exclusively locked until final delivery applies.
    func claimInbox() {
        guard booted, state != .destroyed, let inbox = ShareInbox.main else { return }
        let st = shareState
        guard !st.claiming, !st.waitingForBatch else {
            st.deferred = true
            return
        }
        st.cancelRetry()
        st.claiming = true
        defer { st.claiming = false }
        if let pending = st.pendingRetire {
            if let stillPending = inbox.retirePending(pending) {
                st.pendingRetire = stillPending
                scheduleInboxRetry(st)
                return
            }
            st.pendingRetire = nil
            st.retryAttempt = 0
        }
        if st.pendingLand != nil {
            guard tryLandPendingShare() else { return }
        }
        inbox.sweepIncomplete()
        let scan = inbox.completeChecked()
        if scan.hadTransientError {
            scheduleInboxRetry(st)
            return
        }
        for dir in scan.entries {
            guard state != .destroyed else { return }
            guard let root = navigationRoot else {
                scheduleInboxRetry(st)
                return
            }
            let lockFd: Int32
            switch ShareInbox.tryAcquireLock(dir) {
            case .acquired(let fd): lockFd = fd
            case .busy: continue
            case .transientError:
                scheduleInboxRetry(st)
                return
            }
            var request: [String: Any]
            switch inbox.requestLocked(dir, lockFd: lockFd) {
            case .ok(let req): request = req
            case .corrupt:
                if let deferred = inbox.retireOrDefer(dir, lockFd: lockFd) {
                    st.releasePendingRetire()
                    st.pendingRetire = deferred
                    scheduleInboxRetry(st)
                    return
                }
                continue
            case .missing: close(lockFd); continue
            case .transientError:
                close(lockFd)
                scheduleInboxRetry(st)
                return
            }
            let hasFiles = ((request["share"] as? [String: Any])?["files"] as? [Any])?.isEmpty == false
            request["root"] = Int(root.id)
            guard let data = try? JSONSerialization.data(withJSONObject: request) else {
                close(lockFd)
                continue
            }
            let batch = runtime.share(String(decoding: data, as: UTF8.self), now: now())
            if batch.held || batch.busy {
                close(lockFd)
                st.deferred = true
                scheduleInboxRetry(st)
                return
            }
            if batch.error == "share: deferred until pending work is applied" {
                close(lockFd)
                st.waitingForBatch = true
                var finished = false
                apply(batch, barrier: true, onLanded: { [weak self] in
                    guard !finished else { return }
                    finished = true
                    guard let self, self.state != .destroyed, let cur = shareSessionStates.object(forKey: self) else { return }
                    cur.waitingForBatch = false
                    DispatchQueue.main.async { self.claimInbox() }
                }, onDropped: { [weak self] in
                    guard !finished else { return }
                    finished = true
                    guard let self, self.state != .destroyed, let cur = shareSessionStates.object(forKey: self) else { return }
                    cur.waitingForBatch = false
                    self.scheduleInboxRetry(cur)
                })
                return
            }
            if batch.error?.hasPrefix("share: invalid: ") == true {
                if let deferred = inbox.retireOrDefer(dir, lockFd: lockFd) {
                    st.releasePendingRetire()
                    st.pendingRetire = deferred
                    apply(batch, barrier: true)
                    scheduleInboxRetry(st)
                    return
                }
                apply(batch, barrier: true)
                continue
            }
            if batch.error?.hasPrefix("share: land: ") == true {
                // `Navigate` already committed per LLP 1133.024 D3; keep the lock in
                // `pendingLand` so retries call `landShare()` only.
                st.releasePendingLand()
                st.pendingLand = (dir: dir, lockFd: lockFd)
                st.waitingForBatch = true
                var finished = false
                let done: () -> Void = { [weak self] in
                    guard !finished else { return }
                    finished = true
                    guard let self, self.state != .destroyed, let cur = shareSessionStates.object(forKey: self) else { return }
                    cur.waitingForBatch = false
                    self.scheduleInboxRetry(cur)
                }
                apply(batch, barrier: true, onLanded: done, onDropped: done)
                return
            }
            if batch.error != nil {
                close(lockFd)
                if !batch.ops.isEmpty {
                    st.waitingForBatch = true
                    var finished = false
                    let done: () -> Void = { [weak self] in
                        guard !finished else { return }
                        finished = true
                        guard let self, self.state != .destroyed, let cur = shareSessionStates.object(forKey: self) else { return }
                        cur.waitingForBatch = false
                        self.scheduleInboxRetry(cur)
                    }
                    apply(batch, barrier: true, onLanded: done, onDropped: done)
                    return
                }
                apply(batch, barrier: true)
                scheduleInboxRetry(st)
                return
            }
            if hasFiles {
                // LLP 1133.024 D3: the route and text arrived with `pending > 0` without
                // blocking on attachment copies; hand `(dir, lockFd)` to `pendingLand` so
                // attachments copy and land on the next turn once this route batch applies.
                st.releasePendingLand()
                st.pendingLand = (dir: dir, lockFd: lockFd)
                st.waitingForBatch = true
                var finished = false
                apply(batch, barrier: true, onLanded: { [weak self] in
                    guard !finished else { return }
                    finished = true
                    guard let self, self.state != .destroyed, let cur = shareSessionStates.object(forKey: self) else { return }
                    cur.waitingForBatch = false
                    DispatchQueue.main.async { self.claimInbox() }
                }, onDropped: { [weak self] in
                    guard !finished else { return }
                    finished = true
                    guard let self else { return }
                    if self.state == .destroyed {
                        if let survivor = ShareInboxPing.shared.recipient, survivor !== self {
                            DispatchQueue.main.async { [weak survivor] in survivor?.claimInbox() }
                        }
                        return
                    }
                    guard let cur = shareSessionStates.object(forKey: self) else { return }
                    cur.waitingForBatch = false
                    self.scheduleInboxRetry(cur)
                })
                return
            }
            // Text/URL-only share: retire only when the delivery batch applies (`onLanded`);
            // if dropped before reveal or on teardown, release `lockFd` without deleting.
            st.waitingForBatch = true
            var finished = false
            var retiredOk = false
            var inLoop = true
            apply(batch, barrier: true, onLanded: { [weak self] in
                guard !finished else { return }
                finished = true
                guard let self, self.state != .destroyed, let cur = shareSessionStates.object(forKey: self) else {
                    inbox.retire(dir, lockFd: lockFd)
                    return
                }
                cur.waitingForBatch = false
                if let deferred = inbox.retireOrDefer(dir, lockFd: lockFd) {
                    cur.releasePendingRetire()
                    cur.pendingRetire = deferred
                    self.scheduleInboxRetry(cur)
                } else {
                    retiredOk = true
                    cur.retryAttempt = 0
                    if !inLoop {
                        DispatchQueue.main.async { self.claimInbox() }
                    }
                }
            }, onDropped: { [weak self] in
                guard !finished else { return }
                finished = true
                if lockFd >= 0 { close(lockFd) }
                guard let self else { return }
                if self.state == .destroyed {
                    if let survivor = ShareInboxPing.shared.recipient, survivor !== self {
                        DispatchQueue.main.async { [weak survivor] in survivor?.claimInbox() }
                    }
                    return
                }
                guard let cur = shareSessionStates.object(forKey: self) else { return }
                cur.waitingForBatch = false
                self.scheduleInboxRetry(cur)
            })
            inLoop = false
            if !finished || !retiredOk {
                return
            }
        }
    }
}
