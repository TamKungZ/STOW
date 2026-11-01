#!/usr/bin/env python3
import argparse
import glob
import gzip
import hashlib
import lzma
import math
import os
import random
import struct
import subprocess
import sys
import tempfile
import time

CORPUS = [
    "licenses.txt",
    "pystdlib.txt",
    "binary.bin",
    "a84r5.txt",
    "words.txt",
    "random.bin",
    "mono_u32.bin",
    "sine_s16.bin",
    "numbers.csv",
]


def read_all(paths, limit=None):
    out = bytearray()
    for p in paths:
        try:
            with open(p, "rb") as f:
                out += f.read()
        except OSError:
            continue
        if limit and len(out) >= limit:
            break
    return bytes(out[:limit] if limit else out)


def build_corpus(dest):
    os.makedirs(dest, exist_ok=True)
    docs = sorted(glob.glob("/usr/share/common-licenses/*")) + sorted(glob.glob("/usr/share/doc/*/copyright"))
    seen, unique = set(), []
    for p in docs:
        real = os.path.realpath(p)
        if real not in seen and os.path.isfile(real):
            seen.add(real)
            unique.append(real)
    files = {}
    files["licenses.txt"] = read_all(unique)
    pyfiles = sorted(glob.glob("/usr/lib/python3*/*.py"))
    files["pystdlib.txt"] = read_all(pyfiles, 3 << 20)
    files["binary.bin"] = read_all(["/usr/bin/python3.10"], 2 << 20)
    files["a84r5.txt"] = b"A84R5" * 60000
    words = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india"]
    r = random.Random(1)
    files["words.txt"] = " ".join(r.choice(words) for _ in range(80000)).encode()
    r = random.Random(2)
    files["random.bin"] = bytes(r.getrandbits(8) for _ in range(300000))
    r = random.Random(3)
    v, vals = 1000, []
    for _ in range(100000):
        v += r.randint(1, 40)
        vals.append(v)
    files["mono_u32.bin"] = struct.pack("<%dI" % len(vals), *vals)
    r = random.Random(4)
    files["sine_s16.bin"] = struct.pack(
        "<200000h",
        *[int(20000 * math.sin(i * 0.013) + r.randint(-8, 8)) for i in range(200000)],
    )
    r = random.Random(6)
    files["scaled_s16.bin"] = struct.pack(
        "<200000h",
        *[64 * int(300 * math.sin(i * 0.013) + r.randint(-1, 1)) for i in range(200000)],
    )
    r = random.Random(7)
    price, prices = 100000, []
    for _ in range(100000):
        price += 5 * r.randint(-4, 4)
        prices.append(price)
    files["prices_u32.bin"] = struct.pack("<%dI" % len(prices), *prices)
    r = random.Random(5)
    lines, t, x = ["id,time,value,count"], 1700000000, 20.0
    for i in range(20000):
        t += r.randint(1, 5)
        x += r.uniform(-0.5, 0.5)
        lines.append("%d,%d,%.2f,%d" % (i, t, x, r.randint(0, 99)))
    files["numbers.csv"] = ("\n".join(lines) + "\n").encode()
    for name, data in files.items():
        with open(os.path.join(dest, name), "wb") as f:
            f.write(data)
        print("%-14s %9d  %s" % (name, len(data), hashlib.sha256(data).hexdigest()[:12]))


def run_stow(exe, path, tmp):
    sto = os.path.join(tmp, "x.sto")
    out = os.path.join(tmp, "out")
    t0 = time.perf_counter()
    subprocess.run([exe, "create", sto, path], check=True)
    enc = time.perf_counter() - t0
    t0 = time.perf_counter()
    subprocess.run([exe, "extract", sto, out], check=True)
    dec = time.perf_counter() - t0
    with open(path, "rb") as a, open(os.path.join(out, os.path.basename(path)), "rb") as b:
        if a.read() != b.read():
            raise SystemExit("roundtrip mismatch: " + path)
    size = os.path.getsize(sto)
    listing = subprocess.run([exe, "list", sto], check=True, capture_output=True, text=True).stdout.split()
    return size, listing[2], enc, dec


def run(corpus, exe):
    print("%-14s %9s %9s %-9s %9s %9s %6s %6s" % ("file", "raw", "stow", "codec", "gzip-9", "xz-9e", "enc_s", "dec_s"))
    totals = [0, 0, 0, 0]
    with tempfile.TemporaryDirectory() as tmp:
        names = [n for n in sorted(os.listdir(corpus)) if n.endswith(('.txt', '.bin', '.csv')) and n != 'SHA256SUMS.txt']
        for name in names:
            path = os.path.join(corpus, name)
            data = open(path, "rb").read()
            size, codec, enc, dec = run_stow(exe, path, tmp)
            gz = len(gzip.compress(data, 9, mtime=0))
            xz = len(lzma.compress(data, format=lzma.FORMAT_XZ, check=lzma.CHECK_CRC64,
                                   filters=[{"id": lzma.FILTER_LZMA2, "preset": 9 | lzma.PRESET_EXTREME}]))
            print("%-14s %9d %9d %-9s %9d %9d %6.2f %6.2f" % (name, len(data), size, codec, gz, xz, enc, dec))
            for i, v in enumerate((len(data), size, gz, xz)):
                totals[i] += v
    print("%-14s %9d %9d %-9s %9d %9d" % ("total", totals[0], totals[1], "", totals[2], totals[3]))


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("corpus")
    c.add_argument("dir")
    r = sub.add_parser("run")
    r.add_argument("dir")
    r.add_argument("exe")
    args = ap.parse_args()
    if args.cmd == "corpus":
        build_corpus(args.dir)
    else:
        run(args.dir, os.path.abspath(args.exe))


if __name__ == "__main__":
    sys.exit(main())
