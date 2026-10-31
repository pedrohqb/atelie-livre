#!/usr/bin/env python3
"""Identifica, quadro a quadro de uma gravação do original, quais imagens de
adesivo (ARTIST.PIC, trincas cor/traço/máscara) aparecem e onde.
Uso: analyze.py PASTA_GRAVACAO CLIQUE_X CLIQUE_Y  → JSON na saída."""
import sys, os, json
import numpy as np
from PIL import Image

BMP = 'out/ARTIST/bitmaps'
CAN = (23, 109, 617, 456)  # x0, y0, x1, y1 (exclusivo)


def load_candidates():
    ids = sorted(int(f[:4], 16) for f in os.listdir(BMP) if f.endswith('.png'))
    S = set(ids)
    cands = []
    for i in ids:
        if not (i + 5 in S and i + 6 in S) or any(i + k in S for k in (1, 2, 3, 4)):
            continue
        im = np.asarray(Image.open(f'{BMP}/{i:04x}.png').convert('RGB')).astype(np.int32)
        mk = np.asarray(Image.open(f'{BMP}/{i + 6:04x}.png').convert('L')) < 128
        if im.shape[:2] != mk.shape or mk.sum() < 30:
            continue
        ys, xs = np.nonzero(mk)
        y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
        col = (im[:, :, 0] << 16) | (im[:, :, 1] << 8) | im[:, :, 2]
        col = col[y0:y1, x0:x1]
        mk = mk[y0:y1, x0:x1]
        nw = mk & (col != 0xFFFFFF)
        if nw.sum() < 10:
            continue
        cands.append(dict(id=i, col=col, mask=mk, nw=nw, nnw=int(nw.sum()),
                          colors=set(np.unique(col[nw]).tolist()), off=(int(x0), int(y0))))
    return cands


def packed(path):
    a = np.asarray(Image.open(path).convert('RGB')).astype(np.int32)
    return (a[:, :, 0] << 16) | (a[:, :, 1] << 8) | a[:, :, 2]


def match_frame(F, base, cands, rng):
    x0, y0, x1, y1 = CAN
    # conhecido: dentro do quadro e onde a base (sem adesivo) é branca; o resto
    # (balão do McZee, fora do quadro) pode esconder o adesivo
    K = np.zeros_like(F, dtype=bool)
    K[y0:y1, x0:x1] = base[y0:y1, x0:x1] == 0xFFFFFF
    D = np.zeros_like(F, dtype=bool)
    sub = F[y0:y1, x0:x1]
    D[y0:y1, x0:x1] = (sub != base[y0:y1, x0:x1]) & (sub != 0xFFFFFF)
    if D.sum() < 20:
        return []
    ys, xs = np.nonzero(D)
    bx0, bx1, by0, by1 = xs.min(), xs.max() + 1, ys.min(), ys.max() + 1
    found = []
    ndiff = int(D.sum())
    if ndiff > 120000:  # tela inteira mudou (diálogo, erro): ignora o quadro
        return []
    fcolors = set(np.unique(F[by0:by1, bx0:bx1]).tolist())
    for c in cands:
        h, w = c['mask'].shape
        if c['nnw'] > ndiff * 1.5 + 50 or not c['colors'] <= fcolors:
            continue
        # a imagem precisa ficar dentro da área alterada (ou sair pela borda do quadro)
        lo_x, hi_x = bx0 - (w - 1), bx1 - 1
        lo_y, hi_y = by0 - (h - 1), by1 - 1
        if hi_x < lo_x or hi_y < lo_y:
            continue
        mys, mxs = np.nonzero(c['mask'])
        kys, kxs = np.nonzero(c['nw'])
        sel = rng.choice(len(kys), size=min(16, len(kys)), replace=False)
        best = None
        X = np.arange(lo_x, hi_x + 1)
        Y = np.arange(lo_y, hi_y + 1)
        H_, W_ = F.shape
        PYs, PXs = [], []
        for k0 in sel[:4]:
            yy = Y[:, None] + kys[k0]
            xx = X[None, :] + kxs[k0]
            cy_, cx_ = np.clip(yy, 0, H_ - 1), np.clip(xx, 0, W_ - 1)
            inside = (yy >= 0) & (yy < H_) & (xx >= 0) & (xx < W_) & K[cy_, cx_]
            # a chave precisa estar visível e igual: posições escondidas não contam
            ok = inside & (F[cy_, cx_] == c['col'][kys[k0], kxs[k0]])
            a, b = np.nonzero(ok)
            PYs.append(Y[a]); PXs.append(X[b])
        P = np.unique(np.stack([np.concatenate(PYs), np.concatenate(PXs)], axis=1), axis=0)
        PY, PX = P[:, 0], P[:, 1]
        for k in sel[1:]:
            if len(PY) == 0:
                break
            yy, xx = PY + kys[k], PX + kxs[k]
            cy_, cx_ = np.clip(yy, 0, H_ - 1), np.clip(xx, 0, W_ - 1)
            inside = (yy >= 0) & (yy < H_) & (xx >= 0) & (xx < W_) & K[cy_, cx_]
            keep = ~inside | (F[cy_, cx_] == c['col'][kys[k], kxs[k]])
            PY, PX = PY[keep], PX[keep]
        for py, px in zip(PY, PX):
            yy, xx = mys + py, mxs + px
            cy_, cx_ = np.clip(yy, 0, F.shape[0] - 1), np.clip(xx, 0, F.shape[1] - 1)
            inside = (yy >= 0) & (yy < F.shape[0]) & (xx >= 0) & (xx < F.shape[1]) & K[cy_, cx_]
            nwv = inside & c['nw'][mys, mxs]
            if nwv.sum() < 0.3 * c['nnw']:
                continue
            eq = F[yy[inside], xx[inside]] == c['col'][mys[inside], mxs[inside]]
            frac = eq.mean()
            if frac >= 0.97 and (best is None or frac > best[0]):
                best = (frac, int(px), int(py), float(inside.mean()))
        if best:
            found.append(dict(id=c['id'], x=best[1] - c['off'][0], y=best[2] - c['off'][1],
                              score=round(best[0], 3), vis=round(best[3], 2), n=int(c['mask'].sum())))
    # descarta achados contidos em outros maiores (mesma região, imagem menor)
    found.sort(key=lambda f: -f['n'])
    return found


def main():
    d, cx, cy = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    cands = load_candidates()
    base = packed(f'{d}/base.png')
    rng = np.random.default_rng(1)
    out = []
    cache = {}
    for f in sorted(x for x in os.listdir(d) if x.startswith('f')):
        F = packed(f'{d}/{f}')
        key = hash(F.tobytes())
        if key not in cache:
            cache[key] = match_frame(F, base, cands, rng)
        res = cache[key]
        out.append([dict(id=f"{r['id']:04x}", dx=r['x'] - cx, dy=r['y'] - cy, vis=r['vis']) for r in res])
    print(json.dumps(out))


if __name__ == '__main__':
    main()
