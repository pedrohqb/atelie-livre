#!/usr/bin/env python3
"""Gera crates/engine/src/scenes/rooms_data.rs a partir das composições (compose.py)."""
import os, sys
import numpy as np
from PIL import Image
BMP = 'out/ARTIST/bitmaps'

def load(i):
    return np.asarray(Image.open(f'{BMP}/{i:04x}.png').convert('RGB'))

def find_mask(i):
    im = load(i); h, w = im.shape[:2]
    nonwhite = np.any(im != 255, axis=2)
    best = None
    for k in range(1, 20):
        p = f'{BMP}/{i + k:04x}.png'
        if not os.path.exists(p):
            continue
        m = np.asarray(Image.open(p).convert('RGB'))
        if m.shape[:2] != (h, w) or len(np.unique(m.reshape(-1, 3), axis=0)) > 2:
            continue
        dark = np.all(m < 128, axis=2)
        if (dark | ~nonwhite).all():  # cobre todo pixel não branco
            score = dark.sum()
            if best is None or score < best[0]:
                best = (score, i + k)
    return best[1] if best else None

EXTRA = {'STUDIO': [(0x6c73, 79, 135)]}
rooms = {
    'LOBBY': ('6a6b', '/tmp/c15.txt', set()),
    # 6c74 é a versão de 1 bit do cavalete (6c73, colorido, fica escondido pela Maggie na captura);
    # os ids pequenos são pedaços da Maggie, que é dinâmica
    'STUDIO': ('6c6b', '/tmp/c17.txt', {'6c74', '14ed', '6719', '3a7d', '66e3'}),
    'PROJECTS': ('67fb', '/tmp/cpole2.txt', set()),
    'TRICKS': ('687b', '/tmp/cpole3.txt', set()),
}
out = ['//! Elementos fixos das salas, medidos em capturas do original (tools/wine/compose.py).',
       '//! Gerado por tools/wine/rooms_gen.py; não editar à mão.', '',
       '/// Imagem de `ARTIST.PIC`, máscara (ou 0) e posição na tela.',
       'pub struct Element {', '    pub id: u16,', '    pub mask: u16,', '    pub x: i32,', '    pub y: i32,', '}', '']
for name, (bg, path, skip) in rooms.items():
    out.append(f'pub const {name}_BG: u16 = 0x{bg};')
    out.append(f'pub const {name}: &[Element] = &[')
    for line in open(path):
        p = line.split()
        if not p or p[0].startswith('#') or p[0] in skip:
            continue
        i = int(p[0], 16)
        m = find_mask(i) or 0
        out.append(f'    Element {{ id: 0x{i:04x}, mask: 0x{m:04x}, x: {p[1]}, y: {p[2]} }},')
    for (i, x, y) in EXTRA.get(name, []):
        m = find_mask(i) or 0
        out.append(f'    Element {{ id: 0x{i:04x}, mask: 0x{m:04x}, x: {x}, y: {y} }},')
    out.append('];')
    out.append('')
open('crates/engine/src/scenes/rooms_data.rs', 'w').write('\n'.join(out))
print('ok')
