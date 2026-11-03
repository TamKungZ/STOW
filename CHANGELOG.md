# Changelog

## 0.2.0

- Per-file screening for entries larger than 64 KiB. The encoder samples the
  head, middle and tail of each file, stores it as is when no codec gains at
  least 2% on the sample, and otherwise picks the transform from the sample
  and skips the plain codecs when the transform clearly wins. Already
  compressed files (jpg, png, mp3, zip) are about 10 times faster to add and
  came out the same size on the files tried.
- Tests for several large files in one archive, identical content under two
  names, and entries that are unaffected by their neighbours.
- No change to the archive format, which stays at version 1.

## 0.1.0

- First prototype: container, fold codecs, transforms, C ABI.
