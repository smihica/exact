import Foundation
import XCTest
@testable import ExactKit
@testable import ExactSharing

// The share extension's ping (LLP 1133.024 D4) goes to one session that
// can claim the inbox: the newest that began to listen and is not
// destroyed, so a session ending leaves the ping with the one before it.
final class ShareInboxPingTests: XCTestCase {
    func testAPingGoesToTheNewestSessionNotDestroyed() {
        let ping = ShareInboxPing()
        // A name nothing posts: this listener outlives the test.
        let name = "test.\(UUID().uuidString).shares"
        let a = ExactApp.shared.makeSession(label: "share-a")
        defer { a.destroy() }
        var b: ExactSession? = ExactApp.shared.makeSession(label: "share-b")
        ping.listen(name, for: a)
        ping.listen(name, for: b!)
        XCTAssertTrue(ping.recipient === b)
        b?.destroy()
        XCTAssertTrue(ping.recipient === a, "a destroyed session's ping goes to the one still live")
        b = nil
        XCTAssertTrue(ping.recipient === a, "a released session's ping goes to the one still live")
        let c = ExactApp.shared.makeSession(label: "share-c")
        defer { c.destroy() }
        ping.listen(name, for: c)
        ping.listen(name, for: a)
        XCTAssertTrue(ping.recipient === a, "listening again makes a session the newest")
        a.destroy()
        XCTAssertTrue(ping.recipient === c)
        c.destroy()
        XCTAssertNil(ping.recipient)
    }

