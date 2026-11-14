# Bordas e fundos de página (`SHARED/STUFF/*.BOR`)

Os `.BOR` guardam as **bordas** (rolos BORDA da Preparação da página do
Creative Writer) e os **fundos** de página (rolo FUNDO do Writer e fundos da
máquina de impressão do Fine Artist). São arquivos [`CHNK`](chnk.md)
little-endian de criador `ICCC`, versão 2. Leitor: `formats::bor`;
desenho: `formats::wmf_draw` (ver [`wmf.md`](wmf.md)) e
`engine::writer::borders`.

| arquivo | bordas | fundos |
|---|---|---|
| `0MAX100.BOR` | 9 (rolo BORDA 1) | 12 retrato (rolo FUNDO, nesta ordem) + 3 paisagem |
| `1MAX100.BOR` | 8 (rolo BORDA 2) | — |
| `2MAX100.BOR` | 7 (rolo BORDA 3) | — |
| `0MAG100.BOR` | — | 11 paisagem (máquina de impressão do Fine Artist) |

A ordem dos rolos é a ordem dos chunks no arquivo. Inventário com nomes e
ids: `cw/report/projects-bordas-fundos.md`; prancha das miniaturas:
`cw/shots/projects_bor_miniaturas.png`.

## Chunks

| chunk | partes | conteúdo |
|---|---|---|
| `BDR n` (nome = o da borda) | `WBDR`, `PKBM` | borda e miniatura 80×72 |
| `IBGF n` (nome = o do fundo) | `IBGF`, `PKBM` | fundo e miniatura (54×72 retrato, 80×60 paisagem) |
| `PICT n` | `META` | figura: metarquivo com um `StretchDIB` |
| `FRAM n` | `FRAM` | molduras do fundo |

### `WBDR`

```
u16  1                     versão
u16  espessura (pt)        16, 24, 30, 32 ou 36
u16  lado lógico dos cantos
u16  largura lógica das peças de lado
u16  altura lógica das peças de lado
8×u8 mapa: peça de cada posição, na ordem
     sup-esq, topo, sup-dir, direita, inf-dir, base, inf-esq, esquerda
u16  n peças
n×u32 tamanhos
n × META                    "META", i16 x, i16 y, u16 largura, u16 altura, WMF
```

Exemplos de mapa: Paw `00×8` (uma peça), Party/Circles `00 01 00 01 …`
(canto e lado), Tribe `00 01 00 02 00 03 00 04` (um lado diferente em cada
direção), Vine `00 … 07` (oito peças; os lados medem 715 × 480 e
480 × 715). Depois do registro `EOF` de cada WMF há alguns bytes de lixo
(contados no tamanho). As peças são **WMF vetoriais** (polígonos, elipses,
retângulos, arcos, canetas e pincéis; ver `wmf.md`); a janela lógica de cada
peça é `(0, 0)–(largura, altura)` do cabeçalho `META`.

### `IBGF` (22 bytes)

| bytes | conteúdo |
|---|---|
| 0–3 | `"PICT"` (ou zeros: sem figura própria) |
| 4–5 | `u16` id do `PICT` |
| 6–7 | `ff 00` paisagem, `00 00` retrato |
| 8–11 | zeros |
| 12–15 | `i16 dx, dy`: posição da arte na página, em pontos |
| 16–19 | `"FRAM"` (ou zeros) |
| 20–21 | `u16` id do `FRAM` |

### `FRAM`

Mesmo formato do chunk `FRAM` dos `.MAX` (`u16 n` + `n × (u8 tamanho,
grpprl)`, sprms em [`max.md`](max.md)). Tipos (`0x28`) nos fundos:

| tipo | papel |
|---|---|
| 6 | **obstáculo** ao texto, em todas as páginas; `0x29`: 1 = texto só à esquerda (tira direita), 2 = só à direita (tira esquerda), 3 = nenhum texto ao lado (rolos de cima e de baixo); `0x30`/`0x31` = 72 |
| 3 | figura (`0x35 → PICT n`) desenhada no retângulo da moldura — é de onde vem a arte dos fundos sem `PICT` próprio (Imaginopolis, Bear, Robots, Western) e de parte do Dinos e do PaintPalette |
| 4 | moldura de página (T-shirt) |

## Miniaturas `PKBM`

