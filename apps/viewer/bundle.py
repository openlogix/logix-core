#!/usr/bin/env python3
"""Packs the logix-viewer program as one app per kind of page (apps.json).

    bundle.py --platform darwin-arm64 --binary <logix-viewer> --out <folder>

macOS: <out>/<name>.app for each kind (Info.plist, the program, the icon),
ad-hoc signed, as Apple Silicon requires. Other platforms: the program
once, plus each kind's icon, since there an app's identity is set by the
program at run time (Windows' app ID, Linux's desktop entry).
Reproducible: nothing in it depends on the build's time or machine.
"""

import argparse
import json
import plistlib
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--platform", required=True)
    p.add_argument("--binary", required=True, type=Path)
    p.add_argument("--out", required=True, type=Path)
    args = p.parse_args()
    apps = json.loads((HERE / "apps.json").read_text())["apps"]
    args.out.mkdir(parents=True, exist_ok=True)
    if args.platform.startswith("darwin"):
        for app in apps:
            bundle = args.out / f"{app['name']}.app"
            if bundle.exists():
                shutil.rmtree(bundle)
            macos = bundle / "Contents" / "MacOS"
            resources = bundle / "Contents" / "Resources"
            macos.mkdir(parents=True)
            resources.mkdir(parents=True)
            shutil.copy2(args.binary, macos / "logix-viewer")
            shutil.copyfile(HERE / "icons" / f"{app['icon']}.icns", resources / "icon.icns")
            info = {
                "CFBundleName": app["name"],
                "CFBundleDisplayName": app["name"],
                "CFBundleIdentifier": app["id"],
                "CFBundleExecutable": "logix-viewer",
                "CFBundleIconFile": "icon",
                "CFBundlePackageType": "APPL",
                "CFBundleInfoDictionaryVersion": "6.0",
                "LSMinimumSystemVersion": "11.0",
                "NSHighResolutionCapable": True,
            }
            with open(bundle / "Contents" / "Info.plist", "wb") as f:
                plistlib.dump(info, f, sort_keys=True)
            subprocess.run(["codesign", "--force", "--sign", "-", str(bundle)], check=True, capture_output=True)
            print(f"{bundle.name}")
    else:
        exe = "logix-viewer.exe" if args.platform.startswith("windows") else "logix-viewer"
        shutil.copy2(args.binary, args.out / exe)
        ext = "ico" if args.platform.startswith("windows") else "png"
        for app in apps:
            shutil.copyfile(HERE / "icons" / f"{app['icon']}.{ext}", args.out / f"{app['kind']}.{ext}")
        print(exe)
    shutil.copyfile(HERE / "apps.json", args.out / "apps.json")


if __name__ == "__main__":
    main()
