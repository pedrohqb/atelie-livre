# PKBM — imagens

Formato das imagens nos `.PIC` (ver `glob.md`) e dentro dos arquivos `CHNK`
(clip-art, molduras, animações e desenhos salvos; ver `chnk.md`). Validado em
todas as 4.779 imagens dos dois `.PIC` e nas 478 imagens embutidas nos arquivos
de `SHARED/`.

## Ordem de bytes

Os dois primeiros bytes são uma marca de ordem de bytes: um `u16` que vale 1.

| Bytes | Ordem | Onde aparece |
|---|---|---|
| `00 01` | big-endian | `.PIC`, `W-MAG100.CHK` (gravados no Mac) |
| `01 00` | little-endian | documentos e bibliotecas gravados no Windows |

A mesma convenção (`bo = 1`, lido como `0x0100` quando a ordem é a outra)
aparece no framework Kauai do 3D Movie Maker, do mesmo time da Microsoft Kids.
Todos os campos numéricos abaixo seguem a ordem indicada pela marca.

## Cabeçalho (42 bytes)

| Offset | Tipo | Conteúdo |
|---|---|---|
| 0 | u16 | marca de ordem de bytes (1) |
| 2 | u16 | 0 (no Kauai, "osk": sistema que gravou) |
| 4 | 4 bytes | `"PKBM"` (sempre nesta ordem de caracteres) |
| 8 | u16 | desconhecido: 0 em quase tudo; valores diversos em algumas pinturas de 8 bits |
| 10 | u16 | compressão: 0 = sem compressão, 1 = PackBits |
| 12 | u16 | 1 (versão?) |
| 14 | u16 | 42 (tamanho do cabeçalho) |
| 16 | u32 | nos `.PIC`: tamanho dos pixels descomprimidos; nos `CHNK` do Windows: offset do fim dos pixels (offset dos pixels + tamanho). O leitor não depende deste campo. |
| 20 | u8 | desconhecido (nos `.PIC`: 1 nas imagens de 1 bit, 0 nas de 4) |
| 21 | u8 | desconhecido (0, 1, 4, 5, 12, 13 observados) |
| 22 | u16 | largura em pixels |
| 24 | u16 | altura em pixels |
| 26 | u16 | profundidade: 1, 4 ou 8 bits por pixel |
| 28 | u16 | número de cores da paleta embutida (0 ou 256) |
| 30 | u16 | offset da paleta embutida (42, inclusive quando não há paleta) |
| 32 | 6 bytes | desconhecido (zeros, ou valores diversos em algumas pinturas de 8 bits) |
| 38 | u32 | offset dos pixels a partir do início do cabeçalho (42, ou 1.578 com paleta) |

## Paleta embutida

Quando o campo 28 é diferente de zero: `contagem × { u16 r, u16 g, u16 b }`
no offset do campo 30, com componentes de 16 bits como o `RGBColor` do Mac
(ex.: `0xff00`). A cor de 8 bits é o byte alto. Só foi vista em imagens de
8 bits (as miniaturas dos desenhos salvos).

## Pixels

- Linhas de cima para baixo; pixels empacotados com o bit mais significativo
  primeiro (1 bit: 8 pixels por byte; 4 bits: 2 por byte; 8 bits: 1 por byte).
- Cada linha ocupa `bytes_por_linha = ceil(largura × profundidade / 8)`
  arredondado para cima até um número **par** (alinhamento de 16 bits, como o
  `rowBytes` do Mac).
- Tamanho descomprimido = `bytes_por_linha × altura`.
- Com compressão, os pixels são um fluxo PackBits contínuo (não separado por
  linha) que se lê até completar o tamanho descomprimido. Em todas as imagens
  dos `.PIC`, o fluxo termina exatamente no fim da entrada.

PackBits: lê um byte de controle `n`. Se `n < 128`, copia os `n + 1` bytes
seguintes. Se `n > 128`, repete o byte seguinte `257 − n` vezes. `n = 128` é
ignorado.

## Cores

