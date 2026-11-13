#!/usr/bin/env python3
"""Builds logix-core for one platform (docs/design.md).

    scripts/build.py --platform darwin-arm64 --out _out [--work _work]

Reads pins.json, downloads each pinned upstream file for the platform
(GitHub releases through the `gh` CLI, other files by URL), unpacks each
into its own subfolder unchanged (only a single top-level folder is
stripped), checks that the programs Logix Engine runs are there, writes
logix-core.json (what's inside, how to run it, and the FPGAs the tools can
build: scripts/fpgas.py), and packs it all as a reproducible
.tgz: fixed file order, times and owners, so the same pins give the same
bytes and the same SHA-256.

Not a product: logix-core is meant only for Logix Engine, which downloads
and runs it, and its content may change at any time.
"""

import argparse
import gzip
import os
import posixpath
import hashlib
import json
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fpgas  # noqa: E402  (scripts/fpgas.py)

PLATFORMS = ("darwin-arm64", "linux-x86-64", "windows-x86-64")

# The programs Logix Engine runs, by component (relative to the
# component's folder). A core without them isn't published.
REQUIRED = {
    "unix": {
        "oss-cad-suite": [
            "bin/yosys",
            "bin/nextpnr-ice40",
            "bin/nextpnr-ecp5",
            "bin/nextpnr-himbaechel",
            "bin/iverilog",
            "bin/vvp",
            "bin/openFPGALoader",
            "bin/iceprog",
        ],
        "openxc7": ["bin/nextpnr-xilinx", "bin/fasm2frames", "bin/xc7frames2bit"],
        "verible": ["bin/verible-verilog-format", "bin/verible-verilog-ls"],
        "elkjs": ["lib/elk.bundled.js", "LICENSE.md"],
    },
    "windows": {
        "oss-cad-suite": [
            "bin/yosys.exe",
            "bin/nextpnr-ice40.exe",
            "bin/nextpnr-ecp5.exe",
            "bin/nextpnr-himbaechel.exe",
            "bin/iverilog.exe",
            "bin/vvp.exe",
            "bin/openFPGALoader.exe",
        ],
        "openxc7": ["bin/nextpnr-xilinx.exe", "bin/fasm2frames.cmd", "bin/xc7frames2bit.exe"],
        "verible": ["verible-verilog-format.exe", "verible-verilog-ls.exe"],
        "elkjs": ["lib/elk.bundled.js", "LICENSE.md"],
    },
}

NOTE = (
    "logix-core is meant only for Logix Engine, which downloads it and runs its tools. "
    "It isn't for use on its own, and its content and layout may change at any time."
)


def run(cmd, **kw):
    print("$", " ".join(str(c) for c in cmd), flush=True)
    subprocess.run(cmd, check=True, **kw)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def for_platform(comp: dict, key: str, platform: str):
    """A component's value for a platform: per platform (a map by platform)
    or the same for all (such as elkjs, which is JavaScript)."""
    value = comp[key]
    if isinstance(value, dict):
        if platform not in value:
            raise SystemExit(f"pins.json: {key} has no {platform} entry")
        return value[platform]
    return value


def download(comp: dict, platform: str, dest: Path) -> Path:
    name = for_platform(comp, "files", platform)
    target = dest / name
    if target.exists():
        return target
    dest.mkdir(parents=True, exist_ok=True)
    if "github" in comp:
        g = comp["github"]
        run(["gh", "release", "download", g["release"], "--repo", g["repo"],
             "--pattern", name, "--dir", str(dest)])
    else:
        url = for_platform(comp, "url", platform)
        print("$ download", url, flush=True)
        with urllib.request.urlopen(url) as r, target.open("wb") as f:
            shutil.copyfileobj(r, f)
    return target


def unpack(archive: Path, dest: Path) -> None:
    """Unpacks `archive` into `dest`, unchanged, stripping a single
    top-level folder when the archive has one and nothing else."""
    tmp = dest.with_name(dest.name + ".unpack")
    shutil.rmtree(tmp, ignore_errors=True)
    tmp.mkdir(parents=True)
    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as z:
            z.extractall(tmp)
    else:
        with tarfile.open(archive) as t:
            t.extractall(tmp, filter="tar")
    entries = list(tmp.iterdir())
    root = entries[0] if len(entries) == 1 and entries[0].is_dir() and not entries[0].is_symlink() else tmp
    shutil.rmtree(dest, ignore_errors=True)
    if root is tmp:
        tmp.rename(dest)
    else:
        root.rename(dest)
        shutil.rmtree(tmp, ignore_errors=True)


