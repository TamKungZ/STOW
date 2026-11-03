# Test report

Results of compressing `examplefiles` and `lab/stow-corpus` with the current working tree of STOW (version 0.2.0 plus the unreleased `wav` specialist and the speed changes described in the notes below). Every archive was created with `stow create`, extracted with `stow extract`, and every extracted file was compared by SHA-256 with its source. The baselines are Python's `gzip` at level 9 and `lzma` at preset 9 with the extreme flag, applied to each file on its own.

52 archives were checked and all of them extracted to files identical to their sources.

Sizes are in bytes. Times are wall clock on one machine (12 logical cores, encoder limited to 4 threads) and are only useful for comparing rows with each other.

## Single files: examplefiles

One archive per file.

| File | Raw | gzip -9 | xz -9e | STOW | STOW % | Codec | Encode s | Decode s | Round trip |
|---|--:|--:|--:|--:|--:|---|--:|--:|:-:|
| README.md | 47 | 64 | 104 | 89 | 189.4% | store | 0.0 | 0.0 | yes |
| file-example_PDF_1MB.pdf | 1,042,157 | 968,290 | 951,452 | 1,037,982 | 99.6% | fold+ac | 4.7 | 0.1 | yes |
| file-sample_150kB.pdf | 142,786 | 136,952 | 137,228 | 142,840 | 100.0% | store | 2.2 | 0.0 | yes |
| file_example_AVI_1920_2_3MG.avi | 2,279,794 | 1,694,132 | 1,699,756 | 1,788,430 | 78.4% | fold+ac | 5.8 | 0.1 | yes |
| file_example_AVI_480_750kB.avi | 742,478 | 151,139 | 145,032 | 155,558 | 21.0% | fold+ac | 1.3 | 0.0 | yes |
| file_example_JPG_100kB.jpg | 102,117 | 100,387 | 100,340 | 102,176 | 100.1% | store | 2.3 | 0.0 | yes |
| file_example_JPG_2500kB.jpg | 2,504,642 | 2,445,623 | 2,436,668 | 2,504,702 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_MOV_1920_2_2MB.mov | 2,247,200 | 1,692,469 | 1,697,904 | 1,786,092 | 79.5% | fold+ac | 6.0 | 0.1 | yes |
| file_example_MOV_480_700kB.mov | 709,764 | 150,640 | 142,836 | 154,518 | 21.8% | fold+ac | 1.3 | 0.0 | yes |
| file_example_MP3_5MG.mp3 | 5,319,693 | 5,087,620 | 5,060,376 | 5,319,750 | 100.0% | store | 2.2 | 0.0 | yes |
| file_example_MP3_700KB.mp3 | 733,645 | 722,690 | 721,484 | 733,704 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_MP4_1920_18MG.mp4 | 17,839,845 | 16,863,694 | 16,874,296 | 17,839,908 | 100.0% | store | 4.4 | 0.1 | yes |
| file_example_MP4_480_1_5MG.mp4 | 1,570,024 | 1,130,470 | 1,127,444 | 1,182,332 | 75.3% | fold+ac | 4.1 | 0.1 | yes |
| file_example_MP4_640_3MG.mp4 | 3,114,374 | 2,677,686 | 2,692,208 | 2,815,094 | 90.4% | fold+ac | 8.3 | 0.2 | yes |
| file_example_OGG_1920_13_3mg.ogg | 13,284,166 | 13,085,349 | 13,043,428 | 13,284,231 | 100.0% | store | 2.3 | 0.1 | yes |
| file_example_OGG_480_1_7mg.ogg | 1,734,919 | 1,723,035 | 1,720,324 | 1,734,982 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_OOG_1MG.ogg | 1,032,948 | 1,018,983 | 1,014,772 | 1,033,005 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_OOG_5MG.ogg | 5,425,517 | 5,403,990 | 5,409,220 | 5,425,574 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_PNG_3MB.png | 2,964,729 | 2,965,652 | 2,964,940 | 2,964,786 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_PNG_500kB.png | 512,596 | 512,774 | 512,680 | 512,655 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_WAV_10MG.wav | 10,544,134 | 9,538,478 | 7,959,928 | 3,663,263 | 34.7% | wav | 6.6 | 0.4 | yes |
| file_example_WAV_1MG.wav | 1,048,558 | 824,667 | 581,620 | 340,273 | 32.5% | wav | 0.5 | 0.0 | yes |
| file_example_WEBM_1920_3_7MB.webm | 3,667,214 | 3,643,412 | 3,650,480 | 3,667,280 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_WEBM_480_900KB.webm | 901,185 | 894,068 | 895,076 | 901,250 | 100.0% | store | 2.3 | 0.0 | yes |
| file_example_WMV_1920_9_3MB.wmv | 9,338,829 | 4,566,477 | 4,119,492 | 4,258,267 | 45.6% | fold+ac | 17.1 | 0.4 | yes |
| file_example_WMV_480_1_2MB.wmv | 1,175,629 | 386,987 | 338,760 | 368,180 | 31.3% | fold+ac | 2.1 | 0.1 | yes |
| zip_9MB.zip | 9,663,802 | 9,630,725 | 9,620,264 | 9,663,846 | 100.0% | store | 2.3 | 0.0 | yes |
| **Total** | 99,642,792 | 88,016,453 | 85,618,112 | 83,380,767 | 83.7% | | 94.0 | 2.0 | |

## Single files: stow-corpus

