# STOW

An experimental archive format and a library to read and write it. The core is
written in Rust using only the standard library and is exposed through a C ABI.

Status: prototype. The format is version 1 draft and may still change.

## Idea

It started as a note written down in bed: take the data `A 8 4 R 5`, call
`A 8 4` by the name 3 and `R 5` by the name 6, then call `3 6 8` by the name 1.
Replace repeated patterns with shorter names, again and again, until what is
left is a small table of rules and a short sequence. This is grammar
compression, an old idea. STOW is a hands-on attempt to see how far it goes
when kept as simple as possible, plus a little arithmetic for numbers.

- **Fold.** Repeatedly replace the most frequent pair of symbols with a new
  symbol, Re-Pair style. Rules that are used only once are merged into their
  parent, and the result is written with an adaptive range coder.
- **Math.** Reversible transforms applied before folding: delta, xor and a
  second-order delta that predicts along a straight line. Division by a common
  factor was tried and gave nothing, so it is not included.

The encoder tries every codec and transform per entry and keeps the smallest.
Large entries are sampled first, so data that does not compress is stored
without a full search.

## Results

On a small mixed corpus (license texts, Python sources, binaries, repeated and
random data) STOW ends up at about 12% of the original size, against 19% for
`gzip -9` and 10% for `xz -9e`.

It does best on highly repetitive data and on smooth or monotone numbers, where
it beats `xz`. On real text, source code and executables it beats `gzip` in some
cases but stays behind `xz` by roughly 15 to 30 percent: folding has no way to
point back at distant repeats the way LZ does, and naming each rule costs about
as many bits as the pattern saves. Random data is stored as is.

`tools/bench.py` builds a corpus and prints sizes next to `gzip` and `xz`.

## Layout

- `stow-core` - container, codecs, transforms, checksum. A command line harness
  is in `stow-core/examples/stow.rs`.
- `stow-ffi` - C ABI. Header: `stow-ffi/include/stow.h`.
- `specification` - format specification.
- `tests/c` - C program exercising the ABI.
- `tools` - benchmark script.

## License

This is an experiment, and anyone is free to use it under the licenses below.

Reference implementation: BSD-3-Clause. Specification: CC BY 4.0.
Copyright (c) 2026 TamKungZ_.
