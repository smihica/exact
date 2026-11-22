/// A refusal whose description is the whole of what a caller reports: a
/// session that would not boot or answer, an unavailable App Group, an arm
/// that would not load.
public struct ExactFailure: Error, CustomStringConvertible {
    public let description: String
    public init(description: String) { self.description = description }
}
