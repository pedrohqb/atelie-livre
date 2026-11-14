# `WRITER.PIC` e `WRITER.SND` — catálogo por tela

> Levantamento da pesquisa do Creative Writer (2026). As capturas (`shots/…`),
> ferramentas (`tools/…`) e amostras (`amostras/…`) citadas ficam na pasta da
> pesquisa, fora do repositório.

Formatos: `WRITER.PIC` é um `GLOB` de imagens `PKBM` (`glob.md`, `pkbm.md`);
`WRITER.SND` é uma DLL NE com recursos `SND` (WAVE ADPCM) e um `SNDRANGE`
(`sons.md`). Os leitores do port já abrem os dois (`Assets` com
`Program::Writer`). Extração: `out/WRITER/bitmaps/<id>.png` (2311 imagens) e
`out/resources/WRITER/WRITER.SND/SND/<id>.bin` (122 sons).

Como no `ARTIST.PIC`, as figuras animadas vêm em **trincas** (colorida,
1 bit, máscara) e os fundos em **pares** (colorido 640×480 e 1 bit para o
modo de 16 cores). Folhas de contato: `shots/formats_pic_fundos.png` (todos
os 640×480) e `shots/formats_atag_figuras.png`.

## `WRITER.PIC` por faixa de ids

| Faixa | Qtde | Conteúdo | Tela |
|---|---|---|---|
| 0x0000–0x00FF, 0x0100–0x01FF | 255 + 256 | ícones de objetos (avião, formiga, balão, curativo, luva, ursinho, pássaro, bicicleta, aranha, sol, mesa, TV, privada, árvore, caminhão…) | ícones dos rolos do **Seleciona-sons** (cada som da biblioteca tem um ícone) — a confirmar com a captura do rolo de sons |
| 0x0200–0x02FD | 177 | botões da **barra de ferramentas de cima** (0x200 adesivos, 0x205 sons, 0x20A removedores, 0x20F palavras/Pablo, 0x215 ovo, 0x222 transformadores, 0x227…) e seus estados; 0x0291 faixa 68×554 (fundo do rolo suspenso) | Escritório (todas as telas de documento) |
| 0x0304–0x03FF | 59 | setas de rolagem, X, botões de balão, "JÁ" | balões e rolos |
| 0x0402–0x0495 | 64 | ícones dos balões (rosto do McZee, tique, PARA, Rex, pasta, varinha mágica) | balões (`WHAT`) |
| 0x0800–0x08FF | 187 | botões da **segunda fileira** (subferramentas): livro/página, Aplicador (0x80B), Efeitos (0x810), MIX (0x81E), FUNDO (0x822), BORDA, colunas, Alto/Largo… | barra de subferramentas |
| 0x0900–0x095B | 66 | mais subferramentas: visualizador de páginas (0x910), Agente secreto, abelha da ortografia (0x920), mão, cola | idem |
| 0x0BB8–0x0BFF | 35 | itens de menu desenhados: "Centro", "Recuar todas menos a 1ª linha", "Marcadores são ótimos para listas", botões ACHAR (0x0BE5) e OUTRA (0x0BEE) | menus suspensos das ferramentas |
| 0x0C01–0x0C65 | 58 | itens de menu: tamanhos ("Enorme", "Mínima"…), margens, colunas, nº de página, folha; 0x0C3B–0x0C41 (325×305, 323×247) e 0x0C61–0x0C63 (341×275) painéis grandes | menus e visualizador de páginas |
| 0x3C09–0x3C41 | 15 | peças da animação de dobrar o cartão; 0x3C41 = céu com nuvens 640×480 | assistente do Cartão; Entrada |
| 0x4000–0x40F9 | 81 | 0x4000 céu (1 bit); 0x4001/0x4008/0x4009 painéis 495×314; ilustrações da história "Nossa casa na árvore" (0x400B…, 169×119) com assinaturas "Maggie"/"Max" (0x4019, 0x4028) | história de exemplo (`WHAT 4000`) / "Conte mais sobre o seu mundo" |
| 0x4C00–0x4C43 | 30 | 0x4C00/0x4C01 painel 575×432, 0x4C3F–0x4C41 448×314, 0x4C43 Rex 123×309 | **Impressão** (assistente do Rex, `WHAT 4c00`) |
| 0x4E20–0x4E23 | 3 | McZee 123×309 | **Entrada/cadastro** (`WHAT 4e00`) |
| 0x6024–0x67FF | 54+105+123+81+81+84+69+9 | quadros das **animações `ATAG`** (bichos, gags, chapéus, mastro) | salas (ver `writer-atag.md`) |
| 0x6737 / 0x6800 | 2 | **tela de título** colorida / 1 bit | Título |
| 0x6801 / 0x6802 | 2 | Imaginópolis vista de fora | "Fora de Imaginópolis" / mapa |
| 0x6803 / 0x6804 | 2 | fachada do prédio | rua/entrada do prédio |
| 0x6805 / 0x6806 | 2 | sala de parede amarela pontilhada, piso roxo | sala do prédio (Saguão ~) |
| 0x6807 / 0x680A | 2 | sala verde com patamar | sala (Biblioteca ~) |
| 0x680B / 0x680C | 2 | lousa verde emoldurada (tela cheia) | quadro de avisos / Máquina de Idéias ~ |
| 0x680D / 0x6810 | 2 | parede salmão, piso de ladrilhos | sala (Escritório ~) |
| 0x6811 / 0x6812 | 2 | canto rosa, piso de tábuas | sala (Oficina de Projetos ~) |
| 0x6813 / 0x6814 | 2 | parede amarela, piso xadrez roxo | sala (Oficina de Idéias ~) |
| 0x6815–0x68FF | ~40 | demais peças das salas | salas |
| 0x690C–0x69BF | 108 | objetos das salas (carrinho, caixa, personagens 0x692B 274×271…) | salas |
| 0x7530–0x7579 | 45 | 0x7530/0x7531 painel 263×338, pincel A (0x7535), botão FEITO (0x753B), peixe, sapo… | cavaletes (Splot / lápis de cera ~) |
| 0x7724–0x7727, 0x7918–0x7937 | 3 + 23 | painéis 312×333 e 314×335; placa "Máquina de Idéias" (0x7918), tesoura, câmera, presente | **Oficina de Idéias** |
| 0x7D00–0x7DFF | 117 | símbolos dos rolos da **Máquina de Idéias** (chapéu, comida, chave, baú, âncora…, 323×183 a 158×155) | Máquina de Idéias (caça-níqueis de idéias) |
| 0x7E00–0x7EFD | 39 | crianças/personagens 143×198… | salas |
| 0x7F01–0x7F61 | 25 | **fotos da equipe** (Page, Clete, Susan, Bruce, Jeffrey, Robert…) e legenda | créditos (segredo) |