| File | Raw | gzip -9 | xz -9e | STOW | STOW % | Codec | Encode s | Decode s | Round trip |
|---|--:|--:|--:|--:|--:|---|--:|--:|:-:|
| SHA256SUMS.txt | 535 | 354 | 456 | 481 | 89.9% | xform | 0.1 | 0.0 | yes |
| aaaa.txt | 300,000 | 479 | 184 | 82 | 0.0% | fold+ac | 0.4 | 0.0 | yes |
| gcc.bin | 1,023,032 | 367,718 | 305,456 | 393,890 | 38.5% | fold+ac | 2.7 | 0.0 | yes |
| licenses.txt | 10,483,966 | 1,643,672 | 463,928 | 598,290 | 5.7% | fold+ac | 17.4 | 0.2 | yes |
| python_src.txt | 1,655,256 | 382,594 | 287,544 | 331,086 | 20.0% | fold+ac | 3.4 | 0.1 | yes |
| random.bin | 300,000 | 300,113 | 300,072 | 300,043 | 100.0% | store | 2.3 | 0.0 | yes |
| words.txt | 453,305 | 52,074 | 45,300 | 36,092 | 8.0% | fold+ac | 0.9 | 0.0 | yes |
| xz.bin | 89,008 | 36,020 | 31,220 | 39,126 | 44.0% | fold+ac | 1.6 | 0.0 | yes |
| **Total** | 14,305,102 | 2,783,024 | 1,434,160 | 1,699,090 | 11.9% | | 28.8 | 0.3 | |

## Grouped archives

Directories added as a whole, compared with the sum of the single-file archives of the same files. Each entry is encoded on its own, so grouping only saves the per-archive header, not compression.

| Archive | Entries | Raw | Sum of single archives | Grouped archive | Grouped % | Encode s | Decode s | Round trip |
|---|--:|--:|--:|--:|--:|--:|--:|:-:|
| examplefiles (directory) | 27 | 99,642,792 | 83,380,767 | 83,380,858 | 83.7% | 30.0 | 1.7 | yes |
| stow-corpus (directory) | 8 | 14,305,102 | 1,699,090 | 1,699,116 | 11.9% | 17.2 | 0.3 | yes |
| both directories | 35 | 113,947,894 | 85,079,857 | 85,079,964 | 74.7% | 38.3 | 2.1 | yes |
| both directories, reversed argument order | 35 | 113,947,894 | 85,079,857 | 85,079,964 | 74.7% | 38.4 | 1.9 | yes |

## Same content under two names

Two directories holding the same WAV and text file.

| Entry | Raw | Stored | Codec |
|---|--:|--:|---|
| dup/one/file_example_WAV_1MG.wav | 1,048,558 | 340,216 | wav |
| dup/one/words.txt | 453,305 | 36,050 | fold+ac |
| dup/two/file_example_WAV_1MG.wav | 1,048,558 | 340,216 | wav |
| dup/two/words.txt | 453,305 | 36,050 | fold+ac |

Both copies are stored in full; there is no deduplication between entries.

## Edge cases

Each file as its own archive, plus all of them together.

| Case | Raw | Archive | Entry codec | Round trip |
|---|--:|--:|---|:-:|
| a/b/c/deep.txt (nested directories) | 5,000 | 81 | fold+ac | yes |
| empty.bin (empty file) | 0 | 42 | store | yes |
| jpg_named_txt.txt (JPEG named .txt) | 102,117 | 102,167 | store | yes |
| one.bin (1 byte) | 1 | 41 | store | yes |
| two.bin (2 bytes) | 2 | 42 | store | yes |
| wav_as_bin.bin (first 300,000 bytes of a WAV, named .bin) | 300,000 | 105,941 | wav | yes |
| wav_cut_mid.wav (WAV cut in the middle of the samples) | 524,280 | 179,286 | wav | yes |
| wav_trailer.wav (WAV with bytes after the data chunk) | 1,048,571 | 340,277 | wav | yes |
| wav_truncated_header.wav (WAV cut inside the header) | 30 | 87 | store | yes |
| zeros_70k.bin (70,000 zero bytes) | 70,000 | 80 | fold+ac | yes |
| ไทย.txt (non-ASCII file name) | 11,100 | 101 | fold+ac | yes |
| all 11 edge files in one directory archive | 2,061,101 | 728,106 | fold+ac, store, wav | yes |

## Damaged archives

A copy of the `stow-corpus` archive with one byte flipped in the middle: rejected, `error: stow-corpus/licenses.txt: archive is corrupt`.
The same archive cut in half: rejected, `archive is truncated`.

## Notes

- Compression is per entry, chosen from store, fold, fold with range coder, fold with arithmetic coder, delta/xor/accel transforms, and the `wav` specialist, whichever is smallest. A specialist result is verified by decoding it before it is used.
- jpg, png, mp3, ogg, webm and zip are stored unchanged (the stored size is the raw size plus the entry header), as are the small PDF and the large MP4 (the 1 MB PDF gains 0.4%). STOW has no format-specific model for them. Only WAV has one. AVI, MOV, WMV and the smaller MP4 files contain redundancy that fold finds (21% to 90% of the original), but `xz -9e` is smaller than STOW on all of them.
- Entries larger than 64 KiB are screened on eight sampled windows; entries larger than 4 MiB are also probed on a 1 MiB slice from the middle and stored if it gains less than 5%. This is a speed trade: a large MP4 that could shrink by about 2% is stored in a few seconds instead of taking about 47 seconds. A file whose sampled parts are incompressible but whose other parts are not will be stored.
- Against `xz -9e`, STOW is clearly smaller only on the two WAV files, `words.txt` and `aaaa.txt`. On the rest of the corpus (text, source, executables) and on the video containers `xz` is smaller. On files neither can compress (png, random data) the two are within a few dozen bytes of each other. STOW's total over `examplefiles` is below `xz` only because of the WAV files.
- An archive costs a 10 byte header plus 23 bytes and the name per entry, which is why tiny and incompressible files come out slightly larger than the source.
- Times include the process start and file I/O of the `stow` example binary, and decode time includes writing the files out.
