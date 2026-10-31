#!/usr/bin/env python3
"""Gera crates/engine/src/scenes/sticker_scripts.rs a partir das gravações do original."""
import json
d = json.load(open('scratch/steps.json'))

def dedupe(images, rest_id):
    # a mesma imagem aparece com dois ids (cópia no GLOB): fica a do grupo do bicho
    by_pos = {}
    for i, dx, dy in images:
        key = (dx, dy)
        v = int(i, 16)
        if key not in by_pos or abs(v - rest_id) < abs(by_pos[key] - rest_id):
            by_pos[key] = v
    return [(v, dx, dy) for (dx, dy), v in by_pos.items()]

def step(images, frames, rest_id):
    ims = ', '.join(f'(0x{v:04x}, {dx}, {dy})' for v, dx, dy in dedupe(images, rest_id))
    return f'        ScriptStep {{ frames: {frames}, images: &[{ims}] }},'

out = ['//! Roteiros dos adesivos animados do primeiro rolo, gravados do Fine Artist',
       '//! original no Wine (tools/wine/rec.sh + analyze.py + gen_scripts.py).',
       '//! Cada passo: duração em quadros de 1/15 s e imagens (id em ARTIST.PIC,',
       '//! deslocamento do canto superior esquerdo em relação ao clique).',
       '//! Gerado; não editar à mão.', '',
       '/// Um passo da animação.',
       'pub struct ScriptStep {',
       '    pub frames: u8,',
       '    pub images: &\'static [(u16, i16, i16)],',
       '}', '',
       '/// Roteiro: passos, imagem de repouso (onde o adesivo fica) e, se houver,',
       '/// o deslocamento por volta de um roteiro que se repete até sair do quadro.',
       'pub struct Script {',
       '    pub steps: &\'static [ScriptStep],',
       '    pub rest: Option<(u16, i16, i16)>,',
       '    pub repeat_shift: Option<(i16, i16)>,',
       '}', '',
       '/// Quadros por segundo dos roteiros.',
       'pub const SCRIPT_FPS: f32 = 15.0;', '',
       '/// Roteiros dos 16 bichos (`None`: não gravado; usa a animação aproximada).',
       'pub const SCRIPTS: [Option<Script>; 16] = [']
# cobra: ciclo que se repete andando para a direita (gravado com clique em 120,300)
out += ['    Some(Script {', '        steps: &[',
        step([('753b', -109, -33)], 5, 0x753b),
        step([('7543', -53, -41)], 4, 0x753b),
        step([('754b', -41, -35)], 5, 0x753b),
        '        ScriptStep { frames: 4, images: &[] },',
        '        ],', '        rest: None,', '        repeat_shift: Some((127, 0)),', '    }),']
for k in range(1, 16):
    v = d.get(f'1-{k}')
    if not v or not v['rest']:
        out.append('    None,')
        continue
    rid, rdx, rdy = v['rest'][0]
    rest_id = int(rid, 16)
    out += ['    Some(Script {', '        steps: &[']
    out += [step(s['images'], s['frames'], rest_id) for s in v['steps']]
    out += ['        ],', f'        rest: Some((0x{rest_id:04x}, {rdx}, {rdy})),',
            '        repeat_shift: None,', '    }),']
out.append('];')
open('crates/engine/src/scenes/sticker_scripts.rs', 'w').write('\n'.join(out) + '\n')
print('ok')
