# `deblockator`

*A platform-agnostic memory allocator designed for the PS Vita*

[![TravisCI](https://img.shields.io/travis/vita-rust/deblockator/master.svg?maxAge=600&style=flat-square)](https://travis-ci.org/vita-rust/deblockator/builds)
[![Codecov](https://img.shields.io/codecov/c/github/vita-rust/deblockator.svg?maxAge=600&style=flat-square)](https://codecov.io/github/vita-rust/deblockator)
[![Source](https://img.shields.io/badge/source-GitHub-303030.svg?maxAge=86400&style=flat-square)](https://github.com/vita-rust/deblockator)
[![CargoMake](https://img.shields.io/badge/built%20with-cargo--make-yellow.svg?maxAge=86400&style=flat-square)](https://sagiegurari.github.io/cargo-make)
[![Changelog](https://img.shields.io/badge/keep%20a-changelog-8A0707.svg?maxAge=86400&style=flat-square)](http://keepachangelog.com/)
[![Crate](https://img.shields.io/crates/v/deblockator.svg?maxAge=86400&style=flat-square)](https://crates.io/crates/deblockator)
[![Documentation](https://img.shields.io/badge/docs-latest-4d76ae.svg?maxAge=86400&style=flat-square)](https://docs.rs/deblockator)


## Introduction

The PS Vita provides a kernel API that allows to allocate memory blocks in the
console RAM. However, the kernel will only ever allocate blocks of 4kB-aligned
memory. While the VitaSDK `newlib` port uses a 32MB sized heap in a single
memory block, it is not the most efficient to do so considering there is a
proper allocator available.

The `deblockator` allocator relies on another allocator to obtain data blocks, and
then uses classic allocation techniques to provide smaller memory chunks within
those blocks.

## Usage

Add this crate to `Cargo.toml`:
```toml
[dependencies.deblockator]
git = "https://github.com/vita-rust/deblockator"
```

### PS Vita

You'll need to have the `armv7-vita-eabihf` target specification in your
`$RUST_TARGET_PATH`. If you don't have it, you can find in its dedicated
[git repository](https://github.com/vita-rust/common).

When compiling for the PS Vita, use the `Vitallocator` from the
[`vitallocator`](https://github.com/vita-rust/vitallocator) crate
and wrap it within the `Deblockator` struct:
```rust
#![feature(global_allocator)]
extern crate deblockator;
extern crate vitallocator;

use deblockator::Deblockator;
use vitallocator::Vitallocator;

#[global_allocator]
static ALLOC: Deblockator = Deblockator::new(Vitallocator::new());
```

Compiling to the PS Vita requires the [`psp2-sys`](https://github.com/vita-rust/psp2-sys) crate.


## Credits

* [**Philipp Oppermann**](https://github.com/phil-opp/) for the
  [Writing an OS in Rust], in particular the [Kernel Heap] section, as well
  as the [`linked_list_allocator`] crate.

[Writing an OS in Rust]: https://os.phil-opp.com/
[Kernel Heap]: https://os.phil-opp.com/kernel-heap/
[`linked_list_allocator`]: https://crates.io/crates/linked_list_allocator

