// Compose the repository banner from the same approved full-tile artwork as the app.
import AppKit
import Foundation

func renderBanner(mark: URL, destination: URL) throws {
    guard let artwork = NSImage(contentsOf: mark),
          let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 2560, pixelsHigh: 600,
                                        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                                        isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
          let graphics = NSGraphicsContext(bitmapImageRep: bitmap) else {
        throw NSError(domain: "WriterCraftBanner", code: 1,
                      userInfo: [NSLocalizedDescriptionKey: "Cannot load the brand tile or allocate the banner."])
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = graphics
    defer { NSGraphicsContext.restoreGraphicsState() }
    graphics.cgContext.scaleBy(x: 2, y: 2)
    let bounds = NSRect(x: 0, y: 0, width: 1280, height: 300)
    let panel = NSBezierPath(roundedRect: bounds, xRadius: 18, yRadius: 18)
    panel.addClip()
    let top = NSColor(srgbRed: 32/255, green: 40/255, blue: 52/255, alpha: 1)
    let bottom = NSColor(srgbRed: 20/255, green: 25/255, blue: 31/255, alpha: 1)
    NSGradient(starting: bottom, ending: top)?.draw(in: bounds, angle: 70)
    let rows: [(String, CGFloat, [NSAttributedString.Key: Any])] = [
        ("WriterCraft", 153, [.font: NSFont.systemFont(ofSize: 58, weight: .bold),
                              .foregroundColor: NSColor(srgbRed: 240/255, green: 243/255, blue: 248/255, alpha: 1)]),
        ("Your stories. Your files. Your workspace.", 103,
         [.font: NSFont.systemFont(ofSize: 24),
          .foregroundColor: NSColor(srgbRed: 181/255, green: 192/255, blue: 207/255, alpha: 1)]),
        ("NATIVE RUST · LOCAL FIRST · OPEN SOURCE", 62,
         [.font: NSFont.systemFont(ofSize: 15, weight: .medium), .kern: 2,
          .foregroundColor: NSColor(srgbRed: 119/255, green: 170/255, blue: 1, alpha: 1)])
    ]
    // Center the complete identity + copy group by measured typography, not the image tag.
    let textWidth = rows.map { ($0.0 as NSString).size(withAttributes: $0.2).width }.max() ?? 0
    let tile: CGFloat = 172
    let gap: CGFloat = 32
    let left = (1280 - tile - gap - textWidth) / 2
    artwork.draw(in: NSRect(x: left, y: 64, width: tile, height: tile),
                 from: .zero, operation: .sourceOver, fraction: 1)
    for (text, baseline, attributes) in rows {
        (text as NSString).draw(at: NSPoint(x: left + tile + gap, y: baseline), withAttributes: attributes)
    }
    guard let data = bitmap.representation(using: .png, properties: [:]) else {
        throw NSError(domain: "WriterCraftBanner", code: 2,
                      userInfo: [NSLocalizedDescriptionKey: "Cannot encode the banner."])
    }
    try data.write(to: destination, options: .atomic)
}

do {
    guard CommandLine.arguments.count == 3 else {
        throw NSError(domain: "WriterCraftBanner", code: 3,
                      userInfo: [NSLocalizedDescriptionKey: "Usage: MakeBanner.swift BRAND_PNG OUTPUT_PNG"])
    }
    try renderBanner(mark: URL(fileURLWithPath: CommandLine.arguments[1]),
                     destination: URL(fileURLWithPath: CommandLine.arguments[2]))
} catch {
    fputs("WriterCraft banner export failed: \(error.localizedDescription)\n", stderr)
    exit(EXIT_FAILURE)
}
