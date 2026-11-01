# STOW

An experimental archive format and a library to read and write it. The core is
written in Rust using only the standard library and is exposed through a C ABI.

Status: prototype. The format is version 1 draft and may still change.

## Layout

- `stow-core` - container, codecs, checksum. A command line harness is in
  `stow-core/examples/stow.rs`.
- `stow-ffi` - C ABI. Builds `stow.dll`, `libstow.so` or `libstow.dylib`, plus a
  static library. Header: `stow-ffi/include/stow.h`.
- `specification` - format specification.
- `tests/c` - C program exercising the ABI.

## Build and test

```
cargo build --release
cargo test
```

Harness:

```
cargo run --release --example stow -p stow-core -- create out.sto some_dir
cargo run --release --example stow -p stow-core -- list out.sto
cargo run --release --example stow -p stow-core -- extract out.sto output_dir
cargo run --release --example stow -p stow-core -- stats file...
```

C test, Linux:

```
gcc -Istow-ffi/include tests/c/roundtrip.c target/release/libstow.a -lpthread -ldl -lm -o roundtrip
```

## License

Reference implementation: BSD-3-Clause. Specification: CC BY 4.0.
Copyright (c) 2026 TamKungZ_.
