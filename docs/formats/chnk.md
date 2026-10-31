# CHNK — documentos e bibliotecas

Formato "chunky" dos documentos salvos (`.MAG`, `.PTM` do Fine Artist; `.MAX`
do Creative Writer; os de exemplo ficam em `SHARED/USERS/` e os dos usuários
em `USERS/`), do arquivo de usuários `MSKIDS.INF` e das bibliotecas de
`SHARED/STUFF` (clip-art `.ART`, molduras `.BOR`, animações `.ANI`, "checks"
`.CHK` e bibliotecas de som `MAG100.SND`/`MAX100.SND`). Validado nos 54
arquivos `CHNK` da instalação, inclusive em 18 `CHNK` aninhados. Os desenhos
salvos estão detalhados em `mag.md`.

É parente, mas não igual, ao formato `CHN2` do framework Kauai do 3D Movie
Maker (`kauai/SRC/CHUNK.CPP`): mesma ideia de "chunks" identificados por
etiqueta de quatro letras e número, mesmo esquema de versão (atual/retroativa)
e a mesma marca de ordem de bytes nos dados; o índice é outro.

## Ordem de bytes

Os campos de versão revelam a ordem: `04 00 04 00` é little-endian (arquivos
gravados no Windows); `00 04 00 04` é big-endian (`W-MAG100.CHK`, gravado no
Mac). Todos os números do cabeçalho e do índice seguem essa ordem. As
etiquetas de quatro letras ficam sempre na ordem dos caracteres.

## Cabeçalho (32 bytes)

| Offset | Tipo | Conteúdo |
|---|---|---|
| 0 | 4 bytes | `"CHNK"` |
| 4 | 4 bytes | criador: `SPLT` (Fine Artist), `INK ` (Creative Writer), `ICCC` (bibliotecas), `CHKE` |
| 8 | u16 | versão atual (4) |
| 10 | u16 | versão mínima de leitura (4) |
| 12 | u16 | número total de registros no índice |
| 14 | u16 | desconhecido (0x0300 nos arquivos little-endian; 0x0054 em `W-MAG100.CHK`) |
| 16 | u32 | soma dos tamanhos dos registros do índice, em bytes |
| 20 | u32 | fim lógico dos dados (o arquivo é completado com zeros até múltiplo de 512) |
| 24 | 8 bytes | cabeçalho do primeiro trecho do índice (ver abaixo) |

## Índice

O índice é uma lista encadeada de trechos. Cada trecho começa com 8 bytes:

```
u16  tamanho do trecho (no primeiro, 0x200: ele ocupa o resto do bloco inicial)
u16  número de registros neste trecho
u32  offset do próximo trecho, ou 0xFFFFFFFF
```

O primeiro trecho fica no offset 24 (seus registros começam no 32 e vão até,
no máximo, o 0x200). Quando o índice cresce, os registros seguintes vão para
trechos no fim do arquivo. A soma dos registros de todos os trechos é o
número do offset 12, e a soma dos seus tamanhos é o valor do offset 16.

### Registro

```
u16   tamanho do registro (inclui este campo)
4     etiqueta do chunk
u16   número do chunk
u16   número de partes
u16   flags (4 = a parte é um arquivo CHNK aninhado)
partes × { 4 bytes etiqueta da parte, u32 offset no arquivo, u32 tamanho }
[u8 tamanho + nome em Windows-1252, completado com um zero até tamanho par]
```

Um chunk pode ter várias partes com etiquetas próprias. Exemplos:

| Chunk | Partes | Onde |
|---|---|---|
| `BDR ` (moldura) | `WBDR` (descrição) + `PKBM` (imagem) | `.BOR` |
| `PICT` (clip-art) | `META` (metarquivo WMF do Windows) + `PKBM` (prévia) | `.ART` |
| `SND ` | `PCM ` (arquivo RIFF/WAVE) | `MAG100.SND` |
| `ATAG` (animação) | `ATAG` = arquivo `CHNK` aninhado, flags 4, com uma parte `ATAG` (roteiro, ver `atag.md`) e os `PKBM` dos quadros | `.ANI` |
| `CARO`, `CSRO` | lista de números de outros chunks (carrossel) | bibliotecas |
| `PKBM`, `THMB` | `PKBM` | documentos |

Exemplo (`2MAX100.BOR`, primeiro registro, 46 bytes):

```
2e 00                         46 bytes
42 44 52 20  06 00            "BDR ", número 6
02 00  00 00                  2 partes, flags 0
57 42 44 52  00 02 00 00  24 01 00 00    "WBDR" em 0x200, 292 bytes
50 4b 42 4d  24 03 00 00  fe 02 00 00    "PKBM" em 0x324, 766 bytes
09 "Checkered"
```

## Conteúdo das partes

- Imagens: `PKBM` (ver `pkbm.md`). Nas bibliotecas, a etiqueta dentro do
  cabeçalho da imagem aparece invertida (`MBKP`), porque foi gravada como
  número de 32 bits little-endian; nos documentos aparece `PKBM`.
- Sons: arquivos RIFF/WAVE completos (ver `sons.md`).
- `META`: metarquivo do Windows (WMF) precedido de um cabeçalho próprio
  iniciado por `"META"`; ainda não interpretado.
- `CHNK` aninhado: um arquivo completo dentro da parte, com seus próprios
  offsets relativos ao início da parte.
- `CTBL`: paleta do arquivo. `u16` marca de ordem de bytes (1), `u16`,
  4 bytes (etiqueta variável, sem uso aparente), `u16` contagem, depois
  contagem × { `u16` r, `u16` g, `u16` b } com o byte alto como cor. Tem 256
  cores nos desenhos salvos e 16 em `A-MAG100.ANI`; vale para as imagens
  `PKBM` do arquivo que não trazem paleta própria (ver `pkbm.md`).
- Demais etiquetas (`SOBJ`, `COBJ`, `TEXT`, `FONT`, `FRAM`, ...): ainda
  não interpretadas.

| Arquivo | Registros | Imagens | Sons | `CHNK` aninhados |
|---|---|---|---|---|
| `MAG100.ART` | 128 | 128 | — | — |
| `A-MAG100.ANI` | 34 | 169 (contando as aninhadas) | — | 16 |
| `W-MAG100.CHK` | 78 | 64 | — | — |
| `0MAG100.BOR` | 33 | 11 | — | — |
| `MAG100.SND` | 54 | — | 50 | — |
| `MAX100.SND` | 67 | — | 63 | — |

## Gravação

`chnk::write` produz arquivos little-endian 4/4 no layout dos documentos
do Windows: dados das partes em sequência a partir do offset 0x200, sem
alinhamento entre elas; índice no resto do primeiro bloco (cabem 20 registros
de 24 bytes); se sobrarem registros, um segundo trecho após os dados, num
offset múltiplo de 512; arquivo completado com zeros até múltiplo de 512. O
campo 20 é o fim dos dados quando o índice cabe no primeiro bloco e o tamanho
total do arquivo quando há segundo trecho, como nos exemplos (`SAMP6.MAG`:
35268 num arquivo de 35328; `SAMP12.MAG`: 62464 = tamanho).

## Espaço livre

Os arquivos podem conter restos de dados antigos fora de qualquer parte
indexada (ex.: uma imagem truncada no fim de `SAMP12.MAG`, logo antes do
segundo trecho do índice). Só o que o índice aponta é conteúdo válido.
