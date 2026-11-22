#!/bin/bash
# Builds Logix Viewer (apps/viewer, Open Logix's own program) for the
# platform this runs on, and packs it as the .tgz scripts/build.py takes as
# the logix-viewer component's download:
#
#     scripts/build-viewer.sh <platform> <out folder>
#
# macOS: one .app per kind of page (apps.json), ad-hoc signed; Windows and
# Linux: the program, apps.json and the icons. Paths are remapped so the
# program doesn't name the machine that built it.
set -euo pipefail
platform="$1"
out="$(mkdir -p "$2" && cd "$2" && pwd)"
repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo/apps/viewer"
# python3 on macOS and Linux, python in Git's bash on Windows.
py="$(command -v python3 || command -v python)"

export RUSTFLAGS="--remap-path-prefix=$repo=logix-core --remap-path-prefix=$HOME=~"
cargo build --release --locked

exe=target/release/logix-viewer
[ "${platform#windows}" != "$platform" ] && exe=target/release/logix-viewer.exe
stage="$(mktemp -d)"
"$py" bundle.py --platform "$platform" --binary "$exe" --out "$stage/logix-viewer"
# A .tgz, not a zip, so the programs keep their executable bit; fixed
# order, times and owners, so the same source gives the same bytes.
"$py" - "$stage/logix-viewer" "$out/logix-viewer-$platform.tgz" <<'PY'
import gzip, os, sys, tarfile
src, dest = sys.argv[1], sys.argv[2]
paths = []
for root, dirs, files in os.walk(src):
    for name in dirs + files:
        paths.append(os.path.join(root, name))
paths.sort(key=lambda p: os.path.relpath(p, src).replace(os.sep, "/"))
def clean(info):
    info.mtime = 0; info.uid = info.gid = 0; info.uname = info.gname = ""
    return info
with open(dest, "wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as gz:
    with tarfile.open(fileobj=gz, mode="w", format=tarfile.PAX_FORMAT) as tar:
        for p in paths:
            tar.add(p, arcname=os.path.relpath(p, src).replace(os.sep, "/"), recursive=False, filter=clean)
PY
echo "logix-viewer-$platform.tgz"
