#!/usr/bin/env python3
"""Desenha fundo + imagens (saída do compose.py) e salva lado a lado com a captura e a diferença.
Uso: render_comp.py CAPTURA ID_FUNDO COMPOSICAO.txt SAIDA.png"""
import sys
import numpy as np
from PIL import Image
BMP = 'out/ARTIST/bitmaps'
shot, bg, comp, out = sys.argv[1:5]
S = np.asarray(Image.open(shot).convert('RGB')).astype(np.int32)
F = np.zeros_like(S)
B = np.asarray(Image.open(f'{BMP}/{bg}.png').convert('RGB')).astype(np.int32)
F[:min(480, B.shape[0]), :min(640, B.shape[1])] = B[:480, :640]
import os
for line in open(comp):
    p = line.split()
    if not p or p[0].startswith('#'):
        continue
    i, x, y = p[0], int(p[1]), int(p[2])
    im = np.asarray(Image.open(f'{BMP}/{i}.png').convert('RGB')).astype(np.int32)
    v = int(i, 16)
    mpath = next((f'{BMP}/{v + k:04x}.png' for k in (2, 6, 1) if os.path.exists(f'{BMP}/{v + k:04x}.png')
                  and Image.open(f'{BMP}/{v + k:04x}.png').size == (im.shape[1], im.shape[0])), None)
    m = (np.asarray(Image.open(mpath).convert('L')) < 128) if mpath else np.any(im != 255, axis=2)
    h, w = im.shape[:2]
    for yy in range(h):
        for xx in range(w):
            if m[yy, xx] and 0 <= y + yy < 480 and 0 <= x + xx < 640:
                F[y + yy, x + xx] = im[yy, xx]
d = np.any(F != S, axis=2)
print(f'iguais {100 - 100 * d.mean():.2f}%')
o = np.concatenate([F, S, np.where(d[..., None], [255, 0, 255], S // 3)], axis=1).astype(np.uint8)
Image.fromarray(o).save(out)
