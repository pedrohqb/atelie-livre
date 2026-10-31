#!/usr/bin/env python3
"""Decompõe uma captura do original em fundo + imagens de ARTIST.PIC.
Uso: compose.py CAPTURA ID_FUNDO  → linhas "id x y ganho" (ordem de escolha).
Usa locate.py para achar candidatos e escolhe, gulosamente, os que explicam
mais pixels diferentes do fundo."""
import sys, os, subprocess
import numpy as np
from PIL import Image

BMP = 'out/ARTIST/bitmaps'
shot_path, bg = sys.argv[1], sys.argv[2]
S = np.asarray(Image.open(shot_path).convert('RGB')).astype(np.int32)
B = np.asarray(Image.open(f'{BMP}/{bg}.png').convert('RGB')).astype(np.int32)
H, W = S.shape[:2]
bg_full = np.zeros_like(S)
bh, bw = B.shape[:2]
bg_full[:min(H, bh), :min(W, bw)] = B[:H, :W]
R = np.any(S != bg_full, axis=2)

env = dict(os.environ, MINFRAC='0.97', MINCOL='2', MINPX='40')
out = subprocess.run(['python3', os.path.join(os.path.dirname(__file__), 'locate.py'), shot_path, BMP],
                     capture_output=True, text=True, env=env).stdout
hits = []
for line in out.splitlines():
    p = line.split()
    i, x, y = p[0], int(p[1][2:]), int(p[2][2:])
    if i == bg:
        continue
    im = np.asarray(Image.open(f'{BMP}/{i}.png').convert('RGB')).astype(np.int32)
    h, w = im.shape[:2]
    x1, y1 = min(W, x + w), min(H, y + h)
    sub = im[:y1 - y, :x1 - x]
    ok = np.all(S[y:y1, x:x1] == sub, axis=2) & np.any(sub != 255, axis=2) & R[y:y1, x:x1]
    ncol = len(np.unique(im.reshape(-1, 3), axis=0))
    hits.append([i, x, y, ok, ncol])

explained = np.zeros_like(R)
chosen = []
while hits:
    best, gain = None, 0
    for h in hits:
        i, x, y, ok, ncol = h
        g = int((ok & ~explained[y:y + ok.shape[0], x:x + ok.shape[1]]).sum())
        # entre a versão colorida e a de 1 bit da mesma imagem, fica a colorida
        if g > gain or (g == gain and best is not None and ncol > best[4]):
            best, gain = h, g
    if best is None or gain < 150:
        break
    i, x, y, ok, _ = best
    explained[y:y + ok.shape[0], x:x + ok.shape[1]] |= ok
    chosen.append((i, x, y, gain))
    hits.remove(best)
for c in chosen:
    print(*c)
print(f'# explicado {explained.sum()} de {R.sum()} pixels diferentes do fundo', file=sys.stderr)
