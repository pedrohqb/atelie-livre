# `ATAG` do `WRITER.EXE` — catálogo das animações

> Levantamento da pesquisa do Creative Writer (2026). As capturas (`shots/…`),
> ferramentas (`tools/…`) e amostras (`amostras/…`) citadas ficam na pasta da
> pesquisa, fora do repositório.

155 roteiros no formato de `atag.md` (variante dos executáveis: blocos 1, 2,
4; imagens são ids do `WRITER.PIC`; sons são ids do `WRITER.SND`). Todos usam
caminho do tipo 5 (um deslocamento por célula), nenhum usa o tipo 9.
Reprodução: o motor do port (`engine::anim`, `scenes::sticker_scripts`) já
toca roteiros `ATAG` do Fine Artist; o Writer só precisa da tabela
roteiro → sala/gatilho.

Folha de contato das figuras-chave: `shots/formats_atag_figuras.png`.

## Grupos

| Grupo | Ids | O que é | Como dispara (a confirmar nas capturas das telas) |
|---|---|---|---|
| Travessias | 0x800–0x805, 0x836–0x839, 0x861, 0x868–0x86b, 0x881–0x885, 0x8b9–0x8bd, 0x8bf–0x8c0, 0x910–0x914, … | bichos e personagens que cruzam a tela vindos de fora (origem x ≈ 574–643 ou −126 a −409; deslocamento total ±665 a ±1076 px): menino de skate (0x606c, som 0x2fc), avião com a faixa "você também é um artista!" (0x65d8, 68 células, som 0x2d3), patinador (0x66a4), cabrito (0x64e8, 0x239), papagaio (0x6598, 0x20a/0x217/0x22f), galo (0x6538, 0x20f), tiranossauro (0x655c, 0x2e6), polvo (0x60dc, 0x30a), pterossauro (0x60fc, 0x211), tartaruga de guarda-chuva (0x60bc, 0x301) | o mesmo bicho aparece em vários grupos de ids (um conjunto por sala: título/saguão 0x800–0x805, 0x836–0x839, 0x868–0x86b, 0x881–0x885, 0x8b9–0x8c0, 0x910–0x914) — sorteio de "gags" de fundo da sala, como no Fine Artist |
| Gags locais | 0x80c–0x81b, 0x857–0x85c, 0x870–0x876, 0x87f–0x880, 0x8a2–0x8ab, 0x8be, 0x906–0x90a, 0x92e–0x934 | animações curtas presas a um objeto da sala (origem pequena, relativa ao objeto): mancha (0x6408), porta (0x6250), hidrante (0x6220), rato de capa (0x621c), rato com bazuca (0x6234, 28 células), cabeça de T-rex, robô, coelho, boneco pixelado (0x62a4, 21 células), nuvem, cachorro, capacete, olhos, pêndulos (0x660c/0x6608) | clique em objetos das salas (equivalente aos `room_objects` do port) |
| Chapéus do McZee | 0x83a–0x844 e 0x8c1–0x8ca (11 + 10) | 5 células: estrela/explosão (0x6114…) e termina com o McZee com um chapéu diferente (0x6134 cartola, 0x616c capacete de bombeiro…); sons 0x281, 0x2f1, 0x2f4 | McZee trocando de chapéu (duas posições: (205, 165) e (160, 170)) |
| Poses de 3 s | 0x8d9–0x8e8, 0x8fc–0x901, 0x961, 0x9c6–0x9c8, 0xa28–0xa2b, 0xa8f–0xa91 | uma célula de 3000 ms com som 0x293: objetos (sombrero, luvas de boxe…) e as formas do mastro (cifrão 0x672c, gancho 0x66b8, nó 0x66d0, mola 0x6734) | troca momentânea de objeto; o mastro muda de forma antes da descida |
| Mastro de bombeiro | 0x962–0x966, 0x9c9–0x9d0, 0xa2c–0xa30, 0xa92–0xa99 (quatro conjuntos) | personagens descendo (dy ≈ +416 a +634) ou subindo (dy ≈ −347 a −710) pelo mastro (figuras 0x654c, 0x668c, 0x666c, 0x6660, 0x65b4, 0x65a4, 0x66e8); sons 0x2f2, 0x20a, 0x28a, 0x310, 0x210, 0x309 | atalho entre andares (WHAT 0bb8 "Studio-firepole", "Proj-firepole", "Idea--firepole"); um conjunto por andar |
| Interface | 0x820–0x823, 0x893 | 1 célula de 250 ms (0x603c, 0x6028, 0x6054) e barra que sobe/desce 70 px (0x6068, 0x822/0x823) | botões e porta do elevador |

