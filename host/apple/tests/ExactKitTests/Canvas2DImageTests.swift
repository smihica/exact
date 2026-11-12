import XCTest
import CoreGraphics
import ImageIO
@testable import ExactSurfaces

final class Canvas2DImageTests: XCTestCase {
    private let colors: [[UInt8]] = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 0, 255]]

    private func jpeg(orientation: Int?) throws -> Data {
        var bytes: [UInt8] = []
        for y in 0..<20 {
            for x in 0..<40 { bytes += colors[(y < 10 ? 0 : 2) + (x < 20 ? 0 : 1)] }
        }
        let image = try XCTUnwrap(Canvas2DSpace.srgb8.image(bytes, width: 40, height: 20))
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, "public.jpeg" as CFString, 1, nil))
        var properties: [CFString: Any] = [kCGImageDestinationLossyCompressionQuality: 1]
        if let orientation { properties[kCGImagePropertyOrientation] = orientation }
        CGImageDestinationAddImage(destination, image, properties as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }

    private func draw(_ host: Canvas2DHost, _ image: CGImage) throws -> CGContext {
        var data = Data()
        func u32(_ value: UInt32) { withUnsafeBytes(of: value.littleEndian) { data.append(contentsOf: $0) } }
        u32(0x4432_4345); u32(1)
        let records: [(Canvas2DOp, [Double])] = [(.image, [1, 112]),
            (.drawImage, [1, 0, 0, Double(image.width), Double(image.height), 0, 0, Double(image.width), Double(image.height)])]
        for (op, numbers) in records {
            u32(op.rawValue); u32(UInt32(numbers.count))
            for value in numbers { withUnsafeBytes(of: value.bitPattern.littleEndian) { data.append(contentsOf: $0) } }
        }
        let replay = Canvas2DReplayer(width: 60, height: 60, scale: 1, lifetime: 1, generation: 0)
        replay.env = host
        XCTAssertTrue(replay.apply(data))
        return try XCTUnwrap(replay.context)
    }

    private func pixel(_ context: CGContext, x: Int, y: Int) throws -> [UInt8] {
        let bytes = try XCTUnwrap(context.data).assumingMemoryBound(to: UInt8.self)
        let offset = y * context.bytesPerRow + x * 4
        return [bytes[offset + 2], bytes[offset + 1], bytes[offset], bytes[offset + 3]]
    }

    func testImageHandlesReportAndDrawAllExifOrientationsAtFullSize() throws {
        let quadrants = [[0, 1, 2, 3], [0, 1, 2, 3], [1, 0, 3, 2], [3, 2, 1, 0], [2, 3, 0, 1],
                         [0, 2, 1, 3], [2, 0, 3, 1], [3, 1, 2, 0], [1, 3, 0, 2]]
        for orientation in 0...8 {
            let host = Canvas2DHost(), bytes = try jpeg(orientation: orientation == 0 ? nil : orientation)
            host.assetBytes = { _ in bytes }
            let loaded = expectation(description: "orientation \(orientation) decoded")
            var decoded: CGImage?
            host.onImage = { _, image in decoded = image; loaded.fulfill() }
            host.load(["p"])
            wait(for: [loaded], timeout: 10)
            let image = try XCTUnwrap(decoded)
            let (w, h) = orientation >= 5 ? (20, 40) : (40, 20)
            XCTAssertEqual(image.width, w, "orientation \(orientation)")
            XCTAssertEqual(image.height, h, "orientation \(orientation)")
            let context = try draw(host, image)
            for i in 0..<4 {
                let actual = try pixel(context, x: w * (i % 2 * 2 + 1) / 4, y: h * (i / 2 * 2 + 1) / 4)
                let expected = colors[quadrants[orientation][i]]
                for channel in 0..<4 {
                    XCTAssertEqual(Int(actual[channel]), Int(expected[channel]), accuracy: 8,
                                   "orientation \(orientation), quadrant \(i), channel \(channel)")
                }
            }
            XCTAssertEqual(try pixel(context, x: w + 5, y: 5), [0, 0, 0, 0])
            XCTAssertEqual(try pixel(context, x: 5, y: h + 5), [0, 0, 0, 0])
        }
    }
}
