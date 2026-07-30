# RK

[![Deploy](https://github.com/NOPLAB/rk/actions/workflows/deploy-pages.yml/badge.svg)](https://github.com/NOPLAB/rk/actions/workflows/deploy-pages.yml)
[![Release](https://github.com/NOPLAB/rk/actions/workflows/release.yml/badge.svg)](https://github.com/NOPLAB/rk/actions/workflows/release.yml)

A 3D CAD editor built with Rust.

## Documentation

[noplab.github.io/rk](https://noplab.github.io/rk/) — source in `apps/docs`.

## Install

Download from [Releases](https://github.com/NOPLAB/rk/releases) (Windows, macOS, Linux).

## Build

Needs Rust, Node.js, CMake and a C++ toolchain — the CAD kernel is
OpenCASCADE, compiled from source, so the first build takes a while.

```bash
cd apps/desktop
npm install
npm run tauri dev
```

## License

MIT
