import Foundation

@main
struct BoundaryCheck {
    static func main() {
        var failures = 0
        func check(_ name: String, _ passed: Bool) {
            if !passed { failures += 1 }
            print("\(passed ? "PASS" : "FAIL") boundary \(name)")
        }
        let before = Date()
        let first = pedometerBoundary(start: nil)
        let after = Date()
        check("an omitted start begins at the first ask",
              first >= before.addingTimeInterval(-0.001) && first <= after.addingTimeInterval(0.001))

        let recent = before.addingTimeInterval(-3600).timeIntervalSince1970 * 1000
        check("a recent epoch start is preserved", abs(pedometerBoundary(start: recent).timeIntervalSince1970 * 1000 - recent) < 0.001)

        let old = before.addingTimeInterval(-30 * 86400).timeIntervalSince1970 * 1000
        let clamped = pedometerBoundary(start: old)
        check("an old start is clamped to seven days before the ask",
              clamped >= before.addingTimeInterval(-7 * 86400 - 0.001) && clamped <= Date().addingTimeInterval(-7 * 86400 + 0.001))
        exit(failures == 0 ? 0 : 1)
    }
}
