import AppKit
import ImageIO
import UniformTypeIdentifiers
import XCTest
@testable import Threadbear

/// Choosing a picture copies it into the looks folder, scaled down: a photo
/// as a JPEG, one with see-through parts as a PNG. Wayne's photos are HEIC
/// from his iPhone as often as JPEG.
@MainActor
final class LooksImportTests: XCTestCase {
    private func sample(_ ext: String) throws -> URL {
        let source = URL(fileURLWithPath: "/System/Library/Desktop Pictures/Mac Purple.heic")
        try XCTSkipUnless(FileManager.default.fileExists(atPath: source.path), "no sample photo on this Mac")
        if ext == "heic" { return source }
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("looks-sample-\(UUID().uuidString).\(ext)")
        let image = try XCTUnwrap(NSImage(contentsOf: source))
        let rep = try XCTUnwrap(image.tiffRepresentation.flatMap(NSBitmapImageRep.init(data:)))
        try XCTUnwrap(rep.representation(using: ext == "png" ? .png : .jpeg, properties: [:])).write(to: out)
        return out
    }

    func testPhotosAreCopiedInScaledDown() throws {
        for ext in ["heic", "jpg"] {
            let name = try XCTUnwrap(Looks.importPicture(from: try sample(ext), maxSide: 1200, prefix: "test"), ext)
            defer { Looks.remove(name) }
            XCTAssertTrue(name.hasPrefix("test-"))
            let copy = try XCTUnwrap(Looks.image(name), ext)
            let rep = try XCTUnwrap(copy.representations.first)
            XCTAssertLessThanOrEqual(max(rep.pixelsWide, rep.pixelsHigh), 1200, ext)
        }
    }

    /// Two frames, written the way any GIF maker does.
    private func animatedGIF() throws -> URL {
        let out = FileManager.default.temporaryDirectory.appendingPathComponent("looks-sample-\(UUID().uuidString).gif")
        let gif = try XCTUnwrap(CGImageDestinationCreateWithURL(out as CFURL, UTType.gif.identifier as CFString, 2, nil))
        for shade in [0.2, 0.8] {
            let context = try XCTUnwrap(CGContext(data: nil, width: 40, height: 20, bitsPerComponent: 8, bytesPerRow: 0,
                                                  space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
            context.setFillColor(gray: shade, alpha: 1)
            context.fill(CGRect(x: 0, y: 0, width: 40, height: 20))
            let frame = [kCGImagePropertyGIFDictionary: [kCGImagePropertyGIFDelayTime: 0.1]] as CFDictionary
            CGImageDestinationAddImage(gif, try XCTUnwrap(context.makeImage()), frame)
        }
        XCTAssertTrue(CGImageDestinationFinalize(gif))
        return out
    }

    func testAnAnimatedGIFBackgroundStaysAGIF() throws {
        let source = try animatedGIF()
        defer { try? FileManager.default.removeItem(at: source) }
        let name = try XCTUnwrap(Looks.importPicture(from: source, maxSide: 2880, prefix: "test", animated: true))
        defer { Looks.remove(name) }
        XCTAssertTrue(name.hasSuffix(".gif"))
        XCTAssertTrue(Looks.isAnimated(try XCTUnwrap(Looks.image(name))))
    }

    /// A page's own picture (the bear's place) is drawn still, so a GIF
    /// chosen there is kept as its first frame, scaled like any other.
    func testAGIFForAPagesPictureBecomesAStill() throws {
        let source = try animatedGIF()
        defer { try? FileManager.default.removeItem(at: source) }
        let name = try XCTUnwrap(Looks.importPicture(from: source, maxSide: 1200, prefix: "test"))
        defer { Looks.remove(name) }
        XCTAssertFalse(name.hasSuffix(".gif"))
        XCTAssertFalse(Looks.isAnimated(try XCTUnwrap(Looks.image(name))))
    }

    func testSomethingThatIsntAPictureIsRefused() throws {
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("not-a-picture-\(UUID().uuidString).jpg")
        try Data("hello".utf8).write(to: file)
        XCTAssertNil(Looks.importPicture(from: file, maxSide: 1200, prefix: "test"))
    }
}