## Tabela completa

| Id | Grupo | Células | Imagens | Duração (ms) | Sons | Origem | Deslocamento | Figura |
|---|---|---|---|---|---|---|---|---|
| 0x800 | travessia | 22 | 0x606c–0x6074 | 3520 | 0x2fc | (574, 269) | (-665, -16) | menino de skate |
| 0x801 | travessia | 68 | 0x65d8–0x65e0 | 6800 | 0x2d3 | (636, 125) | (-1076, 0) | avião com faixa "você também é um artista!" |
| 0x802 | travessia | 23 | 0x66a4–0x66b4 | 3680 | 0x2fc | (624, 284) | (-697, -6) | patinador |
| 0x803 | travessia | 15 | 0x64e8–0x6500 | 3600 | 0x239 | (615, 246) | (-721, 30) | bode/cabrito saltando |
| 0x804 | travessia | 50 | 0x6598–0x65b0 | 5000 | 0x20a, 0x217, 0x22f | (643, 126) | (-756, 2) | papagaio |
| 0x805 | travessia | 31 | 0x6538–0x6550 | 6200 | 0x20f | (-190, 317) | (883, -18) | galo |
| 0x80c | gag local | 21 | 0x6408–0x6438 | 7800 | 0x2bc, 0x2bd, 0x2be, 0x2bf, 0x2f1, 0x2f2 | (413, 237) | (-167, 7) | mancha |
| 0x811 | gag local | 9 | 0x6250–0x6258 | 1800 | 0x2f9 | (-2, -1) | (0, 0) | porta |
| 0x816 | gag local | 17 | 0x6220–0x6240 | 3400 | 0x221, 0x26e, 0x2bc, 0x2be, 0x303 | (-5, -4) | (4, -47) | hidrante |
| 0x81b | gag local | 9 | 0x6268–0x6288 | 3600 | 0x2f8, 0x2fd, 0x309, 0x30a | (-7, -10) | (0, 0) | caixa de correio/sanduíche |
| 0x820 | interface (elevador/botões) | 1 | 0x603c–0x603c | 250 | — | (0, 0) | (0, 0) |  |
| 0x821 | interface (elevador/botões) | 1 | 0x6028–0x6028 | 250 | — | (0, 0) | (0, 0) |  |
| 0x822 | interface (elevador/botões) | 7 | 0x6068–0x6090 | 1400 | — | (74, 134) | (0, 70) | barra (elevador) |
| 0x823 | interface (elevador/botões) | 7 | 0x6068–0x6090 | 1400 | — | (74, 204) | (0, -70) | barra (elevador) |
| 0x836 | travessia | 26 | 0x655c–0x657c | 5200 | 0x2e6 | (-402, 336) | (1053, 0) | tiranossauro |
| 0x837 | travessia | 14 | 0x64e8–0x6500 | 3400 | 0x239 | (588, 286) | (-771, -18) | bode/cabrito saltando |
| 0x838 | travessia | 24 | 0x60dc–0x60f4 | 4800 | 0x30a | (-126, 238) | (721, 6) | polvo |
| 0x839 | travessia | 22 | 0x606c–0x6074 | 3520 | 0x2fc | (574, 269) | (-665, -16) | menino de skate |
| 0x83a | chapéus do McZee | 5 | 0x6114–0x6124 | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x83b | chapéus do McZee | 5 | 0x6114–0x612c | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x83c | chapéus do McZee | 5 | 0x6114–0x6134 | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x83d | chapéus do McZee | 5 | 0x6114–0x613c | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x83e | chapéus do McZee | 5 | 0x6114–0x6144 | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x83f | chapéus do McZee | 5 | 0x6114–0x614c | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x840 | chapéus do McZee | 5 | 0x6114–0x6154 | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x841 | chapéus do McZee | 5 | 0x6114–0x615c | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x842 | chapéus do McZee | 5 | 0x6114–0x6164 | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x843 | chapéus do McZee | 5 | 0x6114–0x616c | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x844 | chapéus do McZee | 5 | 0x6114–0x6174 | 3800 | 0x281, 0x2f1, 0x2f4 | (205, 165) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x857 | gag local | 7 | 0x621c–0x622c | 2100 | 0x282, 0x30f | (-1, -6) | (-42, 15) | rato de capa |
| 0x858 | gag local | 28 | 0x6234–0x626c | 8400 | 0x301 | (-30, 0) | (149, -87) | rato com bazuca |
| 0x859 | gag local | 12 | 0x6274–0x627c | 3600 | 0x2e6 | (-11, 4) | (0, 0) | T-rex (cabeça) |
| 0x85a | gag local | 5 | 0x6284–0x6284 | 1500 | 0x30d | (15, 1) | (0, 0) | robô |
| 0x85b | gag local | 12 | 0x628c–0x629c | 3600 | 0x288 | (11, 9) | (135, -136) | coelho |
| 0x85c | gag local | 21 | 0x62a4–0x62fc | 6300 | 0x30e | (37, 9) | (0, 0) | boneco pixelado |
| 0x861 | travessia | 12 | 0x6178–0x61c8 | 3500 | — | (-13, 245) | (494, -79) | carrinho de mão |
| 0x868 | travessia | 15 | 0x64e8–0x6500 | 3800 | 0x239 | (615, 273) | (-815, -14) | bode/cabrito saltando |
| 0x869 | travessia | 50 | 0x6598–0x65b0 | 5000 | 0x20a, 0x217, 0x22f | (643, 45) | (-756, 2) | papagaio |
| 0x86a | travessia | 22 | 0x606c–0x6074 | 3520 | 0x2fc | (574, 269) | (-665, -16) | menino de skate |
| 0x86b | travessia | 23 | 0x66a4–0x66b4 | 3680 | 0x2fc | (624, 284) | (-697, -6) | patinador |
| 0x870 | gag local | 12 | 0x6274–0x627c | 3600 | 0x2e6 | (-11, 3) | (0, 0) | T-rex (cabeça) |
| 0x871 | gag local | 12 | 0x6304–0x6314 | 3600 | 0x288, 0x2fc | (0, -3) | (0, 0) | nuvem |
| 0x872 | gag local | 12 | 0x628c–0x629c | 3600 | 0x288 | (11, 8) | (135, -136) | coelho |
| 0x873 | gag local | 19 | 0x6348–0x6358 | 5700 | 0x2bc, 0x2be | (10, -5) | (10, 0) | cachorro |
| 0x874 | gag local | 12 | 0x6360–0x6388 | 3600 | 0x311 | (4, -1) | (3, -71) | capacete |
| 0x875 | gag local | 6 | 0x6390–0x63a0 | 1800 | 0x22d, 0x230 | (10, 17) | (0, 0) | olhos |
| 0x876 | mastro (desce/sobe) | 22 | 0x6524–0x6544 | 4400 | 0x294 | (17, 2) | (-4, -422) | boneco pixelado 2 |
| 0x87f | gag local | 15 | 0x660c–0x663c | 3000 | 0x28a, 0x309 | (556, 160) | (0, 0) | pêndulo |
| 0x880 | gag local | 14 | 0x6608–0x6620 | 2800 | 0x28a, 0x309 | (234, 211) | (0, 0) | pêndulo 2 |
| 0x881 | travessia | 22 | 0x606c–0x6074 | 3520 | 0x2fc | (574, 269) | (-665, -16) | menino de skate |
| 0x882 | travessia | 49 | 0x6598–0x65b0 | 4900 | 0x20a, 0x217, 0x22f | (643, 192) | (-741, 0) | papagaio |
| 0x883 | travessia | 23 | 0x66a4–0x66b4 | 3680 | 0x2fc | (624, 284) | (-697, -6) | patinador |
| 0x884 | travessia | 14 | 0x64e8–0x6500 | 3400 | 0x239 | (611, 259) | (-766, 6) | bode/cabrito saltando |
| 0x885 | travessia | 25 | 0x655c–0x657c | 5000 | 0x2e6 | (-409, 327) | (1012, -37) | tiranossauro |
| 0x893 | interface (elevador/botões) | 1 | 0x6054–0x6054 | 250 | — | (0, 0) | (0, 0) |  |
| 0x8a2 | gag local | 7 | 0x621c–0x622c | 2100 | 0x282, 0x30f | (-1, -6) | (-42, 15) | rato de capa |
| 0x8a3 | gag local | 28 | 0x6234–0x626c | 8400 | 0x301 | (-30, 0) | (149, -87) | rato com bazuca |
| 0x8a4 | gag local | 12 | 0x6274–0x627c | 3600 | 0x2e6 | (-11, 4) | (0, 0) | T-rex (cabeça) |
| 0x8a5 | gag local | 12 | 0x6304–0x6314 | 3600 | 0x2fc | (0, -2) | (0, 0) | nuvem |
| 0x8a6 | travessia | 28 | 0x6320–0x6340 | 8400 | — | (-528, -127) | (536, 134) | pinguim |
| 0x8a7 | gag local | 19 | 0x6348–0x6358 | 5700 | 0x2bc, 0x2be | (10, -4) | (10, 0) | cachorro |
| 0x8a8 | gag local | 21 | 0x62a4–0x62fc | 6300 | 0x30e | (37, 9) | (0, 0) | boneco pixelado |
| 0x8a9 | gag local | 12 | 0x6360–0x6388 | 3600 | 0x311 | (4, 0) | (6, -64) | capacete |
| 0x8aa | gag local | 8 | 0x6390–0x63a0 | 2400 | 0x22d, 0x230 | (10, 18) | (0, 0) | olhos |
| 0x8ab | mastro (desce/sobe) | 22 | 0x6024–0x6544 | 4400 | 0x294 | (18, -3) | (-9, -401) | boneco pixelado 2 |
| 0x8b9 | travessia | 24 | 0x60fc–0x6104 | 4800 | 0x211 | (606, 179) | (-735, 85) | pterossauro |
| 0x8ba | travessia | 49 | 0x6598–0x65b0 | 4900 | 0x20a, 0x217, 0x22f | (643, 192) | (-740, 0) | papagaio |
| 0x8bb | travessia | 25 | 0x655c–0x657c | 7500 | 0x2e6 | (-404, 340) | (1011, -36) | tiranossauro |
| 0x8bc | travessia | 14 | 0x64e8–0x6500 | 3400 | 0x239 | (615, 287) | (-771, -18) | bode/cabrito saltando |
| 0x8bd | mastro (desce/sobe) | 29 | 0x60bc–0x60d4 | 5800 | 0x301 | (41, 458) | (374, -613) |  |
| 0x8be | gag local | 7 | 0x65f4–0x663c | 4200 | 0x28a, 0x309 | (221, 97) | (0, 0) | pêndulo 3 |
| 0x8bf | travessia | 22 | 0x606c–0x6074 | 3520 | 0x2fc | (574, 269) | (-665, -16) | menino de skate |
| 0x8c0 | travessia | 24 | 0x60dc–0x60f4 | 4800 | 0x30a | (-126, 238) | (721, 6) | polvo |
| 0x8c1 | chapéus do McZee | 5 | 0x6114–0x618c | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8c2 | chapéus do McZee | 5 | 0x6114–0x6194 | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8c3 | chapéus do McZee | 5 | 0x6114–0x619c | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8c4 | chapéus do McZee | 5 | 0x6114–0x61a4 | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8c5 | chapéus do McZee | 5 | 0x6114–0x61ac | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8c6 | chapéus do McZee | 5 | 0x6114–0x61b4 | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8c7 | chapéus do McZee | 5 | 0x6114–0x61bc | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8c8 | chapéus do McZee | 5 | 0x6114–0x61c4 | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8c9 | chapéus do McZee | 5 | 0x6114–0x61cc | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8ca | chapéus do McZee | 5 | 0x6114–0x61d4 | 3800 | 0x281, 0x2f1, 0x2f4 | (160, 170) | (0, 0) | estrela/explosão + chapéu do McZee |
| 0x8d9 | pose/objeto 3 s | 1 | 0x6448–0x6448 | 3000 | 0x293 | (86, 12) | (0, 0) | objeto (chapéu, sombrero...) |
| 0x8da | pose/objeto 3 s | 1 | 0x6450–0x6450 | 3000 | 0x293 | (39, -54) | (0, 0) | sombrero |
| 0x8db | pose/objeto 3 s | 1 | 0x6458–0x6458 | 3000 | 0x293 | (51, 24) | (0, 0) |  |
| 0x8dc | pose/objeto 3 s | 1 | 0x6460–0x6460 | 3000 | 0x293 | (51, -8) | (0, 0) |  |
| 0x8dd | pose/objeto 3 s | 1 | 0x6468–0x6468 | 3000 | 0x293 | (107, 28) | (0, 0) |  |
| 0x8de | pose/objeto 3 s | 1 | 0x6470–0x6470 | 3000 | 0x293 | (45, -8) | (0, 0) |  |
| 0x8e3 | pose/objeto 3 s | 1 | 0x6480–0x6480 | 3000 | 0x293 | (71, 37) | (0, 0) |  |
| 0x8e4 | pose/objeto 3 s | 1 | 0x6488–0x6488 | 3000 | 0x293 | (42, 21) | (0, 0) |  |
| 0x8e5 | pose/objeto 3 s | 1 | 0x6490–0x6490 | 3000 | 0x293 | (79, -5) | (0, 0) |  |
| 0x8e6 | pose/objeto 3 s | 1 | 0x6498–0x6498 | 3000 | 0x293 | (25, 0) | (0, 0) |  |
| 0x8e7 | pose/objeto 3 s | 1 | 0x64a0–0x64a0 | 3000 | 0x293 | (35, 13) | (0, 0) |  |
| 0x8e8 | pose/objeto 3 s | 1 | 0x64a8–0x64a8 | 3000 | 0x293 | (19, 0) | (0, 0) |  |
| 0x8fc | pose/objeto 3 s | 1 | 0x631c–0x631c | 3000 | 0x293 | (6, -1) | (0, 0) | luvas de boxe |
| 0x8fd | pose/objeto 3 s | 1 | 0x6324–0x6324 | 3000 | 0x293 | (9, -20) | (0, 0) |  |
| 0x8fe | pose/objeto 3 s | 1 | 0x632c–0x632c | 3000 | 0x293 | (9, -3) | (0, 0) |  |
| 0x8ff | pose/objeto 3 s | 1 | 0x633c–0x633c | 3000 | 0x293 | (9, -23) | (0, 0) |  |
| 0x900 | pose/objeto 3 s | 1 | 0x6344–0x6344 | 3000 | 0x293 | (6, -3) | (0, 0) |  |
| 0x901 | pose/objeto 3 s | 1 | 0x634c–0x634c | 3000 | 0x293 | (9, -35) | (0, 0) |  |
| 0x906 | gag local | 28 | 0x6234–0x626c | 8400 | 0x301 | (-31, 1) | (149, -87) | rato com bazuca |
| 0x907 | travessia | 28 | 0x6320–0x6340 | 8400 | — | (-529, -126) | (536, 134) | pinguim |
| 0x908 | gag local | 21 | 0x62a4–0x62fc | 6300 | 0x30e | (36, 10) | (0, 0) | boneco pixelado |
| 0x909 | gag local | 11 | 0x646c–0x648c | 1100 | 0x2fc | (9, 19) | (72, 140) | bola |
| 0x90a | mastro (desce/sobe) | 20 | 0x6524–0x6544 | 4000 | 0x294 | (17, -1) | (38, -396) | boneco pixelado 2 |
| 0x910 | travessia | 15 | 0x64e8–0x6500 | 3600 | 0x239 | (615, 246) | (-721, 30) | bode/cabrito saltando |
| 0x911 | travessia | 28 | 0x6538–0x6550 | 5600 | 0x20f | (-179, 243) | (817, 66) |  |
| 0x912 | travessia | 22 | 0x606c–0x6074 | 3520 | 0x2fc | (574, 269) | (-665, -16) | menino de skate |
| 0x913 | travessia | 24 | 0x60dc–0x60f4 | 4800 | 0x30a | (-126, 238) | (721, 6) | polvo |
| 0x914 | travessia | 23 | 0x66a4–0x66b4 | 3680 | 0x2fc | (624, 284) | (-697, -6) | patinador |
| 0x92e | gag local | 7 | 0x621c–0x622c | 2100 | 0x282, 0x30f | (-3, -5) | (-42, 15) | rato de capa |
| 0x92f | gag local | 12 | 0x6274–0x627c | 3600 | 0x2e6 | (-13, 5) | (0, 0) | T-rex (cabeça) |
| 0x930 | gag local | 4 | 0x6284–0x6284 | 1200 | 0x30d | (13, 2) | (0, 0) | robô |
| 0x931 | gag local | 12 | 0x628c–0x629c | 3600 | 0x288 | (9, 10) | (135, -136) | coelho |
| 0x932 | mastro (desce/sobe) | 23 | 0x6524–0x6544 | 4600 | 0x294 | (22, 9) | (9, -445) | boneco pixelado 2 |
| 0x933 | gag local | 28 | 0x6234–0x626c | 8400 | 0x301 | (-31, 1) | (149, -87) | rato com bazuca |
| 0x934 | gag local | 12 | 0x646c–0x648c | 2400 | 0x2fc | (20, 19) | (73, 152) | bola |
| 0x961 | pose/objeto 3 s | 1 | 0x672c–0x672c | 3000 | 0x293 | (-33, 56) | (0, 0) | mastro em cifrão |
| 0x962 | mastro (desce/sobe) | 8 | 0x654c–0x6554 | 1600 | 0x2f2 | (-107, -151) | (-2, 480) | personagem no mastro |
| 0x963 | mastro (desce/sobe) | 9 | 0x654c–0x6554 | 1800 | — | (-107, 330) | (0, -593) |  |
| 0x964 | mastro (desce/sobe) | 6 | 0x668c–0x6694 | 1200 | 0x2f2 | (-50, 23) | (26, 327) | personagem no mastro 2 |
| 0x965 | mastro (desce/sobe) | 6 | 0x668c–0x6694 | 1200 | — | (-24, 351) | (-26, -347) |  |
| 0x966 | mastro (desce/sobe) | 7 | 0x666c–0x6684 | 2400 | 0x20a, 0x28a, 0x2f2, 0x310 | (-52, -118) | (-6, 416) | personagem no mastro 3 |
| 0x9c6 | pose/objeto 3 s | 1 | 0x66b8–0x66b8 | 3000 | 0x293 | (-72, 121) | (0, 0) | mastro em gancho |
| 0x9c7 | pose/objeto 3 s | 1 | 0x66d0–0x66d0 | 3000 | 0x293 | (-18, 271) | (0, 0) | mastro com nó |
| 0x9c8 | pose/objeto 3 s | 1 | 0x6734–0x6734 | 3000 | 0x293 | (-39, 82) | (0, 0) | mastro em mola |
| 0x9c9 | mastro (desce/sobe) | 6 | 0x668c–0x6694 | 1200 | — | (-27, 351) | (-24, -474) |  |
| 0x9ca | mastro (desce/sobe) | 6 | 0x6660–0x6668 | 1200 | — | (-42, 345) | (-20, -510) |  |
| 0x9cb | mastro (desce/sobe) | 8 | 0x65b4–0x65bc | 1600 | — | (-53, 290) | (-16, -472) |  |
| 0x9cc | mastro (desce/sobe) | 8 | 0x666c–0x6684 | 3400 | 0x20a, 0x28a, 0x2f2, 0x310 | (-52, -230) | (-4, 529) | personagem no mastro 3 |
| 0x9cd | gag local | 7 | 0x66e8–0x6700 | 4400 | 0x210, 0x309 | (-43, 419) | (0, -3) | lagarta |
| 0x9ce | mastro (desce/sobe) | 8 | 0x65a4–0x65ac | 1600 | 0x2f2 | (-40, -169) | (10, 499) | personagem no mastro 6 |
| 0x9cf | mastro (desce/sobe) | 9 | 0x65a4–0x65ac | 1800 | — | (-30, 330) | (-10, -591) |  |
| 0x9d0 | mastro (desce/sobe) | 8 | 0x65b4–0x65bc | 1600 | 0x2f2 | (-69, -167) | (16, 457) | personagem no mastro 5 |
| 0xa28 | pose/objeto 3 s | 1 | 0x66d0–0x66d0 | 3000 | 0x293 | (-18, 253) | (0, 0) | mastro com nó |
| 0xa29 | pose/objeto 3 s | 1 | 0x6734–0x6734 | 3000 | 0x293 | (-39, 256) | (0, 0) | mastro em mola |
| 0xa2a | pose/objeto 3 s | 1 | 0x672c–0x672c | 3000 | 0x293 | (-33, 258) | (0, 0) | mastro em cifrão |
| 0xa2b | pose/objeto 3 s | 1 | 0x66b8–0x66b8 | 3000 | 0x293 | (-72, 254) | (0, 0) | mastro em gancho |
| 0xa2c | mastro (desce/sobe) | 7 | 0x668c–0x6694 | 1400 | 0x2f2 | (-51, -145) | (24, 496) | personagem no mastro 2 |
| 0xa2d | mastro (desce/sobe) | 7 | 0x6660–0x6668 | 1400 | 0x2f2 | (-62, -211) | (20, 556) | personagem no mastro 4 |
| 0xa2e | mastro (desce/sobe) | 9 | 0x666c–0x6684 | 3600 | 0x20a, 0x28a, 0x2f2, 0x310 | (-52, -322) | (-4, 622) | personagem no mastro 3 |
| 0xa2f | mastro (desce/sobe) | 8 | 0x654c–0x6554 | 1600 | 0x2f2 | (-106, -152) | (-1, 482) | personagem no mastro |
| 0xa30 | mastro (desce/sobe) | 8 | 0x654c–0x6554 | 1600 | — | (-107, 329) | (1, -485) |  |
| 0xa8f | pose/objeto 3 s | 1 | 0x66b8–0x66b8 | 3000 | 0x293 | (-72, 309) | (0, 0) | mastro em gancho |
| 0xa90 | pose/objeto 3 s | 1 | 0x66d0–0x66d0 | 3000 | 0x293 | (-18, 279) | (0, 0) | mastro com nó |
| 0xa91 | pose/objeto 3 s | 1 | 0x672c–0x672c | 3000 | 0x293 | (-33, 52) | (0, 0) | mastro em cifrão |
| 0xa92 | mastro (desce/sobe) | 7 | 0x668c–0x6694 | 1400 | — | (-27, 350) | (-24, -574) |  |
| 0xa93 | mastro (desce/sobe) | 7 | 0x6660–0x6668 | 1400 | 0x2f2 | (-62, -214) | (10, 556) | personagem no mastro 4 |
| 0xa94 | mastro (desce/sobe) | 9 | 0x65b4–0x65bc | 1800 | — | (-53, 290) | (-18, -710) |  |
| 0xa95 | mastro (desce/sobe) | 9 | 0x666c–0x6684 | 3600 | 0x20a, 0x28a, 0x2f2, 0x310 | (-52, -328) | (-4, 634) | personagem no mastro 3 |
| 0xa96 | gag local | 7 | 0x66e8–0x6700 | 4400 | 0x28a, 0x309 | (-42, 420) | (6, 70) | lagarta |
| 0xa97 | mastro (desce/sobe) | 9 | 0x65a4–0x65ac | 1800 | 0x2f2 | (-40, -229) | (10, 557) | personagem no mastro 6 |
| 0xa98 | mastro (desce/sobe) | 9 | 0x65a4–0x65ac | 1800 | — | (-30, 328) | (-10, -600) |  |
| 0xa99 | mastro (desce/sobe) | 8 | 0x65b4–0x65bc | 1600 | 0x2f2 | (-69, -167) | (16, 457) | personagem no mastro 5 |