def core_relative(component: str, path: str) -> str:
    """A path relative to a component (pins.json's path and env), relative
    to the core instead: "bin" in openxc7 is "openxc7/bin", and
    "../oss-cad-suite/bin" reaches another component's folder."""
    return posixpath.normpath(f"{component}/{path}")


def check_required(core: Path, platform: str) -> None:
    kind = "windows" if platform.startswith("windows") else "unix"
    missing = [f"{c}/{p}" for c, paths in REQUIRED[kind].items() for p in paths
               if not (core / c / p).exists()]
    if missing:
        sys.exit("Missing in logix-core: " + ", ".join(missing))


def check_yosys_release(core: Path, pins: dict) -> None:
    """openXC7 is built against one YosysHQ suite release and must run with
    its yosys (docs/design.md, openXC7): its package's BUILD-INFO.json names
    that release as `yosys-release-tag`, which must be the suite release
    pins.json pins. A core that mixes them isn't published."""
    want = pins["components"]["oss-cad-suite"]["github"]["release"]
    info = core / "openxc7" / "BUILD-INFO.json"
    if not info.is_file():
        sys.exit("openxc7: no BUILD-INFO.json, so its yosys release can't be checked")
    got = json.loads(info.read_text()).get("yosys-release-tag")
    if got != want:
        sys.exit(f"openxc7 was built for the YosysHQ suite {got!r}, but pins.json pins {want!r}: "
                 "pin an openXC7 build made for that suite, or that suite")


def pack(core: Path, out: Path) -> None:
    """A reproducible .tgz of the contents of `core` (no top folder): sorted
    entries, time 0, owner 0, gzip without a name or time."""
    paths = []
    for root, dirs, files in os.walk(core, followlinks=False):
        for name in dirs + files:
            paths.append(Path(root) / name)
    paths.sort(key=lambda p: p.relative_to(core).as_posix())

    def clean(info: tarfile.TarInfo) -> tarfile.TarInfo:
        info.mtime = 0
        info.uid = info.gid = 0
        info.uname = info.gname = ""
        return info

    with out.open("wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0,
                                              compresslevel=9) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.PAX_FORMAT) as tar:
            for p in paths:
                tar.add(p, arcname=p.relative_to(core).as_posix(), recursive=False, filter=clean)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--platform", required=True, choices=PLATFORMS)
    ap.add_argument("--out", required=True, help="folder for the .tgz")
    ap.add_argument("--work", default="_work", help="scratch folder for downloads")
    ap.add_argument("--package-tag", default="", help="yyyymmdd for the file name")
    args = ap.parse_args()

    repo = Path(__file__).resolve().parent.parent
    pins = json.loads((repo / "pins.json").read_text())
    work = Path(args.work).resolve()
    core = work / args.platform / "logix-core"
    shutil.rmtree(core, ignore_errors=True)
    core.mkdir(parents=True)

    meta = {"format": 1, "note": NOTE, "platform": args.platform, "components": {}}
    for name, comp in pins["components"].items():
        archive = download(comp, args.platform, work / "downloads" / args.platform)
        unpack(archive, core / name)
        source = {"upstream": comp["upstream"], "file": archive.name, "sha256": sha256(archive)}
        if "github" in comp:
            source["repo"] = comp["github"]["repo"]
            source["release"] = comp["github"]["release"]
        else:
            source["url"] = for_platform(comp, "url", args.platform)
            source["version"] = comp["version"]
        meta["components"][name] = {
            "description": comp["description"],
            "folder": name,
            "source": source,
            # How Logix Engine runs the component's programs: these
            # folders, relative to the core, go on PATH.
            "run": {"path": [core_relative(name, p) for p in for_platform(comp, "path", args.platform)],
                    # Variables the programs need, each naming a folder
                    # (such as VERILATOR_ROOT on Windows), relative to the core.
                    "env": {k: core_relative(name, v)
                            for k, v in comp.get("env", {}).get(args.platform, {}).items()}},
        }
    check_required(core, args.platform)
    check_yosys_release(core, pins)
    # What the tools can build, per toolchain, in their own terms: Logix
    # Engine checks its pinned catalog against it.
    meta["fpgas"] = fpgas.list_fpgas(core)
    (core / "logix-core.json").write_text(json.dumps(meta, indent=2) + "\n")

    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    tag = f"-{args.package_tag}" if args.package_tag else ""
    tgz = out / f"logix-core-{args.platform}{tag}.tgz"
    pack(core, tgz)
    print(f"{tgz.name}: {tgz.stat().st_size} bytes, sha256 {sha256(tgz)}")


if __name__ == "__main__":
    main()