    func testRequestKeepsUnreadableShareAndRemovesCorruptJSON() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("share-inbox-\(UUID().uuidString)", isDirectory: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let inbox = ShareInbox(target: ["action": "/share"], directory: root)

        // Corrupt JSON: bytes read, parse fails -> directory removed so it does not loop forever.
        let corrupt = root.appendingPathComponent("corrupt", isDirectory: true)
        try FileManager.default.createDirectory(at: corrupt, withIntermediateDirectories: true)
        try Data("{not json".utf8).write(to: corrupt.appendingPathComponent("share.json"))
        XCTAssertNil(inbox.request(corrupt))
        XCTAssertFalse(FileManager.default.fileExists(atPath: corrupt.path))

        // Unreadable share.json (transient I/O / protection error): directory stays intact.
        let locked = root.appendingPathComponent("locked", isDirectory: true)
        try FileManager.default.createDirectory(at: locked, withIntermediateDirectories: true)
        let lockedRecord = locked.appendingPathComponent("share.json")
        try Data("{\"v\":1}".utf8).write(to: lockedRecord)
        try FileManager.default.setAttributes([.posixPermissions: 0o000], ofItemAtPath: lockedRecord.path)
        defer { try? FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: lockedRecord.path) }
        XCTAssertNil(inbox.request(locked))
        XCTAssertTrue(FileManager.default.fileExists(atPath: locked.path))
    }

    func testSweepIncompleteRemovesOnlyUnlockedStaleStagingDirectories() throws {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("share-sweep-\(UUID().uuidString)", isDirectory: true)
        defer { try? fm.removeItem(at: root) }
        let shares = root.appendingPathComponent("shares", isDirectory: true)
        let staging = root.appendingPathComponent("staging", isDirectory: true)
        let inbox = ShareInbox(target: ["action": "/share"], directory: shares, stagingDirectory: staging)
        let now = Date()
        let old = now.addingTimeInterval(-7200)
        let recent = now.addingTimeInterval(-60)

        let legacyInShares = shares.appendingPathComponent("legacy-incomplete", isDirectory: true)
        let failedPostRenameInShares = shares.appendingPathComponent("failed-post-rename", isDirectory: true)
        let oldComplete = shares.appendingPathComponent("old-complete", isDirectory: true)
        let lockedInShares = shares.appendingPathComponent("locked-publishing", isDirectory: true)
        let staleStaging = staging.appendingPathComponent("share-stale-staging", isDirectory: true)
        let unrelatedStaging = staging.appendingPathComponent("unrelated-staging", isDirectory: true)
        let freshStaging = staging.appendingPathComponent("share-fresh-staging", isDirectory: true)
        let activeStaging = staging.appendingPathComponent("share-active-staging", isDirectory: true)
        let lockedStaging = staging.appendingPathComponent("share-locked-staging", isDirectory: true)
        let unreadableStaging = staging.appendingPathComponent("share-unreadable-staging", isDirectory: true)
        for dir in [legacyInShares, failedPostRenameInShares, oldComplete, lockedInShares, staleStaging, unrelatedStaging, freshStaging, activeStaging, lockedStaging, unreadableStaging] {
            try fm.createDirectory(at: dir, withIntermediateDirectories: true)
        }
        try Data("part".utf8).write(to: legacyInShares.appendingPathComponent("1.txt"))
        try Data("{\"v\":1}".utf8).write(to: failedPostRenameInShares.appendingPathComponent("share.json.tmp"))
        try Data("{\"v\":1}".utf8).write(to: oldComplete.appendingPathComponent("share.json"))
        try Data("{\"v\":1}".utf8).write(to: lockedInShares.appendingPathComponent("share.json"))
        try Data("part".utf8).write(to: staleStaging.appendingPathComponent("1.txt"))
        try Data("keep".utf8).write(to: unrelatedStaging.appendingPathComponent("note.txt"))
        try Data("part".utf8).write(to: freshStaging.appendingPathComponent("1.txt"))
        let activeFile = activeStaging.appendingPathComponent("1.txt")
        try Data("copying".utf8).write(to: activeFile)
        try Data("part".utf8).write(to: lockedStaging.appendingPathComponent("1.txt"))
        try Data("part".utf8).write(to: unreadableStaging.appendingPathComponent("1.txt"))

        try fm.setAttributes([.modificationDate: old], ofItemAtPath: legacyInShares.appendingPathComponent("1.txt").path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: legacyInShares.path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: failedPostRenameInShares.appendingPathComponent("share.json.tmp").path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: failedPostRenameInShares.path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: oldComplete.appendingPathComponent("share.json").path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: oldComplete.path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: staleStaging.appendingPathComponent("1.txt").path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: staleStaging.path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: unrelatedStaging.appendingPathComponent("note.txt").path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: unrelatedStaging.path)
        try fm.setAttributes([.modificationDate: recent], ofItemAtPath: freshStaging.appendingPathComponent("1.txt").path)
        try fm.setAttributes([.modificationDate: recent], ofItemAtPath: freshStaging.path)
        try fm.setAttributes([.modificationDate: recent], ofItemAtPath: activeFile.path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: activeStaging.path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: lockedStaging.appendingPathComponent("1.txt").path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: lockedStaging.path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: unreadableStaging.appendingPathComponent("1.txt").path)
        try fm.setAttributes([.modificationDate: old], ofItemAtPath: unreadableStaging.path)
        try fm.setAttributes([.posixPermissions: 0o000], ofItemAtPath: unreadableStaging.path)
        defer { try? fm.setAttributes([.posixPermissions: 0o700], ofItemAtPath: unreadableStaging.path) }

        let lockStagingFd = lockedStaging.withUnsafeFileSystemRepresentation { open($0!, O_RDONLY) }
        XCTAssertGreaterThanOrEqual(lockStagingFd, 0)
        XCTAssertEqual(flock(lockStagingFd, LOCK_EX | LOCK_NB), 0)
        defer { close(lockStagingFd) }

        let lockSharesFd = lockedInShares.withUnsafeFileSystemRepresentation { open($0!, O_RDONLY) }
        XCTAssertGreaterThanOrEqual(lockSharesFd, 0)
        XCTAssertEqual(flock(lockSharesFd, LOCK_EX | LOCK_NB), 0)
        defer { close(lockSharesFd) }

        // A directory still locked by ShareInboxWriter during final parent sync is skipped by complete()/request().
        XCTAssertEqual(inbox.complete().map(\.lastPathComponent), ["old-complete"])
        XCTAssertNil(inbox.request(lockedInShares))

        // Agent override (stagingDirectory == nil) sweeps nothing at all.
        ShareInbox(target: ["action": "/share"], directory: shares, stagingDirectory: nil).sweepIncomplete(olderThan: 3600, now: now)
        XCTAssertTrue(fm.fileExists(atPath: staleStaging.path))

        inbox.sweepIncomplete(olderThan: 3600, now: now)

        XCTAssertTrue(fm.fileExists(atPath: legacyInShares.path), "legacy shares/ dirs without share.json.tmp are never swept")
        XCTAssertFalse(fm.fileExists(atPath: failedPostRenameInShares.path), "stale unpublished share.json.tmp dirs are swept")
        XCTAssertTrue(fm.fileExists(atPath: oldComplete.path))
        XCTAssertFalse(fm.fileExists(atPath: staleStaging.path))
        XCTAssertTrue(fm.fileExists(atPath: unrelatedStaging.path), "unrelated staging subdirs without share-/done- prefix are never swept")
        XCTAssertTrue(fm.fileExists(atPath: freshStaging.path))
        XCTAssertTrue(fm.fileExists(atPath: activeStaging.path))
        XCTAssertTrue(fm.fileExists(atPath: lockedStaging.path))
        XCTAssertTrue(fm.fileExists(atPath: unreadableStaging.path))

        // Atomic retire moves a locked share out of `shares/` and cleans it up.
        let lockFd = try XCTUnwrap(ShareInbox.acquireLock(oldComplete))
        XCTAssertNil(ShareInbox.acquireLock(oldComplete), "a second consumer cannot lock an already-claimed share")
        XCTAssertTrue(inbox.retire(oldComplete, lockFd: lockFd))
        XCTAssertFalse(fm.fileExists(atPath: oldComplete.path))
    }

    func testSessionDestroyReleasesPendingLandLockAndRemovesState() throws {
        ExactSharing.install()
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("share-destroy-\(UUID().uuidString)", isDirectory: true)
        defer { try? fm.removeItem(at: root) }
        let shareDir = root.appendingPathComponent("entry", isDirectory: true)
        try fm.createDirectory(at: shareDir, withIntermediateDirectories: true)
        try Data("{\"v\":1}".utf8).write(to: shareDir.appendingPathComponent("share.json"))

        let session = ExactApp.shared.makeSession(label: "share-destroy-holder")
        let lockFd = try XCTUnwrap(ShareInbox.acquireLock(shareDir))
        session.setPendingLandForTests(shareDir, lockFd: lockFd)
        XCTAssertNil(ShareInbox.acquireLock(shareDir), "lock must be held before destroy")

        // Keep strong reference to `session` after destroy() to prove destroy() releases
        // the lock immediately rather than waiting for deinit.
        session.destroy()
        let reacquired = try XCTUnwrap(ShareInbox.acquireLock(shareDir), "destroy() must release held inbox lock")
        close(reacquired)
        _ = session
    }

    func testApplyCallbacksFireOnImmediateQueuedAndDroppedBatches() {
        let session = ExactApp.shared.makeSession(label: "share-apply-callbacks")
        defer { session.destroy() }

        // Held/busy batches report onDropped immediately without calling onLanded.
        var landed = 0, dropped = 0
        var held = Batch(ops: [], timers: false, motion: false, clock: nil, error: nil)
        held.held = true
        session.apply(held, onLanded: { landed += 1 }, onDropped: { dropped += 1 })
        XCTAssertEqual(landed, 0)
        XCTAssertEqual(dropped, 1)

        let busy = Batch(ops: [], timers: false, motion: false, clock: nil, error: nil, busy: true)
        session.apply(busy, onLanded: { landed += 1 }, onDropped: { dropped += 1 })
        XCTAssertEqual(landed, 0)
        XCTAssertEqual(dropped, 2)

        // An immediate empty batch applies synchronously and invokes onLanded once.
        session.apply(Batch(ops: [], timers: false, motion: false, clock: nil, error: nil), onLanded: { landed += 1 }, onDropped: { dropped += 1 })
        XCTAssertEqual(landed, 1)
        XCTAssertEqual(dropped, 2)

        // A queued batch behind a pending entry invokes onDropped when cancelStage() runs.
        var blocked = Batch(ops: [], timers: false, motion: false, clock: nil, error: nil)
        blocked.sequence = 100
        var queued = Batch(ops: [], timers: false, motion: false, clock: nil, error: nil)
        queued.sequence = 101
        session.wait(PendingBatch(batch: blocked, landed: { landed += 10 }, dropped: { dropped += 10 }))
        session.wait(PendingBatch(batch: queued, landed: { landed += 100 }, dropped: { dropped += 100 }))
        session.cancelStage()
        XCTAssertEqual(landed, 1)
        XCTAssertEqual(dropped, 112)
    }

    func testRequestRejectsSymlinkAndOversizedShareRecord() throws {
        let fm = FileManager.default
        let root = fm.temporaryDirectory.appendingPathComponent("share-bounds-\(UUID().uuidString)", isDirectory: true)
        defer { try? fm.removeItem(at: root) }
        let inbox = ShareInbox(target: ["action": "/share"], directory: root)

        let targetFile = root.appendingPathComponent("outside.json")
        try fm.createDirectory(at: root, withIntermediateDirectories: true)
        try Data("{\"v\":1}".utf8).write(to: targetFile)

        let symDir = root.appendingPathComponent("symlink-record", isDirectory: true)
        try fm.createDirectory(at: symDir, withIntermediateDirectories: true)
        try fm.createSymbolicLink(at: symDir.appendingPathComponent("share.json"), withDestinationURL: targetFile)
        XCTAssertNil(inbox.request(symDir))
        XCTAssertFalse(fm.fileExists(atPath: symDir.path), "symlink share.json should be retired")
        XCTAssertTrue(fm.fileExists(atPath: targetFile.path), "symlink target outside share dir must stay intact")

        let bigDir = root.appendingPathComponent("oversized-record", isDirectory: true)
        try fm.createDirectory(at: bigDir, withIntermediateDirectories: true)
        try Data(repeating: 0x20, count: ShareInbox.recordCap + 1).write(to: bigDir.appendingPathComponent("share.json"))
        XCTAssertNil(inbox.request(bigDir))
        XCTAssertFalse(fm.fileExists(atPath: bigDir.path), "oversized share.json should be retired")
    }
}
