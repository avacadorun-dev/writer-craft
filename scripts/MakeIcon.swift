// Native export of the approved raster artwork. No tracing, recoloring or replacement drawing.
import AppKit
import CryptoKit
import Darwin
import Foundation

private let sourceSHA256 = "43cd82c1e450674a2dca93a1c68c8f89002c361d0112aea44f5a073b98fa3329"
private let cornerFraction = CGFloat(250) / 1254
private let safeAreaFraction: CGFloat = 0.875

private func failure(_ message: String) -> NSError {
    NSError(domain: "WriterCraftIcon", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
}

private func color(_ hex: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255,
            green: CGFloat((hex >> 8) & 255) / 255,
            blue: CGFloat(hex & 255) / 255, alpha: 1)
}

private func bitmap(_ pixels: Int) throws -> CGContext {
    guard pixels > 0, pixels <= 4096,
          let space = CGColorSpace(name: CGColorSpace.sRGB),
          let context = CGContext(data: nil, width: pixels, height: pixels,
                                  bitsPerComponent: 8, bytesPerRow: pixels * 4, space: space,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue |
                                      CGBitmapInfo.byteOrder32Big.rawValue) else {
        throw failure("Cannot allocate a bounded sRGB icon bitmap.")
    }
    context.interpolationQuality = .high
    context.setShouldAntialias(true)
    return context
}

private func encode(_ image: CGImage, to path: URL) throws {
    let representation = NSBitmapImageRep(cgImage: image)
    guard let png = representation.representation(using: .png, properties: [:]) else {
        throw failure("Cannot encode a native PNG.")
    }
    try png.write(to: path, options: .atomic)
}

private func roundedContains(_ x: CGFloat, _ y: CGFloat, rect: CGRect, radius: CGFloat) -> Bool {
    guard rect.contains(CGPoint(x: x, y: y)) else { return false }
    let centerX = min(max(x, rect.minX + radius), rect.maxX - radius)
    let centerY = min(max(y, rect.minY + radius), rect.maxY - radius)
    return hypot(x - centerX, y - centerY) < radius
}

private func render(_ source: CGImage, pixels: Int, safeArea: Bool) throws -> (CGImage, [String: Any]) {
    let context = try bitmap(pixels)
    let side = CGFloat(pixels) * (safeArea ? safeAreaFraction : 1)
    let inset = (CGFloat(pixels) - side) / 2
    let tile = CGRect(x: inset, y: inset, width: side, height: side)
    let radius = side * cornerFraction
    context.addPath(CGPath(roundedRect: tile, cornerWidth: radius, cornerHeight: radius, transform: nil))
    context.clip()
    context.draw(source, in: tile)
    guard let image = context.makeImage(), let raw = context.data else {
        throw failure("Cannot finish the native icon bitmap.")
    }
    let bytes = raw.bindMemory(to: UInt8.self, capacity: pixels * pixels * 4)
    var bounds = [pixels, pixels, -1, -1]
    var opaqueInterior = 0
    for y in 0..<pixels {
        for x in 0..<pixels {
            let alpha = bytes[(y * pixels + x) * 4 + 3]
            if alpha > 0 {
                bounds[0] = min(bounds[0], x)
                bounds[1] = min(bounds[1], y)
                bounds[2] = max(bounds[2], x)
                bounds[3] = max(bounds[3], y)
            }
            // Exclude only the one-pixel antialias boundary of the approved rounded tile.
            if roundedContains(CGFloat(x) + 0.5, CGFloat(y) + 0.5,
                               rect: tile.insetBy(dx: 1, dy: 1), radius: max(0, radius - 1)) {
                guard alpha == 255 else { throw failure("The rounded icon has a nonopaque interior.") }
                opaqueInterior += 1
            }
        }
    }
    let expectedMin = Int(inset)
    let expectedMax = pixels - expectedMin - 1
    guard bounds == [expectedMin, expectedMin, expectedMax, expectedMax], opaqueInterior > 0 else {
        throw failure("The icon's visible bounds do not match its full-tile/safe-area contract.")
    }
    return (image, ["pixels": pixels, "visible_bounds_inclusive": bounds,
                    "opaque_interior_pixels": opaqueInterior, "safe_area": safeArea])
}