As faixas "~" estão por inferência da figura; as capturas por tela dos
outros relatórios fecham a correspondência exata (use
`tools/wine/locate.py CAPTURA out/WRITER/bitmaps`).

## `WRITER.SND`

122 recursos `SND`, ids 0x209 a 0x312 (`SNDRANGE` = `09 02 12 03`), WAVE
ADPCM mono a 11 kHz (alguns 44,1 kHz), de 0,01 s a 7,98 s (0x302).

- **99 são idênticos** (mesmo id, mesmos bytes) aos do `ARTIST.SND`: o mapa
  de uso do Fine Artist (`docs/telas/sons-das-acoes.md`) vale para eles.
- **23 só existem no Writer**: 0x20C, 0x20D, 0x20F, 0x211, 0x282, 0x284–0x297.
  Usos já identificados pelos roteiros `ATAG`: 0x20F galo, 0x211
  pterossauro, 0x282 rato de capa, 0x288 coelho/boneco, 0x28A pêndulo e
  mastro, 0x293 troca de objeto/forma do mastro (3 s), 0x294 boneco pixelado,
  0x297… (a mapear pelos logs `+winmm` das telas).

Durações (s): 0209 0,95 · 020a 0,23 · 020b 0,41 · 020c 0,58 · 020d 0,91 ·
020e 0,37 · 020f 0,91 · 0210 0,79 · 0211 0,97 · 0212 0,58 · 0217 0,28 · 021c
0,26 · 021d 0,52 · 021e 0,66 · 021f 0,30 · 0220 0,56 · 0221 0,08 · 0222 0,44 ·
0223 0,81 · 0224 0,91 · 0225 0,84 · 0226 1,00 · 0229 2,35 · 022a 1,34 · 022b
1,25 · 022c 0,86 · 022d 0,02 · 022e 0,60 · 022f 0,28 · 0230 0,01 · 0231 0,10 ·
0232 0,95 · 0233 0,60 · 0234 1,01 · 0239 0,40 · 023b 0,30 · 025f 1,40 · 0262
0,67 · 0267 0,59 · 026e 0,80 · 0281 0,63 · 0282 0,95 · 0283 1,00 · 0284 0,86 ·
0285 1,04 · 0286 0,88 · 0287 1,32 · 0288 0,47 · 0289 0,30 · 028a 0,67 · 028b
0,40 · 028c 0,37 · 028d 0,53 · 028e 0,37 · 0290 0,47 · 0291 0,47 · 0292 1,04 ·
0293 0,65 · 0294 0,28 · 0295 0,63 · 0296 0,64 · 0297 0,60 · 02bc–02bf 0,21 ·
02c0–02c5 0,05 · 02c6 1,00 · 02ce 0,21 · 02cf 0,42 · 02d2 0,12 · 02d3 0,63 ·
02d4 0,62 · 02d6 0,91 · 02d7 0,98 · 02d8 2,69 · 02dc 0,33 · 02dd 1,01 · 02e0
0,45 · 02e1 0,40 · 02e2 0,53 · 02e6 0,67 · 02e8 0,25 · 02e9 0,31 · 02ea 0,23 ·
02eb 0,33 · 02ec 0,14 · 02ed 1,16 · 02ee 0,40 · 02f1 0,70 · 02f2 1,00 · 02f3
1,07 · 02f4 0,18 · 02f5 0,33 · 02f6 0,77 · 02f7 0,88 · 02f8 0,90 · 02f9 1,02 ·
02fb 0,35 · 02fc 0,72 · 02fd 0,07 · 02fe 0,25 · 02ff 0,13 · 0300 1,00 · 0301
0,30 · 0302 7,98 · 0303 0,72 · 0304 3,20 · 0307 0,84 · 0308 0,59 · 0309 0,93 ·
030a 0,95 · 030d 0,07 · 030e 0,74 · 030f 0,20 · 0310 0,34 · 0311 0,75 · 0312
1,00.

Para identificar o som tocado numa captura do original: o log do Wine com
`WINEDEBUG=+winmm` traz `waveOutWrite … dwBufferLength: N`; compare N com o
tamanho dos dados decodificados (adaptar o casamento de sons da pesquisa para
`WRITER.SND` + `MAX100.SND`).

## Bibliotecas compartilhadas usadas pelo Writer (`SHARED/STUFF`)

| Arquivo | Uso no Writer |
|---|---|
| `MAXMAG10.ART`, `MAX100.ART`, `MAG100.ART`, `C-MAG100.ART` | rolos do Seleciona-adesivos (os mesmos `CARO`/`PICT` do Fine Artist); os `PICT` são copiados para o documento |
| `0MAX100.BOR`, `1MAX100.BOR`, `2MAX100.BOR` | os três rolos de BORDA da Preparação da página (`0MAG100.BOR` é do Fine Artist) |
| `MAX100.SND` | sons dos rolos do Seleciona-sons (copiados para o documento como `SND n`) |
| `PABLO.FON` | fonte dos balões, barra de status e menus desenhados |
| `WORKSHOP.CHK`, `B-MAG100.CHK`, `W-MAG100.CHK`, `A-MAG100.ANI`, `0MAG100.BOR` | do Fine Artist |