| Profundidade | Paleta, em ordem de preferência |
|---|---|
| 1 bit | 0 = branco, 1 = preto |
| 4 bits | `CTBL` de 16 cores do próprio arquivo `CHNK` (animações); senão `LOGPALETTE` 4 do executável **reordenada para a ordem do Mac** (ver abaixo) |
| 8 bits | paleta embutida na imagem; senão `CTBL` de 256 cores do próprio arquivo `CHNK` (desenhos salvos); senão `LOGPALETTE` 8 do executável |

Os desenhos salvos (`.MAG`, `.PTM`) guardam a paleta da pintura no chunk
`CTBL` (ver `chnk.md`); ela é idêntica à paleta embutida na miniatura (`THMB`)
do mesmo documento e diferente da `LOGPALETTE` do executável. A animação
`A-MAG100.ANI` também traz uma `CTBL` própria, de 16 cores, usada pelas suas
imagens de 4 bits. As imagens dos `.PIC` usam as `LOGPALETTE` do executável;
nelas, as 16 primeiras cores da paleta de 256 coincidem com a de 16.

**Ordem das 16 cores.** As imagens de 4 bits vieram do Mac, cuja tabela de 16
cores tem o branco no índice 0 e o preto no 15. A `LOGPALETTE` de 16 cores do
Windows tem o branco no 0 e o preto no 1. Por isso o índice `i` de uma imagem de
4 bits corresponde à entrada `LOGPALETTE[i + 1]` para `i` de 1 a 14, o índice 0 à
entrada 0 e o 15 à entrada 1. Há uma exceção: os índices 12 e 13 trocam de
lugar, de modo que 12 é o cinza claro (190), 13 o médio (140) e 14 o escuro
(100), como na tabela do Mac. Confirmado comparando com o Fine Artist original
rodando no Wine: com essa ordem o céu da tela de entrada (0x4139) sai lilás
(136, 136, 230) e a rua (0x6d01) sai idêntica ao original, pixel a pixel.

## Gravação

`pkbm::encode` grava little-endian com PackBits (repetições de 3 ou mais
bytes viram run, o resto literal, ambos até 128), campo 16 = offset dos
pixels + tamanho descomprimido (a convenção dos `CHNK` do Windows) e a
paleta embutida quando o `Bitmap` traz uma. Os bytes 20/21 são `08 00` por
padrão (os das miniaturas dos desenhos salvos); `encode_with` permite outros.

## Máscaras

O formato não tem máscara embutida. Muitas imagens coloridas têm uma imagem de
1 bit do mesmo tamanho logo em seguida (ex.: em `ARTIST.PIC`, 32141 em 4 bits
e 32144 em 1 bit), na qual a silhueta aparece em 0 (branco) e o fundo em 1
(preto), como a máscara AND do Windows. A associação entre imagem e máscara
fica a cargo da engine (provavelmente pelos recursos de comportamento).

## Exemplos

`ARTIST.PIC`, id 590 (112×37, 1 bit, comprimida):

```
00 01 00 00 50 4b 42 4d   ordem big, "PKBM"
00 00 00 01               desconhecido 0, compressão 1
00 01 00 2a               1, cabeçalho de 42
00 00 02 06               518 = 14 bytes/linha × 37
01 00                     1, 0
00 70 00 25 00 01         112 × 37, 1 bit
00 00 00 00 00 00 00 00 00 00   sem paleta
00 00 00 2a               pixels no offset 42
0d 03 ff ff 80 00 ...     PackBits: copia 14 bytes ...
```

`SAMP1.MAG` (desenho salvo), miniatura de 144×84 em 8 bits com paleta:

```
01 00 00 00 50 4b 42 4d   ordem little, "PKBM"
00 00 01 00 01 00 2a 00   0, compressão 1, 1, 42
6a 35 00 00               13.674 = 1.578 + 144 × 84
08 00 90 00 54 00 08 00   8, 0, 144 × 84, 8 bits
00 01 2a 00               256 cores no offset 42
00 00 00 00 00 00
2a 06 00 00               pixels no offset 1.578
```
