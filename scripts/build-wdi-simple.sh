#!/bin/bash
# Builds libwdi's wdi-simple.exe, the WinUSB driver installer Logix Engine
# runs on Windows (docs/design.md, wdi-simple). libwdi publishes no build of
# it, so logix-core builds it from the libwdi source pins.json pins,
# unmodified, for WinUSB only (no libusb0 or libusbK drivers).
#
#   scripts/build-wdi-simple.sh OUT_DIR
#
# Writes OUT_DIR/<the zip pins.json names>: wdi-simple.exe and libwdi's
# licenses (LGPL-3.0, with the GPL-3.0 text it builds on). Runs in MSYS2's
# MINGW64 shell on Windows, with git, curl, jq and zip (the
# build-pre-release workflow's build-wdi-simple job). The workflow
# caches the zip by the pin and this script, so a new build happens only
# when either changes and the core keeps its bytes from day to day.

set -euo pipefail

out="$(mkdir -p "$1" && cd "$1" && pwd)"
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
pin() { jq -r ".components[\"wdi-simple\"]$1" "$repo_dir/pins.json"; }

repo="$(pin .build.repo)"
tag="$(pin .build.tag)"
commit="$(pin .build.commit)"
wdk_url="$(pin .build.wdk.url)"
wdk_sha256="$(pin .build.wdk.sha256)"
zip_name="$(pin '.files["windows-x86-64"]')"

work="$(mktemp -d)"
cd "$work"

# The pinned commit, which the pinned tag must still name: a tag can move,
# a commit can't.
git clone --quiet "https://github.com/$repo.git" src
cd src
git checkout --quiet "$commit"
tagged="$(git rev-parse "refs/tags/$tag^{commit}")"
if [[ "$tagged" != "$commit" ]]; then
  echo "error: $repo's tag $tag names $tagged, not the pinned $commit" >&2
  exit 1
fi
git submodule update --quiet --init --recursive

# Microsoft's WDF co-installers, which libwdi embeds for WinUSB, checked
# against the pinned SHA-256 and unpacked with an administrative install
# (which installs nothing).
curl -fsSL -o wdk.msi "$wdk_url"
echo "$wdk_sha256  wdk.msi" | sha256sum -c -
MSYS2_ARG_CONV_EXCL='*' msiexec /a "$(cygpath -w "$PWD/wdk.msi")" /qn \
  "TARGETDIR=$(cygpath -w "$PWD/wdk")"
[[ -d "wdk/Windows Kits/8.0" ]] || { echo "error: the WDK didn't unpack" >&2; exit 1; }

# No time stamp in the program, so a rebuild is more likely to give the
# same bytes.
./bootstrap.sh
./configure \
  --build=x86_64-w64-mingw32 \
  --host=x86_64-w64-mingw32 \
  --enable-examples-build \
  --disable-debug \
  --disable-shared \
  --disable-32bit \
  --with-wdkdir="wdk/Windows Kits/8.0" \
  --with-wdfver=1011 \
  LDFLAGS="-Wl,--no-insert-timestamp"
make
[[ -f examples/wdi-simple.exe ]] || { echo "error: no examples/wdi-simple.exe" >&2; exit 1; }

# A reproducible zip: fixed times, no extra attributes.
mkdir "$work/pkg"
cp examples/wdi-simple.exe "$work/pkg/"
cp COPYING-LGPL "$work/pkg/LICENSE-LGPL-3.0.txt"
cp COPYING "$work/pkg/LICENSE-GPL-3.0.txt"
cd "$work/pkg"
touch -t 198001010000 LICENSE-GPL-3.0.txt LICENSE-LGPL-3.0.txt wdi-simple.exe
rm -f "$out/$zip_name"
zip -X -D -q "$out/$zip_name" LICENSE-GPL-3.0.txt LICENSE-LGPL-3.0.txt wdi-simple.exe
ls -l "$out/$zip_name"
sha256sum "$out/$zip_name"
