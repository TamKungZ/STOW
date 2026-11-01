# STOW Archive Format Specification

Version 1 (draft)

Copyright (c) 2026 TamKungZ_. This specification is licensed under CC BY 4.0
(https://creativecommons.org/licenses/by/4.0/).

## Conventions

All fixed-size integers are unsigned and little-endian. A varint is an unsigned
LEB128 integer of at most 64 bits. Conventional file extension: `.sto`.

## Container

```
header
entry[entry_count]
```

No data may follow the last entry.

Header, 10 bytes:

| Offset | Size | Field                                   |
| ------ | ---- | --------------------------------------- |
| 0      | 4    | magic, ASCII `STOW`                     |
| 4      | 1    | version, `1`                            |
| 5      | 1    | flags, must be `0`                      |
| 6      | 4    | entry_count                             |

Entry:

| Size       | Field                                              |
| ---------- | -------------------------------------------------- |
| 2          | name_len                                           |
| name_len   | name, UTF-8                                        |
| 1          | codec id                                           |
| 8          | raw_len, size of the original data                 |
| 8          | stored_len, size of the payload                    |
| 4          | crc32 of the original data (IEEE 802.3, as in zlib)|
| stored_len | payload                                            |

A reader must reject an archive with an unknown version or non-zero flags,
and must verify crc32 after decoding an entry.

## Entry names

A name is a relative path using `/` as separator. It must not be empty, must not
start with `/`, and must not contain `NUL`, `\`, or `:`. No segment may be empty,
`.`, or `..`. Names must be unique within an archive. Only files are stored;
directories are implied by names.

## Codecs

| Id | Name    | Payload                                    |
| -- | ------- | ------------------------------------------ |
| 0  | store   | the original bytes, stored_len = raw_len   |
| 1  | fold    | grammar, see below                         |
| 2  | fold+rc | varint inner_len, then a range coder stream whose decoded bytes are a codec 1 payload |
| 3  | fold+ac | grammar coded directly with the adaptive range coder, see below |
| 4  | xform   | u8 transform id, u8 inner codec id, then the inner payload |

An encoder may choose any codec per entry. The reference encoder picks the
smallest payload. A reader must reject unknown codec ids.

### fold

The payload is:

```
varint rule_count
varint seq_count
rule[rule_count]   each: varint left, varint right
varint[seq_count]  the symbol sequence
```

Symbols 0 to 255 are literal bytes. Symbol `256 + i` is rule `i`, which expands
to the expansion of `left` followed by the expansion of `right`. The children of
rule `i` must be symbols below `256 + i`, so rules never refer forward. Every
symbol in the sequence must be below `256 + rule_count`.

The original data is the concatenation of the expansions of the sequence
symbols. Its length must equal raw_len exactly, and the payload must contain no
trailing bytes.

### rc

A binary adaptive range coder in the LZMA style, modelling each byte as an
8-level bit tree (nodes 1 to 255, most significant bit first) with a single
order-0 context.

- Probabilities are 11-bit, initial value 1024, probability of a 0 bit.
- Update after a 0 bit: `p += (2048 - p) >> 5`. After a 1 bit: `p -= p >> 5`.
- 32-bit range, initial `0xFFFFFFFF`, normalised when below `2^24`.
- Bound: `(range >> 11) * p`. A 0 bit keeps the lower part.
- The encoder flushes 5 bytes. The stream begins with a byte that is always 0,
  and the decoder reads 5 bytes to initialise its code value.
- The decoder produces exactly inner_len bytes and must consume exactly the
  whole stream.

### fold+ac

The payload is:

```
u8     mode          bit 0 distance, bit 1 ranked, bit 2 n-ary, other bits zero
varint rule_count
varint seq_count
rc stream
```

The rc stream uses the range coder described under rc and must be consumed
exactly. It carries `rule_count` rules followed by `seq_count` sequence symbols,
with the grammar semantics of fold. Rule `i` is coded as its first child, then its
remaining children. The three streams of symbols (first children, other children,
sequence) each use their own model, with all probabilities starting at 1024.

With bit 2 set a rule has 2 to 256 children instead of exactly 2, and its child
count is coded before its children: `c - 1` is coded like the position `v` below
with bound 255, using `len[p]` where `p` is the bit length of the previous rule's
`c - 1` capped at 9 (0 for the first rule). The expansion of a rule is the
concatenation of the expansions of its children, which must all be symbols below
`256 + i`.

With bit 0 set a child that is a rule `c >= 256` of rule `i` is coded as the value
`255 + d` with `d = 256 + i - c`, so `d` is between 1 and `i`. Otherwise it is
coded as `c`. Literals are coded as their byte value in both modes.

Coding a value `x` under context `ctx`:

1. One flag bit with probability `flag[ctx]`: 0 for `x < 256`, 1 otherwise.
2. A literal is coded as an 8-level bit tree, most significant bit first, with
   probabilities `literal[ctx][node]`.
3. Otherwise let `v = x - 255` and `n` its bit length (1 to 25). `n` is coded as
   a 5-level bit tree with `len[ctx][node]`; a decoded `n` of 0 or above 25 is
   invalid. Then the `n - 1` bits of `v` below its leading one are coded most
   significant first. The first three use a bit tree `top[n][node]`, the rest use
   `low[n][bit position]`.

With bit 1 set the sequence symbols are not coded as above. Let `first(x)` and
`last(x)` be the first and last byte of the expansion of symbol `x`. For each byte
`f` the group `f` lists, in increasing symbol order, every symbol `x` with
`first(x) = f` (the literal `f` and then the rules). A sequence symbol is coded as
`first(x)` followed by its 1-based position in that group, which is omitted when
the group has one member. `first(x)` is an 8-level bit tree with probabilities
selected by `last` of the previous sequence symbol (0 at the start). The position
`v` is coded like the length and mantissa of the non-ranked scheme, using
`len[n']` with `n'` the bit length of the group size, and the same `top` and `low`
trees. A decoded bit length above that of the group size, or `v` above the group
size, is invalid.

The context is `START = 272` for the first value of a stream. Afterwards it is
the previous value of the same stream `y` in that stream's coded form: `y` itself
when `y < 256`, otherwise `256 + min(m - 1, 15)` where `m` is the bit length of
`y - 255`.

A decoder must reject a mode above 7, a child count above 256, a distance larger than `i`, a
symbol at or above `256 + rule_count`, and a stream that overruns or leaves
bytes unused. The reference decoder also rejects `rule_count` above 2^24 and
counts above `(stream_len + 16) * 64`.

### xform

A transform is a reversible, length-preserving rewrite of the entry data applied
before the inner codec. The inner codec decodes to raw_len bytes, then the
inverse transform is applied. The inner codec id must not be 4.

| Id | Name   | Forward, per element                                |
| -- | ------ | --------------------------------------------------- |
| 1  | delta1 | `out = cur - prev`, 1-byte elements                 |
| 2  | delta2 | same, 2-byte little-endian elements                 |
| 3  | delta4 | same, 4-byte little-endian elements                 |
| 4  | xor1   | `out = cur ^ prev`, 1-byte elements                 |
| 5  | xor2   | same, 2-byte little-endian elements                 |
| 6  | xor4   | same, 4-byte little-endian elements                 |
| 7  | accel1 | `out = cur - 2 * prev + prev2`, 1-byte elements     |
| 8  | accel2 | same, 2-byte little-endian elements                 |
| 9  | accel4 | same, 4-byte little-endian elements                 |

`prev` and `prev2` are the previous two elements of the original data, 0 where
they do not exist.
Subtraction wraps within the element width. Trailing bytes that do not fill an
element are left unchanged. A reader must reject an unknown transform id.

Codecs 3 and 4 were added without changing how existing archives decode, so the
version stays 1.

## Security considerations

Readers must treat all input as untrusted. In particular: validate names before
touching the filesystem, bound the output of decoding by raw_len, reject stream
lengths that imply an implausible expansion (the reference reader rejects
inner_len above `(stream_len + 16) * 64`), and check that stored_len does not
exceed the remaining input.