private func comparison(_ source: CGImage, destination: URL) throws {
    let width = 1200
    let height = 740
    guard let space = CGColorSpace(name: CGColorSpace.sRGB),
          let context = CGContext(data: nil, width: width, height: height,
                                  bitsPerComponent: 8, bytesPerRow: width * 4, space: space,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue |
                                      CGBitmapInfo.byteOrder32Big.rawValue) else {
        throw failure("Cannot allocate the comparison bitmap.")
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(cgContext: context, flipped: false)
    defer { NSGraphicsContext.restoreGraphicsState() }
    context.translateBy(x: 0, y: CGFloat(height))
    context.scaleBy(x: 1, y: -1)
    func label(_ text: String, x: CGFloat, y: CGFloat, size: CGFloat, hex: UInt32, bold: Bool = false) {
        // AppKit text is drawn upright inside the comparison's top-down layout.
        context.saveGState()
        context.translateBy(x: x, y: y + size * 1.4)
        context.scaleBy(x: 1, y: -1)
        (text as NSString).draw(at: .zero, withAttributes: [
            .font: bold ? NSFont.boldSystemFont(ofSize: size) : NSFont.systemFont(ofSize: size),
            .foregroundColor: color(hex),
        ])
        context.restoreGState()
    }
    color(0xf0f2f6).setFill()
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))
    label("WriterCraft — approved lion", x: 36, y: 22, size: 26, hex: 0x252c38, bold: true)
    label("Original artwork and blue texture preserved. Native AppKit exports at actual pixel sizes.",
          x: 36, y: 66, size: 16, hex: 0x5d697a)
    for (row, theme) in [("Light", (UInt32(0xf7f8fa), UInt32(0x252c38))),
                          ("Dark", (UInt32(0x21252b), UInt32(0xe8ecf3)))].enumerated() {
        let top = CGFloat(122 + row * 258)
        color(theme.1.0).setFill()
        context.fill(CGRect(x: 20, y: top, width: 1160, height: 242))
        label(theme.0 + " theme panel", x: 38, y: top + 14, size: 16, hex: theme.1.1, bold: true)
        for (column, size) in [32, 64, 128, 32, 64, 128].enumerated() {
            let safeArea = column >= 3
            let x = CGFloat(62 + column * 190)
            let y = top + 68 + CGFloat(128 - size) / 2
            let (image, _) = try render(source, pixels: size, safeArea: safeArea)
            context.saveGState()
            context.translateBy(x: x, y: y + CGFloat(size))
            context.scaleBy(x: 1, y: -1)
            context.draw(image, in: CGRect(x: 0, y: 0, width: size, height: size))
            context.restoreGState()
            label("\(safeArea ? "OS icon" : "Full tile") · \(size) px", x: x - 5,
                  y: top + 211, size: 13, hex: theme.1.1)
        }
    }
    label("Full tile: no outer padding. OS icon: 87.5% tile, transparent safe area, no backing or shadow.",
          x: 36, y: 665, size: 15, hex: 0x5d697a)
    label("Asset comparison only — not an application screenshot.", x: 36, y: 696, size: 13, hex: 0x5d697a)
    guard let image = context.makeImage() else { throw failure("Cannot finish the comparison.") }
    try encode(image, to: destination)
}

private func main() throws {
    guard CommandLine.arguments.count == 3 else {
        throw failure("Usage: MakeIcon.swift APPROVED_SOURCE_PNG OUTPUT_FOLDER")
    }
    let sourceURL = URL(fileURLWithPath: CommandLine.arguments[1])
    let destination = URL(fileURLWithPath: CommandLine.arguments[2], isDirectory: true)
    let data = try Data(contentsOf: sourceURL)
    let sourceHash = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    guard sourceHash == sourceSHA256,
          let representation = NSBitmapImageRep(data: data),
          representation.pixelsWide == 1254, representation.pixelsHigh == 1254,
          !representation.hasAlpha, let source = representation.cgImage else {
        throw failure("Use the unchanged approved opaque 1254×1254 lion PNG with its recorded SHA-256.")
    }
    let iconset = destination.appendingPathComponent("WriterCraft.iconset", isDirectory: true)
    try FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)
    var report: [[String: Any]] = []
    for (size, scale) in [(16, 1), (16, 2), (32, 1), (32, 2), (128, 1), (128, 2),
                          (256, 1), (256, 2), (512, 1), (512, 2)] {
        let (image, evidence) = try render(source, pixels: size * scale, safeArea: true)
        let suffix = scale == 2 ? "@2x" : ""
        try encode(image, to: iconset.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"))
        report.append(evidence)
    }
    var toolbarRenditions: [[String: Any]] = []
    for size in [32, 64, 128] {
        let (image, evidence) = try render(source, pixels: size, safeArea: false)
        try encode(image, to: destination.appendingPathComponent("writercraft-mark-\(size).png"))
        toolbarRenditions.append(evidence)
    }
    let (toolbar, toolbarEvidence) = try render(source, pixels: 512, safeArea: false)
    try encode(toolbar, to: destination.appendingPathComponent("writercraft-mark.png"))
    let (application, _) = try render(source, pixels: 512, safeArea: true)
    try encode(application, to: destination.appendingPathComponent("WriterCraft.png"))
    try comparison(source, destination: destination.appendingPathComponent("comparison.png"))
    let receipt: [String: Any] = [
        "source_sha256": sourceHash, "source_pixels": 1254,
        "source_unchanged": true, "recoloring": false,
        "rounded_corner_fraction": Double(cornerFraction),
        "os_safe_area_fraction": Double(safeAreaFraction),
        "toolbar": toolbarEvidence, "toolbar_renditions": toolbarRenditions,
        "os_renditions": report,
        "color_space": "sRGB", "renderer": "native AppKit/CoreGraphics",
    ]
    let json = try JSONSerialization.data(withJSONObject: receipt, options: [.prettyPrinted, .sortedKeys])
    try json.write(to: destination.appendingPathComponent("icon-generation.json"), options: .atomic)
}

do {
    try main()
} catch {
    fputs("WriterCraft icon export failed: \(error.localizedDescription)\n", stderr)
    exit(EXIT_FAILURE)
}
