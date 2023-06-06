<div align="center">

# `retour` (a `detour` Fork)

[![Crates.io][crates-badge]][crates-url]
[![docs.rs][docs-badge]][docs-url]
[![Lcense][license-badge]][license-url]
[![Cargo Check/Tests][actions-badge]][actions-url]
</div>

[crates-badge]: https://img.shields.io/crates/v/retour.svg
[crates-url]: https://crates.io/crates/retour

[docs-badge]: https://docs.rs/retour/badge.svg
[docs-url]: https://docs.rs/retour

[license-badge]: https://img.shields.io/crates/l/retour
[license-url]: ./LICENSE

[actions-badge]: https://github.com/Hpmason/retour-rs/actions/workflows/ci.yml/badge.svg
[actions-url]: https://github.com/Hpmason/retour-rs/actions/workflows/ci.yml

(Fork of original [detour-rs](https://github.com/darfink/detour-rs) 
that works on nightly after nightly-2022-11-07)


This is a cross-platform detour library developed in Rust. Beyond the basic
functionality, this library handles branch redirects, RIP-relative
instructions, hot-patching, NOP-padded functions, and allows the original
function to be called using a trampoline whilst hooked.

This is one of few **cross-platform** detour libraries that exists, and to
maintain this feature, not all desired functionality can be supported due to
lack of cross-platform APIs. Therefore [EIP relocation](#appendix) is not
supported.

**NOTE**: Nightly is currently required for `static_detour!` and is enabled with the `static-detour` feature flag.

## Platforms

This library provides CI for these targets:

- Linux
  * `i686-unknown-linux-gnu`
  * `x86_64-unknown-linux-gnu`
  * `x86_64-unknown-linux-musl`
- Windows
  * `i686-pc-windows-gnu`
  * `i686-pc-windows-msvc`
  * `x86_64-pc-windows-gnu`
  * `x86_64-pc-windows-msvc`
- macOS
  * ~~`i686-apple-darwin`~~
  * `x86_64-apple-darwin`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# `static-detour` feature requires nightly
retour = { version = "0.3", features = ["static-detour"] }
```

## Example

- A static detour (one of *three* different detours):

```rust
use std::error::Error;
use retour::static_detour;

static_detour! {
  static Test: /* extern "X" */ fn(i32) -> i32;
}

fn add5(val: i32) -> i32 {
  val + 5
}

fn add10(val: i32) -> i32 {
  val + 10
}

fn main() -> Result<(), Box<dyn Error>> {
  // Reroute the 'add5' function to 'add10' (can also be a closure)
  unsafe { Test.initialize(add5, add10)? };

  assert_eq!(add5(1), 6);
  assert_eq!(Test.call(1), 6);

  // Hooks must be enabled to take effect
  unsafe { Test.enable()? };

  // The original function is detoured to 'add10'
  assert_eq!(add5(1), 11);

  // The original function can still be invoked using 'call'
  assert_eq!(Test.call(1), 6);

  // It is also possible to change the detour whilst hooked
  Test.set_detour(|val| val - 5);
  assert_eq!(add5(5), 0);

  unsafe { Test.disable()? };

  assert_eq!(add5(1), 6);
  Ok(())
}
```

- A Windows API hooking example is available [here](./examples/messageboxw_detour.rs); build it by running:
```
$ cargo +nightly build --features="static-detour" --example messageboxw_detour
```

- A non-nightly example using GenericDetour can be found [here](./examples/kernel32_detour.rs); build it by running:
```
$ cargo build --example kernel32_detour
```

## Mentions

This is fork of the original [detour-rs][detour-rs] creator 
[darfink][detour-rs-author] that put *so much* work into the original crate.

Part of the library's external user interface was inspired by
[minhook-rs][minhook], created by [Jascha-N][minhook-author], and it contains
derivative code of his work.

## Appendix

- *EIP relocation*

  *Should be performed whenever a function's prolog instructions
  are being executed, simultaneously as the function itself is being
  detoured. This is done by halting all affected threads, copying the affected
  instructions and appending a `JMP` to return to the function. This is
  barely ever an issue, and never in single-threaded environments, but YMMV.*

- *NOP-padding*
  ```c
  int function() { return 0; }
  // xor eax, eax
  // ret
  // nop
  // nop
  // ...
  ```
  *Functions such as this one, lacking a hot-patching area, and too small to
  be hooked with a 5-byte `jmp`, are supported thanks to the detection of
  code padding (`NOP/INT3` instructions). Therefore the required amount of
  trailing `NOP` instructions will be replaced, to make room for the detour.*

<!-- Links -->
[minhook-author]: https://github.com/Jascha-N
[minhook]: https://github.com/Jascha-N/minhook-rs/
[detour-rs]: https://github.com/darfink/detour-rs
[detour-rs-author]: https://github.com/darfink
