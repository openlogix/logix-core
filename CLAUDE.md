# CLAUDE.md

Guidance for Claude Code (claude.ai/code) and other AI agents working in this repo.

## What this is

`logix-core` builds the package of open source FPGA tools that Logix Engine downloads and runs: one reproducible `.tgz` per platform (`darwin-arm64`, `linux-x86-64`, `windows-x86-64`), published as dated GitHub pre-releases. `docs/design.md` has the decisions, the reasons and the processes.

- **Not an Open Logix product:** no product page, no docs site, no `logix-` product rules beyond the shared ones. It's infrastructure for Logix Engine.
- **Not for use on its own.** Say so wherever people meet it (README, release notes, `logix-core.json`'s `note`): it's meant only for Logix Engine, and its content and layout may change at any time.
- **Boards and FPGAs aren't here:** they're the `logix-catalog` package, built by the `logix-catalog` repo. Examples belong to each frontend.

The shared Open Logix rules (products, principles, wording, license, folder names) live in the website repo's `CLAUDE.md` (`openlogix/openlogix.github.io`); follow them here.

## Rules

**Repo type: dependency** (`.github/logix-repo-metadata.jsonc`; the website repo's `CLAUDE.md`, Repo types, owns the policy): Logix Engine downloads its releases as its remote config names them.

- **Pins change only by pull request.** `pins.json` names each tool's upstream release and files per platform. No job updates it on its own.
- **Upstream, unchanged.** Each tool is its upstream release, unpacked into its own folder (only a single top folder is stripped). Logix builds none of the tools and patches none of them, except that **a tool upstream publishes no build of is built here** from its pinned upstream source (tag and commit), unmodified (the user's rule, 2026-11-13): today only libwdi's `wdi-simple`, in the Windows package only (`docs/design.md`, wdi-simple). Never use a personal repo's build or workflow (the user's rule). **Open Logix's own programs** the engine runs live here too and are built by the workflow (the user's decision, 2026-11-22): today Logix Viewer, `apps/viewer/` (`docs/design.md`, Logix Viewer).
- **How to run each tool lives in the package,** in `logix-core.json` (`run.path`, `run.env`), set from `pins.json`. Logix Engine reads it; don't hard-wire tool folders in the engine.
- **Reproducible.** The same pins give the same bytes: nothing in the package may change from build to build (no dates, run ids or host details). Build details go in the release's `BUILD-INFO.json`, beside the packages.
- **Daily pre-releases** (the Open Logix rule for every pre-release build, website repo's `CLAUDE.md`): the workflow runs on every push to `main`, daily at 00:00 UTC (so breakage from GitHub's runners or upstream shows up even when nothing changes) and manually. It's built on the shared actions (`start-pre-release`, `read-build-info`, `publish-pre-release`, which writes the standard release text, remakes the day's pre-release from scratch and deletes old ones by the shared policy). A pinned pre-release is temporary: an engine release that ships pins a stable release (pre-release unticked), which is never deleted.
- **Required programs:** `scripts/build.py`'s `REQUIRED` lists the programs Logix Engine runs; a build without them fails. Update it when the engine starts or stops running a program.
- **openXC7's yosys** (the user's rule, 2026-11-13): openXC7 is built against one YosysHQ suite release and must run with its yosys. Its package's `BUILD-INFO.json` names that release (`yosys-release-tag`), and `scripts/build.py` (`check_yosys_release`) fails the build unless it's the `oss-cad-suite` release `pins.json` pins. So the two move together, in one pull request.

## Layout

- `pins.json`: the components and their pinned upstream files, per platform.
- `scripts/build.py`: builds one platform's package (`--platform`, `--out`, `--work`, `--package-tag`). Needs Python 3.12+, the `gh` CLI and `requirements.txt` (msgpack).
- `scripts/fpgas.py`: the FPGAs the tools can build, per toolchain, in the tools' terms, written into `logix-core.json` `fpgas` (docs/design.md, The FPGA list). It reads data files only, never runs the tools, so every platform builds on Linux. Logix Engine's tests check its pinned catalog against it. A local run downloads about 2 GB per platform into `_work/`.
- `scripts/build-viewer.sh`: builds Logix Viewer for the platform it runs on, into the `.tgz` `build.py` takes as the `logix-viewer` component's download.
- `scripts/build-wdi-simple.sh`: builds `wdi-simple.exe` from the pinned libwdi source (MSYS2 on Windows), into the zip `build.py` takes as the component's download.
- `.github/workflows/build-pre-release.yaml`: builds `wdi-simple` on a Windows runner (cached by its pin and script), then all platforms on Linux runners, and publishes the pre-release, using the shared actions in `openlogix/.github`.
- `apps/viewer/`: Logix Viewer, Open Logix's own program (not an upstream tool) that shows Logix Engine's pages (waveforms, diagrams) in windows of their own, one app per kind of page (`apps.json`: Logix Waveforms, Logix Diagrams, Logix Viewer), each with its own name and Dock icon; the requirements are the engine's (Logix Viewer). Rust (`cargo build --release` in `apps/viewer/`); `src/protocol.rs` is what engines send it over its local socket, `src/host.js` the `window.logixHost` pages get; `icons/make-icons.py` (on a Mac) makes the committed icon files from the SVGs, variants of Logix Engine's icon; `bundle.py` packs the apps. It's the `logix-viewer` component: `scripts/build-viewer.sh` builds it on each platform's runner (the workflow's `build-viewer` job, cached by its sources), and `scripts/build.py` requires its programs (`REQUIRED_BY_PLATFORM`).
- `docs/design.md`: the design.

## Commits

Short imperative subject, then a body saying what changed and why.
