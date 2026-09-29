#!/usr/bin/env python3
"""Decode an 8-bit, non-interlaced PNG (RGB or RGBA) to raw RGBA8 with the stdlib only.
Usage: png_to_rgba.py <in.png> <out.rgba>  -> prints JSON {width, height, png_sha256, rgba_sha256}."""
import hashlib, json, struct, sys, zlib

src, dst = sys.argv[1], sys.argv[2]
data = open(src, "rb").read()
assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
pos, idat, ihdr = 8, b"", None
while pos < len(data):
    (length,) = struct.unpack(">I", data[pos:pos + 4])
    kind = data[pos + 4:pos + 8]
    body = data[pos + 8:pos + 8 + length]
    if kind == b"IHDR":
        ihdr = struct.unpack(">IIBBBBB", body)
    elif kind == b"IDAT":
        idat += body
    pos += 12 + length
width, height, depth, color, _, _, interlace = ihdr
assert depth == 8 and interlace == 0 and color in (2, 6), f"unsupported PNG: depth={depth} color={color} interlace={interlace}"
channels = 3 if color == 2 else 4
raw = zlib.decompress(idat)
stride = width * channels
out, prev, i = bytearray(), bytearray(stride), 0
for _ in range(height):
    ftype = raw[i]; i += 1
    line = bytearray(raw[i:i + stride]); i += stride
    for x in range(stride):
        a = line[x - channels] if x >= channels else 0
        b = prev[x]
        c = prev[x - channels] if x >= channels else 0
        if ftype == 1: line[x] = (line[x] + a) & 0xFF
        elif ftype == 2: line[x] = (line[x] + b) & 0xFF
        elif ftype == 3: line[x] = (line[x] + (a + b) // 2) & 0xFF
        elif ftype == 4:
            p = a + b - c; pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
            line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 0xFF
    prev = line
    if channels == 4:
        out += line
    else:
        for x in range(width):
            out += line[3 * x:3 * x + 3] + b"\xff"
open(dst, "wb").write(out)
print(json.dumps({"width": width, "height": height, "png_sha256": hashlib.sha256(data).hexdigest(),
                  "rgba_sha256": hashlib.sha256(out).hexdigest()}))