As de borda (80×72, 1 bit; Hearts, Ladybugs, Trees e Pencils em 4 bits com
o branco no índice 0 e uma paleta própria) mostram a borda ladrilhada com
o mesmo arranjo da página: **cantos quadrados** e peças de lado esticadas
entre eles (`formats::bor::tile`). O número de peças varia de miniatura para
miniatura (3×3 células na maioria; 4×4 em Seattle, Stars, Hearts…; 5×5 em
Clocks e Flowers) e a espessura ora vem da altura, ora da largura. Com o
arranjo certo, o desenho do port bate com elas: Checkered 100 %, Tribe
99 %, coloridas 94–96 %, média 90 % (teste `real_bor.rs`; o resto é
deslocamento de 1 pixel da ferramenta que as fez).

As de fundo são a página inteira reduzida mantendo o preto (`BLACKONWHITE`
do GDI) com um contorno de 1 pixel; o desenho do port reduzido assim bate
80–98 % (média 89 %).

## Como o Writer desenha

Medido na Visão normal (1 pt = 1 pixel; folha em (22, 107) da tela) com
Tribe, Paw, Scroll e Marble (`cw/report/office-complemento.md` §5,
comparação em `cw/shots/bor_compare.png`, gerada por
`cargo run -p engine --example bor_compare`).

### Borda

1. Faixa nominal `E = 1,5 × espessura`. O `DOP` grava `0x47` = espessura + 1
   e `0x48` = `E` + 1 (`Border::dop_widths`).
2. A faixa fica **fora** do texto e não o empurra: o lado de dentro dela é o
   retângulo do texto alargado em **5 pt na horizontal e 1 pt na vertical**
   (página Alto, margens de 72: lado de dentro em x 67..528, y 71..721).
3. Em cada direção cabem `n = round(lado / E)` peças (com a proporção da
   peça, para a Vine), esticadas para encher o lado exatamente. A espessura
   desenhada é a menor das duas medidas resultantes e os cantos são
   quadrados dessa espessura, por fora dos cantos do retângulo interno.
   Tribe (24 pt): 13 × 18 peças, faixa de 461/13 = 35,46 pt → cantos em
   x 54..88 e y 143..177 da tela, como no original. Paw (32 pt): 10 × 14,
   faixa de 46,1 pt → 12 patas de x 43 a 595.
4. Sem rotação: cada lado usa a sua peça do mapa.
5. Se a faixa não cabe na folha (margem estreita), o original avisa
   `0fa5[11]` (`BorderLayout::fits`).

Concordância com as capturas fora da área de texto: Tribe 97 %, Paw 95 %
(as patas saem 1–2 px mais baixas: o original arredonda as peças para
pixels inteiros).

### Fundo

1. A arte do `PICT` vai em tamanho natural, com a **origem da janela WMF**
   (o `x, y` do `META`) em `(dx, dy)` pt da página: Scroll em (10, 17).
   Somar as coordenadas lógicas a `(dx, dy)` dá resultado pior em todas as
   miniaturas em que as duas leituras diferem (Dinos, McZee heads,
   Paint & Palette).
2. Depois, as figuras das molduras de tipo 3 nos seus retângulos.
3. As molduras de tipo 6 são obstáculos: o texto corre em volta delas como
   em volta de um adesivo (Scroll: texto de (108, 245) até x ≈ 500).
4. Cartão e faixa não aceitam fundo (`0805[32]`).

Concordância: Scroll 99 %, Marble 98 %. Na captura do Marble
(`projects_131b`, modo projeto) o original não mostra as bolhas a menos de
≈ 18 pt da beira da folha; no Escritório (Scroll) não há esse recorte. Fica
em aberto se o modo projeto recorta o fundo na área imprimível.

## Cópia no documento

O `.MAX` guarda cópias do que usa (`BORDA_PA.MAX`, `FUNDO_SC.MAX`):

- borda: `BDR n` (`WBDR` + `PKBM`), apontado por `DOP 0x58`/`0x5a`;
- fundo: `IBGF n` (que aponta `PICT` e `FRAM` do próprio documento),
  `PICT n` e `FRAM m` com `m ≠ 0`, apontado por `DOP 0x56 "IBGF" n`. As
  molduras copiadas ganham `0x27 = ffff` (todas as páginas) e `0x2b = ff`.

`engine::writer::borders::PageArt::from_document` lê as duas cópias.
