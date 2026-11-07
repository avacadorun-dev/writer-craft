import AppKit
import Foundation

guard CommandLine.arguments.count == 2 else {
    throw NSError(domain: "WriterCraftIcon", code: 1, userInfo: [NSLocalizedDescriptionKey: "Supply one destination iconset folder."])
}
let destination = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)

func rgb(_ red: CGFloat, _ green: CGFloat, _ blue: CGFloat, _ alpha: CGFloat = 1) -> NSColor {
    NSColor(calibratedRed: red / 255, green: green / 255, blue: blue / 255, alpha: alpha)
}

func path(_ points: [(CGFloat, CGFloat)]) -> NSBezierPath {
    let result = NSBezierPath()
    if let first = points.first {
        result.move(to: NSPoint(x: first.0, y: first.1))
        for point in points.dropFirst() {
            result.line(to: NSPoint(x: point.0, y: point.1))
        }
        result.close()
    }
    result.lineJoinStyle = .round
    result.lineCapStyle = .round
    return result
}

func drawMark() {
    let blueInk = rgb(31, 55, 86)
    let paper = rgb(255, 248, 232)
    let crest = NSBezierPath()
    crest.move(to: NSPoint(x: 20, y: 1.5))
    crest.curve(to: NSPoint(x: 35.5, y: 16), controlPoint1: NSPoint(x: 29, y: 1.5), controlPoint2: NSPoint(x: 35.5, y: 7))
    crest.line(to: NSPoint(x: 35.5, y: 26.2))
    crest.curve(to: NSPoint(x: 20, y: 38.1), controlPoint1: NSPoint(x: 35.5, y: 31.4), controlPoint2: NSPoint(x: 28.5, y: 35.2))
    crest.curve(to: NSPoint(x: 4.5, y: 26.2), controlPoint1: NSPoint(x: 11.5, y: 35.2), controlPoint2: NSPoint(x: 4.5, y: 31.4))
    crest.line(to: NSPoint(x: 4.5, y: 16))
    crest.curve(to: NSPoint(x: 20, y: 1.5), controlPoint1: NSPoint(x: 4.5, y: 7), controlPoint2: NSPoint(x: 11, y: 1.5))
    crest.close()
    rgb(225, 234, 245).setFill()
    crest.fill()
    rgb(107, 139, 180, 0.8).setStroke()
    crest.lineWidth = 0.5
    crest.stroke()

    let hood = NSBezierPath()
    hood.move(to: NSPoint(x: 10, y: 30))
    hood.curve(to: NSPoint(x: 15, y: 9.5), controlPoint1: NSPoint(x: 11.2, y: 22), controlPoint2: NSPoint(x: 12, y: 13))
    hood.curve(to: NSPoint(x: 20, y: 6), controlPoint1: NSPoint(x: 16.8, y: 7.2), controlPoint2: NSPoint(x: 18.4, y: 6))
    hood.curve(to: NSPoint(x: 25, y: 9.5), controlPoint1: NSPoint(x: 21.6, y: 6), controlPoint2: NSPoint(x: 23.2, y: 7.2))
    hood.curve(to: NSPoint(x: 30, y: 30), controlPoint1: NSPoint(x: 28, y: 13), controlPoint2: NSPoint(x: 28.8, y: 22))
    hood.close()
    if let robeGradient = NSGradient(starting: rgb(91, 137, 193), ending: rgb(45, 86, 140)) {
        robeGradient.draw(in: hood, angle: 90)
    } else {
        rgb(66, 108, 165).setFill()
        hood.fill()
    }
    blueInk.setStroke()
    hood.lineWidth = 0.75
    hood.lineJoinStyle = .round
    hood.stroke()

    let opening = NSBezierPath()
    opening.move(to: NSPoint(x: 20, y: 10))
    opening.curve(to: NSPoint(x: 15.1, y: 16), controlPoint1: NSPoint(x: 16.8, y: 12), controlPoint2: NSPoint(x: 15.1, y: 14))
    opening.curve(to: NSPoint(x: 20, y: 23.5), controlPoint1: NSPoint(x: 15.2, y: 19.5), controlPoint2: NSPoint(x: 17, y: 22.1))
    opening.curve(to: NSPoint(x: 24.9, y: 16), controlPoint1: NSPoint(x: 23, y: 22.1), controlPoint2: NSPoint(x: 24.8, y: 19.5))
    opening.curve(to: NSPoint(x: 20, y: 10), controlPoint1: NSPoint(x: 24.9, y: 14), controlPoint2: NSPoint(x: 23.2, y: 12))
    opening.close()
    blueInk.setFill()
    opening.fill()

    let face = NSBezierPath()
    face.move(to: NSPoint(x: 20, y: 12.2))
    face.curve(to: NSPoint(x: 16.8, y: 16.8), controlPoint1: NSPoint(x: 17.9, y: 13.6), controlPoint2: NSPoint(x: 16.8, y: 15.1))
    face.curve(to: NSPoint(x: 20, y: 21.8), controlPoint1: NSPoint(x: 16.8, y: 18.8), controlPoint2: NSPoint(x: 18.1, y: 20.5))
    face.curve(to: NSPoint(x: 23.2, y: 16.8), controlPoint1: NSPoint(x: 21.9, y: 20.5), controlPoint2: NSPoint(x: 23.2, y: 18.8))
    face.curve(to: NSPoint(x: 20, y: 12.2), controlPoint1: NSPoint(x: 23.2, y: 15.1), controlPoint2: NSPoint(x: 22.1, y: 13.6))
    face.close()
    paper.setFill()
    face.fill()

    let folds = NSBezierPath()
    folds.move(to: NSPoint(x: 15.7, y: 21.4)); folds.line(to: NSPoint(x: 13.5, y: 27))
    folds.move(to: NSPoint(x: 24.3, y: 21.4)); folds.line(to: NSPoint(x: 26.5, y: 27))
    folds.lineWidth = 0.75
    folds.lineCapStyle = .round
    blueInk.withAlphaComponent(0.6).setStroke()
    folds.stroke()

    for points: [(CGFloat, CGFloat)] in [
        [(7, 25), (20, 27), (20, 35), (7.5, 33)],
        [(20, 27), (33, 25), (32.5, 33), (20, 35)],
    ] {
        let page = path(points)
        paper.setFill()
        page.fill()
        blueInk.setStroke()
        page.lineWidth = 0.8
        page.stroke()
    }
    let rules = NSBezierPath()
    for segment: [(CGFloat, CGFloat)] in [
        [(10.2, 28.3), (17.2, 29.5)], [(29.8, 28.3), (22.8, 29.5)],
        [(10.4, 30.6), (17.2, 31.8)], [(29.6, 30.6), (22.8, 31.8)],
    ] {
        rules.move(to: NSPoint(x: segment[0].0, y: segment[0].1))
        rules.line(to: NSPoint(x: segment[1].0, y: segment[1].1))
    }
    rules.lineWidth = 0.55
    rules.lineCapStyle = .round
    blueInk.withAlphaComponent(0.65).setStroke()
    rules.stroke()
}

