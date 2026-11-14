# Figuras do Mac: `PICT` e MacPaint

O Creative Writer abre "arquivos do Mac sem extensão" (tipo 0x20 da tabela
de tipos do `WRITER.EXE`, ver `pictimp.md` na pesquisa): primeiro como
`PICT`, convertido pelo filtro `PICTIMP.FLT` (`QD2GDI`, PICT → metarquivo
do Windows), e senão como MacPaint, decodificado pelo próprio Writer
(balões `idsOpenMacPaintNot` "Não consigo ler essa figura." e
`idsEmptyMacPaint` "Está vazia! Esta figura está vazia…", bloco `WHAT 0801`).

O port não usa o filtro (código de 16 bits). Leitores próprios:
`formats::pict` e `formats::macpaint`, que devolvem uma imagem RGBA
(`winres::Image`) para virar figura do documento.

Ordem de reconhecimento sugerida: `pict::is_pict` (assinatura de versão
em posição fixa), depois `macpaint::is_macpaint`.

## `PICT`

Inteiros big-endian. Um arquivo tem 512 bytes de cabeçalho do aplicativo
(ignorados) antes da figura; figuras embutidas (área de transferência,
recursos) começam direto. `pict::header_offset` testa 512 e depois 0.

```
u16   tamanho (truncado; ignorado)
Rect  quadro: top, left, bottom, right (i16, 72 ppp)
versão 1:  11 01                 opcodes de 1 byte, sem alinhamento
versão 2:  00 11 02 ff           opcodes de 2 bytes, dados alinhados a 2
           0c 00 + 24 bytes      HeaderOp: versão -1 (caixa em ponto fixo)
                                 ou -2 (hRes, vRes, Rect de origem, 4 reservados)
opcodes… até 0x00ff
```

Com o cabeçalho -2 as coordenadas dos opcodes estão na resolução nativa e o
retângulo de origem do cabeçalho define a área; senão vale o quadro. A
imagem do leitor tem o tamanho dessa área, com fundo branco.

### Bitmaps (o que é desenhado)

| Opcode | Nome | Dados |
|---|---|---|
| 0x90 | `BitsRect` | BitMap ou PixMap, `srcRect`, `dstRect`, `mode`, linhas **sem** compressão |
| 0x91 | `BitsRgn` | idem + região de máscara (ignorada) |
| 0x98 | `PackBitsRect` | idem, linhas em PackBits |
| 0x99 | `PackBitsRgn` | idem + região |
| 0x9a | `DirectBitsRect` | `u32` baseAddr (0xff), PixMap de 16 ou 32 bits, retângulos, `mode`, linhas |
| 0x9b | `DirectBitsRgn` | idem + região |

`rowBytes` (`u16`): bit 15 ligado = **PixMap** (cor), desligado = BitMap de
1 bit (1 = cor de frente, 0 = fundo). Depois vêm `bounds` (Rect) e, no
PixMap, 36 bytes: versão, `packType`, `packSize`, `hRes`, `vRes`,
`pixelType`, `pixelSize`, `cmpCount`, `cmpSize`, `planeBytes`, `pmTable`,
reservado. Os PixMaps indexados (1, 2, 4, 8 bits) trazem uma tabela de
cores: `u32` seed, `u16` flags (0x8000 = o índice é a posição), `u16` n−1 e
n × (`u16` valor, `u16` r, g, b; byte alto = cor).

Linhas comprimidas: cada linha começa pela contagem de bytes (`u16` se
`rowBytes` > 250, senão `u8`), seguida de PackBits. Com `rowBytes` < 8 as
linhas vêm cruas pela especificação; o ImageMagick comprime mesmo assim, e
o leitor escolhe a leitura depois da qual vem um opcode plausível.

`DirectBitsRect` por `packType`:

| packType | Pixels |
|---|---|
| 1 | crus: 32 bits xRGB ou 16 bits x-5-5-5 |
| 2 | 24 bits RGB crus, sem contagens |
| 3 (padrão de 16 bits) | PackBits de **palavras** por linha |
| 4 (padrão de 32 bits) | PackBits de bytes por linha; os componentes vêm em planos (A, R, G, B com `cmpCount` 4; R, G, B com 3) |

A cópia de `srcRect` para `dstRect` escala pelo vizinho mais próximo. Modos
`srcOr` (1), `srcBic` (3) e `transparent` (36) não desenham os pixels da cor
de fundo; os demais copiam.

### Opcodes pulados

Todos os outros opcodes da versão 2 são pulados pelo tamanho (fixo, com
palavra de tamanho, com `u32` de tamanho, ou `(op >> 8) × 2` bytes nos
reservados 0x0100–0x7fff). Cores de frente/fundo (`FgColor` 0x0e e
`BkColor` 0x0f clássicas, `RGBFgCol` 0x1a e `RGBBkCol` 0x1b) são guardadas e
usadas nos BitMaps de 1 bit. Linhas, texto, retângulos, ovais, arcos,
polígonos e regiões **não** são desenhados; os opcodes ficam em
`Picture::skipped` para avisar que a importação foi parcial.

## MacPaint (`PNTG`)

```
[128 bytes MacBinary, se houver: byte 0 = 0, byte 1 = tamanho do nome (1–63), bytes 65–68 = "PNTG"]
u32        versão: 0 (sem padrões), 2 ou 3
38 × 8     padrões de preenchimento do documento
204        reservado
dados      720 linhas de 72 bytes (576 pixels de 1 bit, 1 = preto), PackBits
```

O MacPaint comprime linha a linha; o leitor trata os dados como um fluxo
contínuo (aceita gravadores que atravessam linhas), completa com branco se
os dados acabarem antes da página e dá erro se uma sequência passar do fim
da página. `MacPaint::is_empty` corresponde ao "MacPaint vazio" do
original; `ink_bounds` dá o retângulo com tinta, para importar só o desenho.
