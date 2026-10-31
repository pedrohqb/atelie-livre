#!/usr/bin/env python3
"""Ícone 256×256 do Ateliê Livre (desenho próprio: paleta com três cores e um pincel)."""
import sys, struct, zlib, math

W = H = 256
px = bytearray(W * H * 4)

def put(x, y, c):
    if 0 <= x < W and 0 <= y < H:
        i = 4 * (y * W + x)
        px[i:i + 4] = bytes(c)

def disc(cx, cy, r, c):
    for y in range(int(cy - r) - 1, int(cy + r) + 2):
        for x in range(int(cx - r) - 1, int(cx + r) + 2):
            if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                put(x, y, c)

# paleta: elipse bege com borda escura e um recorte (o buraco do polegar)
for y in range(H):
    for x in range(W):
        dx, dy = (x - 128) / 118.0, (y - 140) / 96.0
        d = dx * dx + dy * dy
        if d <= 1.0:
            put(x, y, (250, 225, 160, 255) if d < 0.92 else (90, 50, 20, 255))
disc(80, 170, 22, (0, 0, 0, 0))
# tintas
disc(80, 110, 20, (220, 40, 40, 255))
disc(128, 92, 20, (250, 200, 30, 255))
disc(176, 110, 20, (40, 90, 220, 255))
disc(170, 170, 20, (50, 170, 70, 255))
# pincel (cabo marrom, virola cinza, pelos pretos)
for t in range(0, 150):
    x = 60 + t
    y = 236 - t
    for w in range(-7, 8):
        put(x + w, y, (150, 90, 40, 255))
for t in range(150, 165):
    x = 60 + t
    y = 236 - t
    for w in range(-7, 8):
        put(x + w, y, (170, 170, 180, 255))
for t in range(165, 200):
    x = 60 + t
    y = 236 - t
    for w in range(-6, 7):
        put(x + w, y, (30, 30, 30, 255))

def chunk(tag, data):
    c = struct.pack('>I', len(data)) + tag + data
    return c + struct.pack('>I', zlib.crc32(tag + data) & 0xffffffff)

raw = b''.join(b'\x00' + bytes(px[4 * y * W:4 * (y + 1) * W]) for y in range(H))
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', W, H, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b'')
open(sys.argv[1], 'wb').write(png)
