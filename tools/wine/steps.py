#!/usr/bin/env python3
"""Converte as análises das gravações (scratch/stickers/r*_k*.json) em roteiros:
lista de passos (imagens com deslocamento em relação ao clique, duração em
quadros de 1/15 s) e a imagem de repouso. Saída: JSON na saída padrão."""
import json, glob, re, sys

FPS = 15
out = {}
for path in sorted(glob.glob('scratch/stickers/r*_k*.json'),
                   key=lambda p: tuple(map(int, re.findall(r'\d+', p)))):
    r, k = map(int, re.findall(r'r(\d+)_k(\d+)', path)[0])
    txt = open(path).read()
    if not txt.strip():
        continue
    frames = json.loads(txt)
    key = [tuple((x['id'], x['dx'], x['dy']) for x in f) for f in frames]
    start = next((i for i, f in enumerate(key) if f != key[0]), len(key))
    if start > 12:
        start = 9
    steps = []
    for f in key[start:]:
        if steps and steps[-1][0] == f:
            steps[-1][1] += 1
        else:
            steps.append([f, 1])
    rest = steps[-1][0] if steps else key[0]
    anim = steps[:-1] if len(steps) > 1 else []
    out[f'{r}-{k}'] = dict(
        preview=[list(x) for x in key[0]],
        steps=[dict(images=[list(x) for x in f], frames=n) for f, n in anim],
        rest=[list(x) for x in rest],
    )
json.dump(out, sys.stdout, indent=1)
