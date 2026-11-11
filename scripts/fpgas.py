"""The FPGAs the tools in a logix-core can build, per toolchain, in the
tools' own terms (docs/design.md, "The FPGA list").

    fpgas.list_fpgas(core) -> {"ice40": [...], "ecp5": [...], "gowin": [...], "openxc7": [...]}

Generated from the databases in the unpacked core, reading data files only
(no program runs), so a core for any platform is built on any machine. The
databases are the same on every platform. Ported from Apio's generators
(FPGAwars/tools-oss-cad-suite fpgas_definitions.py); the Xilinx list is
openXC7's own inventory.

    ice40    oss-cad-suite/share/icebox/chipdb-*.txt, with NEXTPNR_ICE40_TYPES
    ecp5     oss-cad-suite/share/trellis/database: devices.json, ECP5/<device>/iodb.json
    gowin    Apycula's *.msgpack.xz (the parts each device serves) and nextpnr's
             share/nextpnr/himbaechel/gowin/chipdb-*.bin (the devices it places)
    openxc7  openxc7/share/nextpnr/himbaechel/xilinx/XILINX-PARTS-INVENTORY.json

The lists are sorted, so the same databases give the same bytes.
"""

import json
import lzma
import re
import sys
from pathlib import Path

import msgpack

# -- nextpnr-ice40's device types (--hx1k and so on) and the icebox die
# -- each runs on. A ':4k' die takes only that die's '<package>:4k'
# -- packages, named without the suffix (nextpnr's --lp4k/--hx4k).
NEXTPNR_ICE40_TYPES = {
    "lp384": "384",
    "lp1k": "1k",
    "hx1k": "1k",
    "lp4k": "8k:4k",
    "hx4k": "8k:4k",
    "lp8k": "8k",
    "hx8k": "8k",
    "up3k": "5k",
    "up5k": "5k",
    "u1k": "u4k",
    "u2k": "u4k",
    "u4k": "u4k",
}

# -- nextpnr-ecp5's device type prefix per series, and the speed grades
# -- Lattice sells (the LFE5UM5G only as -8).
ECP5_SERIES = {"LFE5U": ("", ["6", "7", "8"]), "LFE5UM": ("um-", ["6", "7", "8"]),
               "LFE5UM5G": ("um5g-", ["8"])}

# -- Gowin devices nextpnr has a database for but can't place a design on
# -- (found by Apio: a blinky fails). Left out until that changes.
GOWIN_UNUSABLE = {"GW5AT-60B"}

XILINX_INVENTORY = "share/nextpnr/himbaechel/xilinx/XILINX-PARTS-INVENTORY.json"


def ice40(suite: Path) -> list[dict]:
    dies = {}
    for path in (suite / "share/icebox").glob("chipdb-*.txt"):
        lines = path.read_text(encoding="utf-8").splitlines()
        die = [x.split()[1] for x in lines if x.startswith(".device ")][0]
        dies[die] = {x.split()[1] for x in lines if x.startswith(".pins ")}
    out = []
    for type_, die_spec in NEXTPNR_ICE40_TYPES.items():
        die, _, suffix = die_spec.partition(":")
        if die not in dies:
            sys.exit(f"ice40: no icebox database for die {die}")
        for package in dies[die]:
            if suffix:
                if not package.endswith(":" + suffix):
                    continue
                package = package.removesuffix(":" + suffix)
            out.append({"device": type_, "package": package})
    return sorted(out, key=lambda p: (p["device"], p["package"]))


def ecp5(suite: Path) -> list[dict]:
    database = suite / "share/trellis/database"
    devices = json.loads((database / "devices.json").read_text(encoding="utf-8"))
    out = []
    for device in devices["families"]["ECP5"]["devices"]:
        series, luts = re.fullmatch(r"(LFE5U[M5G]*)-(\d+)F", device).groups()
        prefix, speeds = ECP5_SERIES[series]
        iodb = json.loads((database / "ECP5" / device / "iodb.json").read_text(encoding="utf-8"))
        # -- iodb.json lists the packages nextpnr accepts (devices.json more).
        for package in iodb["packages"]:
            for speed in speeds:
                out.append({"device": f"{prefix}{luts}k", "package": package, "speed": speed})
    return sorted(out, key=lambda p: (p["device"], p["package"], p["speed"]))


def gowin(suite: Path) -> list[dict]:
    # -- lib/python3.x/site-packages/apycula on macOS and Linux; found
    # -- wherever it is, for Windows' layout.
    apycula = next((p for p in sorted((suite / "lib").rglob("apycula"))
                    if any(p.glob("*.msgpack.xz"))), None)
    if apycula is None:
        sys.exit("gowin: Apycula isn't in the suite")
    placed = {p.stem.removeprefix("chipdb-")
              for p in (suite / "share/nextpnr/himbaechel/gowin").glob("chipdb-*.bin")}
    out = []
    for path in sorted(apycula.glob("*.msgpack.xz")):
        device = path.name.split(".")[0]
        # -- A device needs both the packer's (Apycula) and the placer's
        # -- (nextpnr) database.
        if device not in placed or device in GOWIN_UNUSABLE:
            continue
        with lzma.open(path) as f:
            db = msgpack.unpackb(f.read(), raw=False, strict_map_key=False, use_list=False)
        for part in db["packages"]:
            out.append({"device": part, "packer_device": device})
    return sorted(out, key=lambda p: (p["device"], p["packer_device"]))


def openxc7(core: Path) -> list[dict]:
    inventory = json.loads((core / XILINX_INVENTORY).read_text(encoding="utf-8"))
    if inventory.get("schema") != 8:
        sys.exit(f"openxc7: unknown inventory schema {inventory.get('schema')}")
    return sorted(({"device": part} for part, p in inventory["parts"].items() if p["generated"]),
                  key=lambda p: p["device"])


def list_fpgas(core: Path) -> dict[str, list[dict]]:
    """The FPGAs of an unpacked core (with its oss-cad-suite/ and openxc7/)."""
    suite = core / "oss-cad-suite"
    lists = {"ice40": ice40(suite), "ecp5": ecp5(suite), "gowin": gowin(suite),
             "openxc7": openxc7(core / "openxc7")}
    for name, parts in lists.items():
        if len(parts) < 10:
            sys.exit(f"{name}: only {len(parts)} FPGAs found; a database moved?")
    return lists


if __name__ == "__main__":
    lists = list_fpgas(Path(sys.argv[1]))
    print(json.dumps({k: len(v) for k, v in lists.items()}))
