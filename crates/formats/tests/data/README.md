# Arquivos de teste gerados

Imagens pequenas feitas para os testes de `formats::pict` e
`formats::macpaint` (`tests/pict_files.rs`). Nenhuma vem dos dados do
produto. Os pixels esperados são calculados pelas mesmas fórmulas no teste.

| Arquivo | Como foi feito | Conteúdo |
|---|---|---|
| `im-direct-70x6.pict` | PPM gerado em Python → `convert x.ppm x.pict` (ImageMagick 6.9.12) | `DirectBitsRect`, 32 bits, packType 4, 3 planos, contagens `u16` (rowBytes 280). Pixel (x, y) = (3x mod 256, 40y, 2(x+y) mod 256), exceto 20 ≤ x < 40, branco |
| `im-indexed-300x4.pict` | idem, com `-type Palette` | `PackBitsRect` 8 bits, tabela de 6 cores. Cor = PAL[(x/7 + y) mod 6], exceto 100 ≤ x < 200, PAL[2]; PAL = branco, preto, #ff0000, #008000, #0000ff, #fac80a |
| `im-argb-7x5.pict` | `convert -size 7x5 xc:white -fill red -draw "point 1,1" -fill "#0000ff" -draw "point 5,3" x.pict` | `DirectBitsRect` com 4 planos (alfa primeiro) |
| `im-indexed-7x5.pict` | a mesma figura, com `-type Palette` | `PackBitsRect` com rowBytes 7 **comprimido** (o ImageMagick grava assim, contra a especificação, e o leitor dele não relê) |
| `check.mac` | gerado em Python (cabeçalho versão 2, padrões k: byte j = (7k + 13j) mod 256, linhas em PackBits) | MacPaint 576×720: tabuleiro de casas 8×8 em x, y < 64 (preta quando (x/8 + y/8) é ímpar), diagonal x = y, última linha preta |

Os quatro primeiros e `check.mac` foram conferidos com o leitor do
ImageMagick (`convert ARQ ppm:-`), menos `im-indexed-7x5.pict`, conferido
à mão.
