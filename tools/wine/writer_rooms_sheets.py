"""Folhas de comparação original × port das salas do Writer.
sheets.py DUMP_DIR OUT_DIR"""
import sys, os
from PIL import Image, ImageDraw
S='/home/claude/cw/shots/'
dump, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
PAIRS = {
 'lobby': [('Lobby_arrival','building_16_after_door'),('Lobby_closed','building_18_closed'),
           ('Lobby_o1','building_17_lobby_opt1'),('Lobby_o2','building_21_lobby_opt2'),
           ('Lobby_o3','building_22_lobby_opt3'),('Lobby_o4','building_23_lobby_opt4'),
           ('Lobby_mczee','building_19_mczee_click'),('Lobby_lamp','building_25_lamp'),
           ('Lobby_bag_open','building_28_bag_open')],
 'office': [('Studio_arrival','building_37_esc_welcome'),('Studio_base','building_38_esc_base'),
           ('Studio_o1','building_40_esc_o1'),('Studio_o2','building_41_esc_o2'),
           ('Studio_o3','building_42_esc_o3'),('Studio_o4','building_43_esc_o4'),
           ('Studio_frame','building_44_esc_frame'),('Studio_clip','building_45_esc_clip'),
           ('Studio_frameplate','building_47_esc_frameplate')],
 'projects': [('Projects_arrival','building_58_p_welcome'),('Projects_base','building_59_p_base'),
           ('Projects_max','building_60_p_max'),('Projects_o1','building_61_p_o1'),
           ('Projects_o2','building_61_p_o2'),('Projects_o3','building_61_p_o3'),
           ('Projects_o4','building_61_p_o4'),('Projects_moose','building_62_p_moose'),
           ('Projects_donkey','building_64_p_donkey'),('Projects_rhino','building_65_p_rhino')],
 'portfolio': [('portfolio','building_34_pasta_doc')],
 'ideas': [('Tricks_arrival','building_71_i_welcome'),('Tricks_base','building_72_i_base'),
           ('Tricks_o1','building_74_i_o1'),('Tricks_o2','building_74_i_o2'),
           ('Tricks_o3','building_74_i_o3'),('Tricks_machine','building_75_i_machine'),
           ('Tricks_quadro','building_76_i_quadro')],
}
SC = 0.6
for sheet, pairs in PAIRS.items():
    w, h = int(640*SC), int(480*SC)
    img = Image.new('RGB', (2*w+30, len(pairs)*(h+22)+24), (255,255,255))
    d = ImageDraw.Draw(img)
    d.text((10,4), f'original (Wine)', fill=(0,0,0)); d.text((w+30,4), 'Ateliê Livre (port)', fill=(0,0,0))
    for i,(port, orig) in enumerate(pairs):
        y = 24 + i*(h+22)
        d.text((10,y), f'{orig}', fill=(0,0,120)); d.text((w+30,y), f'cw_{port}', fill=(120,0,0))
        o = Image.open(S+orig+'.png').convert('RGB').resize((w,h), Image.LANCZOS)
        pp = os.path.join(dump, f'cw_{port}.ppm')
        p = Image.open(pp).convert('RGB').resize((w,h), Image.LANCZOS) if os.path.exists(pp) else Image.new('RGB',(w,h),(200,0,0))
        img.paste(o, (10, y+12)); img.paste(p, (w+30, y+12))
    path = os.path.join(out, f'rooms_compare_{sheet}.png')
    img.save(path); print(path, img.size)
