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

## Security considerations

Readers must treat all input as untrusted. In particular: validate names before
touching the filesystem, bound the output of decoding by raw_len, reject stream
lengths that imply an implausible expansion (the reference reader rejects
inner_len above `(stream_len + 16) * 64`), and check that stored_len does not
exceed the remaining input.
