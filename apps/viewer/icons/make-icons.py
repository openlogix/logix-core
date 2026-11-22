#!/usr/bin/env python3
"""Makes the viewer apps' icon files from the SVGs here: <kind>.icns
(macOS), <kind>.ico (Windows) and <kind>.png (Linux, 256 px). Run it on a
Mac (it uses sips and iconutil) whenever an SVG changes, and commit the
results: the package builds only copy them.

The SVGs are variants of Logix Engine's icon (the website repo's brand,
docs/brand/engine.svg): its purple tile and chip, with each app's glyph.
On macOS the tile fills 824 of 1024 px, Apple's icon grid, as Logix IDE's.
"""

import struct
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
KINDS = ["waveforms", "diagram", "viewer"]


def render(svg: Path, size: int, out: Path) -> None:
    subprocess.run(
        ["sips", "-s", "format", "png", "-z", str(size), str(size), str(svg), "--out", str(out)],
        check=True,
        capture_output=True,
    )


def mac_svg(svg: Path, out: Path) -> None:
    """The icon inset on Apple's 1024 px grid: the 64-unit tile at 824 px."""
    inner = svg.read_text().split(">", 1)[1].rsplit("</svg>", 1)[0]
    scale = 824 / 64
    out.write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">'
        f'<g transform="translate(100 100) scale({scale})">{inner}</g></svg>\n'
    )


def icns(svg: Path, out: Path, tmp: Path) -> None:
    mac = tmp / "mac.svg"
    mac_svg(svg, mac)
    iconset = tmp / "icon.iconset"
    iconset.mkdir()
    for size in [16, 32, 128, 256, 512]:
        render(mac, size, iconset / f"icon_{size}x{size}.png")
        render(mac, size * 2, iconset / f"icon_{size}x{size}@2x.png")
    subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(out)], check=True)


def ico(svg: Path, out: Path, tmp: Path) -> None:
    """An .ico of PNG images, which Windows Vista and later read."""
    sizes = [16, 24, 32, 48, 64, 256]
    images = []
    for size in sizes:
        png = tmp / f"ico-{size}.png"
        render(svg, size, png)
        images.append(png.read_bytes())
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = len(header) + 16 * len(images)
    entries = b""
    for size, data in zip(sizes, images):
        dim = 0 if size == 256 else size
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    out.write_bytes(header + entries + b"".join(images))


def main() -> None:
    if sys.platform != "darwin":
        sys.exit("make-icons.py runs on macOS (it uses sips and iconutil).")
    for kind in KINDS:
        svg = HERE / f"{kind}.svg"
        with tempfile.TemporaryDirectory() as t:
            tmp = Path(t)
            icns(svg, HERE / f"{kind}.icns", tmp)
            ico(svg, HERE / f"{kind}.ico", tmp)
            render(svg, 256, HERE / f"{kind}.png")
        print(f"{kind}: .icns, .ico, .png")


if __name__ == "__main__":
    main()
