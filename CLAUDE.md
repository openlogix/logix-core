# CLAUDE.md

Guidance for Claude Code (claude.ai/code) and other AI agents working in this repo.

## What this is

`logix-core` builds the package of open source FPGA tools that Logix Engine downloads and runs: one reproducible `.tgz` per platform (`darwin-arm64`, `linux-x86-64`, `windows-x86-64`), published as dated GitHub pre-releases. `docs/design.md` has the decisions, the reasons and the processes.

- **Not an Open Logix product:** no product page, no docs site, no `logix-` product rules beyond the shared ones. It's infrastructure for Logix Engine.
- **Not for use on its own.** Say so wherever people meet it (README, release notes, `logix-core.json`'s `note`): it's meant only for Logix Engine, and its content and layout may change at any time.
- **Boards and FPGAs aren't here:** they're the `logix-catalog` package, built by the `logix-catalog` repo. Examples belong to each frontend.

The shared Open Logix rules (products, principles, wording, license, folder names) live in the website repo's `CLAUDE.md` (`openlogix/openlogix.github.io`); follow them here.

## Rules

- **Pins change only by pull request.** `pins.json` names each tool's upstream release and files per platform. No job updates it on its own.
- **Upstream, unchanged.** Each tool is its upstream release, unpacked into its own folder (only a single top folder is stripped). Logix builds none of the tools and patches none of them.
- **How to run each tool lives in the package,** in `logix-core.json` (`run.path`, `run.env`), set from `pins.json`. Logix Engine reads it; don't hard-wire tool folders in the engine.
- **Reproducible.** The same pins give the same bytes: nothing in the package may change from build to build (no dates, run ids or host details). Build details go in the release's `BUILD-INFO.json`, beside the packages.
- **Old releases stay:** engine releases pin them by tag, so the workflow never deletes them.
- **Required programs:** `scripts/build.py`'s `REQUIRED` lists the programs Logix Engine runs; a build without them fails. Update it when the engine starts or stops running a program.

## Layout

- `pins.json`: the components and their pinned upstream files, per platform.
- `scripts/build.py`: builds one platform's package (`--platform`, `--out`, `--work`, `--package-tag`). Needs Python 3.12+ and the `gh` CLI. A local run downloads about 2 GB per platform into `_work/`.
- `.github/workflows/build-pre-release.yaml`: builds all platforms on Linux runners and publishes the pre-release, using the shared actions in `openlogix/.github`.
- `docs/design.md`: the design.

## Commits

Short imperative subject, then a body saying what changed and why.
