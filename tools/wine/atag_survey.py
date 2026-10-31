#!/usr/bin/env python3
"""Levantamento dos roteiros ATAG de um executável: origem, imagens, tamanhos,
duração e sons de cada um, com a versão colorida e a máscara de cada célula
(imagem de id anterior no .PIC e id + 1; ver docs/formats/atag.md).

uso: atag_survey.py PASTA_ATAG PASTA_PNG [ID...]
  PASTA_ATAG: recursos extraídos (XXXX.bin), ex. out/resources/ARTIST.EXE/ATAG
  PASTA_PNG:  imagens do .PIC extraídas (XXXX.png), ex. out/ARTIST/bitmaps
Sem IDs, lista uma linha por roteiro; com IDs, imprime as células passo a passo
(posição acumulada de cada célula, como o Animator do motor as desenha).
"""
import glob
import os
import struct
import sys

from PIL import Image


def parse(d):
    p = 12  # versão, flags, reservado
    cells, moves, kind, origin = [], [], None, (0, 0)
    while p + 2 <= len(d):
        op = struct.unpack_from('<H', d, p)[0]
        p += 2
        if op == 0:
            break
        if op == 1:
            continue
        if op == 2:
            p += 6
            n = struct.unpack_from('<H', d, p)[0]
            p += 2
            for _ in range(n):
                _, _, img, mask, dx, dy, dur, snd = struct.unpack_from('<HHHHhhIH', d, p)
                p += 18
                cells.append((img, mask, dur, snd))
        elif op == 4:
            p += 2
            kind = struct.unpack_from('<H', d, p)[0]
            p += 2
            origin = struct.unpack_from('<hh', d, p)
            p += 8
            n = struct.unpack_from('<H', d, p)[0]
            p += 2
            for _ in range(n):
                _, tw, dx, dy, _ = struct.unpack_from('<BBhhH', d, p)
                p += 8
                moves.append((dx, dy, tw))
        else:
            raise ValueError(f'código {op} em {p}')
    return cells, moves, kind, origin


class Pics:
    def __init__(self, folder):
        self.folder = folder
        self.ids = sorted(int(f[:4], 16) for f in os.listdir(folder) if f.endswith('.png'))
        self.cache = {}

    def info(self, i):
        if i not in self.cache:
            f = f'{self.folder}/{i:04x}.png'
            if os.path.exists(f):
                im = Image.open(f).convert('RGB')
                self.cache[i] = (im.size, len(im.getcolors(1 << 20) or []))
            else:
                self.cache[i] = None
        return self.cache[i]

    def triple(self, x):
        """(colorida, 1 bit, máscara) da imagem de 1 bit x."""
        k = self.ids.index(x) if x in self.ids else -1
        prev = self.ids[k - 1] if k > 0 else None
        xi = self.info(x)
        ci = self.info(prev) if prev else None
        mi = self.info(x + 1)
        colour = prev if ci and xi and ci[0] == xi[0] else None
        mask = x + 1 if mi and xi and abs(mi[0][0] - xi[0][0]) <= 1 and abs(mi[0][1] - xi[0][1]) <= 1 else None
        return colour, x, mask


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    res, pics = sys.argv[1], Pics(sys.argv[2])
    wanted = [int(a, 16) for a in sys.argv[3:]]
    for f in sorted(glob.glob(f'{res}/*.bin')):
        sid = int(os.path.basename(f)[:4], 16)
        if wanted and sid not in wanted:
            continue
        cells, moves, kind, origin = parse(open(f, 'rb').read())
        if not wanted:
            total = sum(c[2] for c in cells)
            imgs = sorted({c[0] for c in cells})
            snds = sorted({c[3] for c in cells if c[3]})
            move = (sum(m[0] for m in moves), sum(m[1] for m in moves))
            trip = [pics.triple(i) for i in imgs]
            print(f'{sid:04x} tipo {kind} origem {origin} células {len(cells)} ms {total} '
                  f'desloc {move} imagens {[hex(i) for i in imgs]} sons {[hex(s) for s in snds]} '
                  f'sem cor {[hex(b) for c, b, m in trip if c is None]} sem máscara {[hex(b) for c, b, m in trip if m is None]}')
            continue
        print(f'{sid:04x} tipo {kind} origem {origin}')
        x, y = origin
        for i, (img, _, dur, snd) in enumerate(cells):
            m = moves[i] if i < len(moves) else (0, 0, 0)
            sz = pics.info(img)
            c, _, mk = pics.triple(img)
            print(f'  célula {i}: 1 bit {img:04x} cor {c and f"{c:04x}"} máscara {mk and f"{mk:04x}"} '
                  f'{sz and sz[0]} {dur} ms som {snd:#x} em ({x},{y}) depois move {m}')
            x += m[0]
            y += m[1]


if __name__ == '__main__':
    main()
