# logix-core: design

How Logix Engine gets its FPGA tools without Apio's packages, and why. This
document replaces the Claude Doc "Open Logix: FPGA Tools Without Apio's
Packages" (reviewed 2026-11-10), which is now obsolete. It's a Milestone 2
goal of Logix Engine ("Decouple from Apio's releases", the engine's
`docs/requirements.md`). How engines already on users' machines get a newer
core is the dependency updates discussion's subject, not this document's.

> [!IMPORTANT]
> **Use it through Logix Engine.** logix-core is meant only for [Logix Engine](https://www.openlogix.org/engine/), which downloads it and runs its tools. It isn't for use on its own, and its content and layout may change at any time.

## Decisions (2026-11-10)

- **Two packages:** `logix-core` holds the tools; `logix-catalog`, built by
  the Logix Catalog repo, holds the boards and FPGAs. Examples belong to
  each frontend. The catalog changes often and is small (a board fix is
  well under 1 MB, not a new 640 MB core), and it arrives first on a new
  install, so a frontend can list boards while the tools download.
- **One repo, `openlogix/logix-core`, with one workflow.** It holds no
  tools, only what to fetch and how to pack it.
- **Upstream releases, unchanged:** the workflow downloads each pinned
  upstream file at build time, as Apio does, unpacks it, and places its
  folder unchanged in the package. Engines download `logix-core`, never
  upstream. Logix builds none of the tools, openXC7 included, with one
  exception: **a tool upstream publishes no build of is built here** from
  its pinned upstream source, unmodified (the user's rule, 2026-11-13).
  Today that's only `wdi-simple`. Open Logix's own small programs the
  engine runs are built here too, from this repo (the user's decision,
  2026-11-22): today Logix Viewer, `apps/viewer/`.
- **The suite whole,** not trimmed to the programs the engine runs.
- **How to run each tool** (the folders for `PATH`, variables) lives in
  `logix-core.json`, per tool, not in the engine; Apio hard-wires it in its
  `config.jsonc`.
- **The engine's remote config names one `logix-core` and one
  `logix-catalog`** per channel, as Apio's remote config names its
  packages. Engines have no built-in pins, so a new core reaches installed
  engines without an engine release (built 2026-11-12).
- **Windows drivers** (decided 2026-11-13): libwdi's `wdi-simple`, run by
  Logix Engine (drivers are the engine's job), in the Windows package only,
  as `wdi-simple/` (the user's decision, Milestone 3's driver phase: built
  in this repo's workflow rather than in a repo of its own). See
  wdi-simple, below.

## Why

- **Control:** the engine changes tools only when Logix decides, tested with
  its own engine, not when another project's daily job runs.
- **Windows is next** (Milestone 3): Apio needs three more packages there.
- **Fewer middle steps:** Apio's packages mostly repackage upstream releases.

## What was in Apio's packages

Apio publishes six packages; five are repackaged upstream releases or
committed binaries, and one is Apio's own data. Every package also carries
a `BUILD-INFO.json`, and Apio's `config.jsonc` sets each platform's `PATH`
and variables for them.

| Apio package | What's in it | Taken from | macOS ARM | Linux x64 | Windows x64 | Logix |
| --- | --- | --- | --- | --- | --- | --- |
| `definitions` | Apio's boards, FPGAs and programmers (`boards.jsonc`, `fpgas.jsonc`, `programmers.jsonc`), 100 examples | `fpgawars/apio-definitions` (Apio's own data) | yes | yes | yes | not used: Logix Catalog and the frontends' examples |
| `oss-cad-suite` | the whole YosysHQ suite, copied as is (minus its `VERSION` file), with `dot` on macOS and Linux | YosysHQ `oss-cad-suite-build`, release 2026-11-04 | `.tgz` (526 MB) | `.tgz` | `.tgz` | `oss-cad-suite/` |
| `oss-cad-suite`, added | four generated part lists, `<ARCH>-PARTS-INDEX.json` (iCE40, ECP5, Gowin, MachXO2) | Apio's `fpgas_definitions.py` | yes | yes | yes | not used: Logix Catalog's FPGA files |
| `openxc7` | nextpnr-xilinx, `fasm2frames`, `xc7frames2bit`, the Project X-Ray database, the chip databases | `cavearr/toolchain-openxc7-releases`, release 2026-11-05 | `.tgz` (124 MB) | `.tgz` | `.tgz` | `openxc7/` |
| `verible` | the Verible tools | chipsalliance `verible`, release v0.0-4294-gc1d8f5e8 | `.tar.gz` (8 MB) | static `.tar.gz` | `.zip` | `verible/` |
| `drivers` | `zadig.exe` (+ `zadig.ini`), `serial_install.exe`, `ftdi_eeprom.exe` | committed as files in `fpgawars/tools-drivers` | none | none | yes | `wdi-simple/` |
| `graphviz` | Graphviz's `dot` | Graphviz 12.1.2, Windows build | none | none | yes | not used: Logix Engine lays out and draws its diagrams itself, with `elkjs/` |

Apio's tool packages are rebuilt every day, whether or not anything
upstream changed, and its remote config pins the dated release each Apio
version uses.

## What Logix Engine runs

| Job | Programs | Folder |
| --- | --- | --- |
| Lint | `verilator_bin` | `oss-cad-suite/` |
| Build, iCE40 | `yosys`, `nextpnr-ice40`, `icepack` | `oss-cad-suite/` |
| Build, ECP5 | `yosys`, `nextpnr-ecp5`, `ecppack` | `oss-cad-suite/` |
| Build, Gowin (GW1N, GW2A, GW5A) | `yosys`, `nextpnr-himbaechel`, `gowin_pack` (Apycula, Python) | `oss-cad-suite/` |
| Build, Xilinx 7-series | `yosys`, `nextpnr-xilinx`, `fasm2frames` (Python), `xc7frames2bit`, the Project X-Ray database | `oss-cad-suite/` + `openxc7/` |
| Test and simulate | `iverilog`, `vvp` (the waveforms show in Logix Viewer, below) | `oss-cad-suite/` |
| Format | `verible-verilog-format` | `verible/` |
| Diagram | `yosys`, then ELK's layout (`elk.bundled.js`), which the engine runs itself | `oss-cad-suite/` + `elkjs/` |
| Program the board | `openFPGALoader`, `iceprog`, `dfu-util`, `icesprog` | `oss-cad-suite/` |
| Live errors in Logix IDE | `verible-verilog-ls` | `verible/` |
| Windows drivers | `wdi-simple` (libwdi), run by Logix Engine | `wdi-simple/` (Windows only) |
| Show pages in windows | Logix Viewer: `Logix Waveforms.app`, `Logix Diagrams.app` and `Logix Viewer.app` on macOS, `logix-viewer` elsewhere, with `apps.json` | `logix-viewer/` |

The engine scans USB with its own libusb, built into the engine. Everything
else in the suite (such as GHDL and formal verification tools) is shipped
but unused today.

## How it works

1. **Pin by pull request:** each tool's upstream version is in
   [`pins.json`](../pins.json), changed by a pull request, never by a daily
   job.
2. **One `.tgz` per platform,** every tool a folder in it: the upstream
   `oss-cad-suite/`, `openxc7/`, `verible/` and `elkjs/`, `wdi-simple/` on
   Windows (built here, see wdi-simple) and Open Logix's own
   `logix-viewer/` (see Logix Viewer).
   The workflow builds it into the day's pre-release, on every change and
   daily (so breakage from GitHub's runners or upstream shows up). Old
   pre-releases are deleted by the shared policy: one stays while a remote
   config on `main` names it, or while it's among the 5 newest. The
   engine's prod remote config names only stable releases, which are never
   deleted.
3. **Reproducible:** fixed file order, times and owners, so the same pins
   give the same bytes and the same SHA-256, and an engine can tell an
   unchanged core from a new one.
4. **Copy, don't modify:** each upstream folder is unchanged (only an
   archive's single top folder is stripped), which also keeps upstream bug
   reports simple: a problem reproduces with upstream's own files.
5. **Metadata inside:** `logix-core.json` lists, per folder, the tool, its
   upstream source and version, the upstream file's SHA-256, and how to run
   it (`run.path`, the folders for `PATH`, the first holding its programs,
   which may include another component's, as openXC7 includes the YosysHQ
   suite's for its yosys and Python; `run.env`, variables naming folders,
   such as `VERILATOR_ROOT` on Windows; all relative to the core), and the
   FPGAs the tools can build (see The FPGA list). It holds
   nothing that changes from day to day, so it doesn't break
   reproducibility; the build details (date, workflow run) are in the
   release's `BUILD-INFO.json`, beside the packages.
6. **openXC7 stays upstream's to build:** it's built today in cavearr's
   repo, which is to move to the openXC7 project and, with no date set, into
   the YosysHQ suite. Logix takes its release like the others and takes no
   responsibility for the build.
7. **The engine's remote config** (`remote-config/logix-engine/<version>/`
   in the engine's repo, a config file per channel) names the release and
   file of `logix-core` here, and of `logix-catalog`; the engine checks each
   download against the release's `SHA256SUMS`. How to run each tool comes
   from `logix-core.json`.
8. **Tested before use:** a remote config names a core only after the
   engine's tests pass with it on each platform's own runner (a build with
   each toolchain, a test, a format, a diagram).
9. **The engine unpacks one archive type itself** (`.tgz` on every
   platform); nothing is installed.

## The FPGA list

`logix-core.json` also lists the FPGAs the core's tools can build, per
toolchain, in the tools' own terms (decided 2026-11-10): `fpgas.ice40`
(nextpnr's device type and package), `fpgas.ecp5` (device type, package,
speed grade), `fpgas.gowin` (the part nextpnr takes and its packer device)
and `fpgas.openxc7` (the part). Logix Engine checks the configuration it
pins: every FPGA in its `logix-catalog` must be in its `logix-core`'s list,
so a core that drops or renames an FPGA, or a catalog that adds one the
tools can't build, fails the engine's tests before it's pinned. The list
doesn't replace the catalog's FPGA files: those are reviewed facts about
the chips, with fields no toolchain database has.

[`scripts/fpgas.py`](../scripts/fpgas.py) generates it at build time, as
Apio's package builds did, reading data files only, so every platform's
core is built on Linux:

| Toolchain | Source |
| --- | --- |
| iCE40 | icebox's chip databases (`share/icebox/chipdb-*.txt`), with nextpnr's device types |
| ECP5 | Project Trellis's `devices.json` and each device's `iodb.json` |
| Gowin | Apycula's databases (`*.msgpack.xz`: the parts each device serves) and nextpnr's Gowin chip databases (the devices it places); GW5AT-60B is left out, since nextpnr can't place a design on it yet |
| Xilinx 7-series | openXC7's own inventory, `XILINX-PARTS-INVENTORY.json` (the parts its release built) |

## Processes

### Moving to a new upstream release (such as a newer YosysHQ suite)

1. A pull request here changes the pinned version.
2. The workflow builds `logix-core` for all three platforms into the next
   dated pre-release, with its SHA-256.
3. A pull request to the engine's remote config names it (the dev
   channel's config may name a pre-release, the prod one only a stable
   release); the engine's tests build, test, format and draw with it.
4. Merged, engines on that channel get it without an engine release.

### Adding a platform

1. `pins.json` adds the platform's upstream files and run folders.
2. The engine's remote config checks gain the platform, and the engine's
   tests get that platform's runner.

### A broken upstream release

1. The engine's tests fail with the new core, so it never reaches users.
2. The engine keeps the last good core, and the problem is reported
   upstream.

### wdi-simple

libwdi's command-line example, which installs a driver for one USB device's
interface (Logix Engine installs WinUSB for boards whose programmers need it
on Windows). libwdi publishes Zadig built, but not `wdi-simple`, so:

1. **The pin is libwdi's source:** `pins.json` names `pbatard/libwdi`, a
   release tag and the commit it names. The build checks out the commit and
   fails if the tag names another one (a tag can move, a commit can't). It
   also pins Microsoft's WDF co-installer package (which libwdi embeds for
   WinUSB) by URL and SHA-256.
2. **WinUSB only** (the user's decision, 2026-11-13): no libusb0 or libusbK
   files, as Apio's users choose WinUSB in Zadig for the same boards.
3. **Built in the workflow** (`build-wdi-simple`, on Windows with MSYS2,
   `scripts/build-wdi-simple.sh`), unmodified, into a zip of
   `wdi-simple.exe` and libwdi's licenses (LGPL-3.0 and the GPL-3.0 text it
   builds on), which the Windows package build unpacks like a download.
4. **Stable bytes:** a compiler build isn't reproducible from day to day
   (MSYS2's toolchain moves), so the zip is cached by its pin and the
   script, and built again only when either changes; the program is linked
   without a time stamp, so a rebuild is more likely to match. If the cache
   is ever lost, the next core has a new SHA-256 for the same pins, and
   engines download it again once.
5. **Moving to a new libwdi release:** change the tag and commit (and the
   WDK pin, if libwdi asks for another one) in one pull request.

### Logix Viewer

Open Logix's own program, not an upstream tool (logix-engine's
`docs/requirements.md`, Logix Viewer): it shows the engine's pages in
windows of their own, one app per kind of page, each with its own name and
icon. Its source is this repo's `apps/viewer/` (Rust: tao, wry, muda, rfd).

1. **Built in the workflow** (`build-viewer`, on each platform's own runner:
   macOS for the `.app` bundles and their ad-hoc signature, Ubuntu 22.04
   with WebKitGTK and GTK, Windows), by `scripts/build-viewer.sh`: a release
   build with paths remapped, then `apps/viewer/bundle.py`, packed as a
   reproducible `.tgz` (a zip would lose the programs' executable bit),
   which the package build unpacks like a download.
   On Linux it needs only WebKitGTK and GTK, which desktops have: muda's
   `libxdo` feature is off (it serves only predefined menu items, and the
   viewer has no menu on Linux), and the runner doesn't install libxdo, so
   a build that links it fails (found in the Linux test, 2026-11-23).
2. **Stable bytes:** cached by its sources and the script, as `wdi-simple`
   is, so it's built again only when they change.
3. **No signing of its own,** as for every program here: the engine
   downloads the package, so nothing is quarantined.
4. **Icons:** `apps/viewer/icons/*.svg`, variants of Logix Engine's icon,
   made into the committed `.icns`, `.ico` and `.png` by
   `icons/make-icons.py` on a Mac.
   On macOS each app bundle has its icon. On Linux and Windows the one
   program learns its app from `apps.json` by the name it's started with
   (`src/identity.rs`): it sets each window's icon, and on Linux it also
   takes the app's ID as its program name, which GTK gives the window as
   its app ID (Wayland) and class (X11), and writes a hidden desktop entry
   by that name in `~/.local/share/applications`, where docks find the
   icon. Without them GNOME showed a generic gear (the Linux test,
   2026-11-23).
5. **Source lines:** Logix Viewer opens no sources: it has no editor, and
   the system's default app for a `.v` file is rarely one (the user's
   decision, 2026-11-22). Its `window.logixHost` leaves `openSource` out of
   `features` (`src/host.js`), so pages show a place, such as "Declared in
   main_tb.v, line 12", only in their hover boxes and offer no double-click
   to it; `openSource` itself does nothing, for older pages. Until
   2026-11-23 it showed the place in a note, which only repeated the hover
   box (the user's decision). Hosts with an editor, such as Logix IDE, list
   it and open the place.

### openXC7

1. Its pin names an openXC7 build release (cavearr's repo now, the openXC7
   project later) built against a YosysHQ release. Each openXC7 package's
   `BUILD-INFO.json` names that release (`yosys-release-tag`), and the build
   fails unless it's the YosysHQ release pinned here (`scripts/build.py`,
   `check_yosys_release`).
2. A new YosysHQ release and its matching openXC7 build move in one pull
   request; once openXC7 is in the YosysHQ suite, its folder goes away.

## Facts to check

- [ ] How long YosysHQ keeps its dated releases (`logix-core` keeps its own
  copy, so this only affects rebuilding old cores).
- [ ] The licenses of each tool, and what redistributing them requires
  (notices, source links).
- [x] Whether the Windows suite has everything the engine runs, including a
  working `gowin_pack` and Python: yes, since the Windows promotion
  (2026-11-16; openXC7 2026-11-15 finds the suite's Python, apio#1194).
- [ ] Whether the suite's `nextpnr-himbaechel` and the openXC7 build need the
  same YosysHQ release, as Apio asserts.

## Sources

Read on 2026-11-10: Apio's
[`config.jsonc`](https://github.com/FPGAwars/apio/blob/main/apio/resources/config.jsonc)
and [remote config `apio-1.7.x.jsonc`](https://github.com/FPGAwars/apio/blob/main/remote-config/apio-1.7.x.jsonc);
the build workflows and scripts of
[`tools-oss-cad-suite`](https://github.com/FPGAwars/tools-oss-cad-suite),
[`tools-openxc7`](https://github.com/FPGAwars/tools-openxc7),
[`tools-verible`](https://github.com/FPGAwars/tools-verible),
[`tools-drivers`](https://github.com/FPGAwars/tools-drivers),
[`tools-graphviz`](https://github.com/FPGAwars/tools-graphviz) and
[`apio-definitions`](https://github.com/FPGAwars/apio-definitions); Logix
Engine's package manifest and source.
