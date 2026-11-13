#!/bin/sh
# SPDX-License-Identifier: MIT
set -eu

cd "$(dirname "$0")/.."
assets=iphone/iDared32bit/Assets.xcassets
workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

sips -s format png -z 1024 1024 res/branding/app-icon.svg --out "$workdir/app-icon.png" >/dev/null
# Core Graphics writes RGB without an alpha channel for the iOS launcher.
swift - "$workdir/app-icon.png" "$assets/AppIcon.appiconset/AppIcon.png" <<'SWIFT'
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let input = URL(fileURLWithPath: CommandLine.arguments[1])
let output = URL(fileURLWithPath: CommandLine.arguments[2])
guard let source = CGImageSourceCreateWithURL(input as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil),
      let context = CGContext(
          data: nil, width: image.width, height: image.height, bitsPerComponent: 8,
          bytesPerRow: 0, space: CGColorSpace(name: CGColorSpace.sRGB)!,
          bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue
      ),
      let destination = CGImageDestinationCreateWithURL(
          output as CFURL, UTType.png.identifier as CFString, 1, nil
      ) else { exit(EXIT_FAILURE) }

context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
guard let rgb = context.makeImage() else { exit(EXIT_FAILURE) }
CGImageDestinationAddImage(destination, rgb, nil)
guard CGImageDestinationFinalize(destination) else { exit(EXIT_FAILURE) }
SWIFT
sips -z 512 512 "$assets/AppIcon.appiconset/AppIcon.png" --out res/icon.png >/dev/null
sips -s format png -z 144 144 res/branding/mark.svg \
  --out "$assets/AppPickerLogo.imageset/AppPickerLogo@3x.png" >/dev/null

python3 - <<'PY'
from pathlib import Path
import xml.etree.ElementTree as ET

res = Path("android/app/src/main/res")
ns = {"svg": "http://www.w3.org/2000/svg"}
source = ET.parse("res/branding/mark.svg").getroot()
background = ET.parse("res/branding/app-icon.svg").getroot().find("svg:path", ns)
mark = source.find("svg:path", ns)
_, _, width, height = source.attrib["viewBox"].split()
(res / "drawable").mkdir(exist_ok=True)
(res / "drawable" / "launcher_mark.xml").write_text(f"""<?xml version="1.0" encoding="utf-8"?>
<!-- Generated from res/branding/mark.svg by dev-scripts/render-branding.sh. -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="108dp" android:height="108dp"
    android:viewportWidth="{width}" android:viewportHeight="{height}">
    <path android:fillColor="{mark.attrib['fill']}" android:fillType="evenOdd"
        android:pathData="{mark.attrib['d']}" />
</vector>
""")
(res / "values" / "launcher_colors.xml").write_text(f"""<?xml version="1.0" encoding="utf-8"?>
<!-- Generated from res/branding/app-icon.svg by dev-scripts/render-branding.sh. -->
<resources>
    <color name="launcher_background">{background.attrib['fill']}</color>
</resources>
""")
PY
