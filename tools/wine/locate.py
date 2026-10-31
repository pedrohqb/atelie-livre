#!/usr/bin/env python3
"""Localiza bitmaps extraídos numa captura do original: imprime id, x, y, w, h, fração igual."""
import sys, os
import numpy as np
from PIL import Image

shot = np.asarray(Image.open(sys.argv[1]).convert('RGB')).astype(np.int32)
S = (shot[:, :, 0] << 16) | (shot[:, :, 1] << 8) | shot[:, :, 2]
H, W = S.shape
dirs = sys.argv[2:] or ['out/ARTIST/bitmaps']
minfrac = float(os.environ.get('MINFRAC', '0.97'))
rng = np.random.default_rng(1)
hits = []
for d in dirs:
    for f in sorted(os.listdir(d)):
        if not f.endswith('.png'):
            continue
        im = np.asarray(Image.open(os.path.join(d, f)).convert('RGB')).astype(np.int32)
        h, w = im.shape[:2]
        if w < 10 or h < 10 or w > W or h > H:
            continue
        B = (im[:, :, 0] << 16) | (im[:, :, 1] << 8) | im[:, :, 2]
        # pixels-chave: não brancos, cores variadas
        ys, xs = np.nonzero(B != 0xFFFFFF)
        if len(ys) < int(os.environ.get("MINPX","60")) or len(np.unique(B)) < int(os.environ.get('MINCOL','3')):
            continue
        idx = rng.choice(len(ys), size=min(24, len(ys)), replace=False)
        cand = None
        for k in idx:
            y, x = ys[k], xs[k]
            m = np.zeros((H - h + 1, W - w + 1), bool)
            m[:] = S[y:y + H - h + 1, x:x + W - w + 1] == B[y, x]
            cand = m if cand is None else (cand & m)
            if not cand.any():
                break
        if cand is None or not cand.any():
            continue
        for (cy, cx) in zip(*np.nonzero(cand)):
            win = S[cy:cy + h, cx:cx + w]
            nw = B != 0xFFFFFF
            frac = (win[nw] == B[nw]).mean()
            if frac >= minfrac:
                hits.append((f[:-4], cx, cy, w, h, frac, d))
best = {}
for h in hits:
    k = h[0]
    if k not in best or h[5] > best[k][5]:
        best[k] = h
hits = list(best.values())
for h in sorted(hits, key=lambda t: (t[2], t[1])):
    print(f"{h[0]} x={h[1]} y={h[2]} w={h[3]} h={h[4]} igual={h[5]:.3f} {os.path.basename(h[6])}")
