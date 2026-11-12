<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://www.openlogix.org/brand/readme/core-dark.svg">
  <img alt="logix-core" src="https://www.openlogix.org/brand/readme/core-light.svg" height="96">
</picture>

<br>

[![License: MIT or Apache 2.0](https://img.shields.io/badge/License-MIT%20or%20Apache%202.0-blue)](#license)

<img src="https://www.openlogix.org/brand/readme/macos.svg" height="40" alt="macOS (Apple Silicon)" title="macOS (Apple Silicon)">

[![core-build](https://img.shields.io/github/actions/workflow/status/openlogix/logix-core/build-pre-release.yaml?branch=main&label=core-build)](https://github.com/openlogix/logix-core/actions/workflows/build-pre-release.yaml)

> [!IMPORTANT]
> **Use it through Logix Engine.** logix-core is meant only for [Logix Engine](https://www.openlogix.org/engine/), which downloads it and runs its tools. It isn't for use on its own, and its content and layout may change at any time. To use these tools, use Logix Engine or a frontend built on it, such as [Logix IDE](https://www.openlogix.org/ide/).

`logix-core` is the package of open source FPGA tools that Logix Engine
downloads: one `.tgz` per platform, published here as dated pre-releases.
It isn't an Open Logix product and has no product page.

Inside each package, every tool is its upstream release, unchanged, in a
folder of its own:

| Folder | Upstream | Platforms |
| --- | --- | --- |
| `oss-cad-suite/` | [YosysHQ OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build): yosys, nextpnr, icestorm, Project Trellis, Apycula, Icarus Verilog, Verilator, GTKWave, openFPGALoader and more | macOS (Apple Silicon), Linux x64, Windows x64 |
| `openxc7/` | [openXC7](https://github.com/cavearr/toolchain-openxc7-releases): nextpnr-xilinx and the Project X-Ray tools and database | macOS (Apple Silicon), Linux x64, Windows x64 |
| `verible/` | [CHIPS Alliance Verible](https://github.com/chipsalliance/verible): the Verilog formatter and language server | macOS (Apple Silicon), Linux x64, Windows x64 |
| `elkjs/` | [ELK for JavaScript](https://github.com/kieler/elkjs) (the Eclipse Layout Kernel), which lays out Logix Engine's design diagrams; the engine runs it itself | macOS (Apple Silicon), Linux x64, Windows x64 |

`logix-core.json`, at the package's top, lists each folder's upstream file,
version and SHA-256, how Logix Engine runs it, and the FPGAs the tools can
build, generated from their databases. The boards and FPGAs
are a separate package, built by [Logix Catalog](https://github.com/openlogix/logix-catalog).

**Problems or ideas?** Report them in [Logix Engine's issues](https://github.com/openlogix/logix-engine/issues/new/choose): this repo takes no issues. Security problems: report them privately, as [SECURITY.md](https://github.com/openlogix/.github/blob/main/SECURITY.md) says.

## How it's made

[`pins.json`](pins.json) pins each tool's upstream release; it changes only
by pull request. [`scripts/build.py`](scripts/build.py) downloads the pinned
files, unpacks each into its folder unchanged, writes `logix-core.json` and
packs a reproducible `.tgz`, so the same pins always give the same SHA-256.
The [build workflow](.github/workflows/build-pre-release.yaml) runs it for
each platform and publishes the day's pre-release, on every change and daily; the 5 newest pre-releases are kept. [`docs/design.md`](docs/design.md)
has the reasons and the processes.

## License

The packaging (this repo's files) is licensed under [MIT](LICENSE-MIT) or the
[Apache License 2.0](LICENSE-APACHE), at your option. Each tool in a package
keeps its own upstream license, shipped in its folder. Part of
[Open Logix](https://www.openlogix.org/).