let variants = [(16, 1), (16, 2), (32, 1), (32, 2), (128, 1), (128, 2), (256, 1), (256, 2), (512, 1), (512, 2)]
for (size, scale) in variants {
    let pixels = size * scale
    guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
          let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
        throw NSError(domain: "WriterCraftIcon", code: 2, userInfo: [NSLocalizedDescriptionKey: "Cannot create the icon bitmap."])
    }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    context.imageInterpolation = .high
    context.cgContext.setShouldAntialias(true)
    context.cgContext.translateBy(x: 0, y: CGFloat(pixels))
    context.cgContext.scaleBy(x: CGFloat(pixels) / 40, y: -CGFloat(pixels) / 40)

    let tile = NSBezierPath(roundedRect: NSRect(x: 2.5, y: 2.5, width: 35, height: 35), xRadius: 7.4, yRadius: 7.4)
    rgb(244, 239, 226).setFill()
    tile.fill()
    if let gradient = NSGradient(starting: rgb(255, 251, 241), ending: rgb(229, 221, 207)) {
        gradient.draw(in: tile, angle: 90)
    }
    rgb(183, 174, 158, 0.45).setStroke()
    tile.lineWidth = 0.25
    tile.stroke()
    context.cgContext.translateBy(x: 4, y: 3.5)
    context.cgContext.scaleBy(x: 0.8, y: 0.8)
    drawMark()
    NSGraphicsContext.restoreGraphicsState()

    guard let png = bitmap.representation(using: .png, properties: [:]) else {
        throw NSError(domain: "WriterCraftIcon", code: 3, userInfo: [NSLocalizedDescriptionKey: "Cannot encode the icon PNG."])
    }
    let suffix = scale == 2 ? "@2x" : ""
    try png.write(to: destination.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"))
}
