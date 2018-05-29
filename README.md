# `vitalloc` - a memory allocator for the PS Vita

[![TravisCI](https://img.shields.io/travis/vita-rust/vitalloc/master.svg?maxAge=600&style=flat-square)](https://travis-ci.org/vita-rust/vitalloc/builds)
[![Source](https://img.shields.io/badge/source-GitHub-303030.svg?maxAge=86400&style=flat-square)](https://github.com/vita-rust/vitalloc)
[![CargoMake](https://img.shields.io/badge/built%20with-cargo--make-yellow.svg?maxAge=86400&style=flat-square)](https://sagiegurari.github.io/cargo-make)
[![Changelog](https://img.shields.io/badge/keep%20a-changelog-8A0707.svg?maxAge=86400&style=flat-square)](http://keepachangelog.com/)
<!-- [![Codecov](https://img.shields.io/codecov/c/github/althonos/packageurl-rs.svg?maxAge=600&style=flat-square)](https://codecov.io/github/althonos/packageurl-rs) -->
<!-- [![Crate](https://img.shields.io/crates/v/packageurl.svg?maxAge=86400&style=flat-square)](https://crates.io/crates/packageurl) -->
<!-- [![Documentation](https://img.shields.io/badge/docs-latest-4d76ae.svg?maxAge=86400&style=flat-square)](https://docs.rs/packageurl) -->


## Introduction

The PS Vita provides a kernel API that allows to allocate memory blocks in the
console RAM. However, the kernel will only ever allocate blocks of 4kB-aligned
memory. While the VitaSDK `newlib` port uses a 32MB sized heap in a single
memory block, it is not the most efficient to do so considering there is a
proper allocator available.

The `vitalloc` allocator relies on another allocator to obtain data blocks, and
then uses classic allocation techniques to provide smaller memory chunks within
those blocks.

## Usage

Add this crate to `Cargo.toml`:
```toml
[dependencies.vitalloc]
git = "https://github.com/vita-rust/vitalloc"
```

### PS Vita

You'll need to have the `armv7-vita-eabihf` target specification in your
`$RUST_TARGET_PATH`. If you don't have it, you can find in its dedicated
[git repository](https://github.com/vita-rust/common).

When compiling for the PS Vita, use the included `KernelAllocator` and wrap it
within the `Allocator` struct:
```rust
#![feature(global_allocator)]
extern crate vitalloc;

#[global_allocator]
static ALLOC: vitalloc::Allocator = vitalloc::Vitalloc::new(vitalloc::KernelAllocator::new());
```

The `Allocator` will use the kernel mutexes as a global lock. Compiling to the
PS Vita requires the [`psp2-sys`](https://github.com/vita-rust/psp2-sys) crate.


## Credits

* [**VitaSDK team**](http://vitasdk.org/) for the `arm-vita-eabi` toolchain, `psp2` headers, ...
* [**Team Molecule**](http://henkaku.xyz/) for the `Henkaku` hard work.


## Disclaimer

*`vitalloc` is not affiliated, sponsored, or otherwise endorsed by Sony
Interactive Entertainment, LLC. PlayStation and PS Vita are trademarks or
registered trademarks of Sony Interactive Entertainment, LLC. This software is
provided "as is" without warranty of any kind under the MIT License.*
