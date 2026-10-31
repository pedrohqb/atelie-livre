# GLOB — arquivos `.PIC`

`ARTIST/ARTIST.PIC` e `WRITER/WRITER.PIC` guardam as imagens da interface
(botões, cenários, ferramentas, carimbos). São um índice simples seguido dos
dados de cada imagem. O índice é **little-endian**; o conteúdo das imagens é
`PKBM`, com campos big-endian (ver `pkbm.md`).

## Layout

```
12 bytes  "GLOBGLOBGLOB"
u16       1   (versão? sempre 1)
u16       1   (sempre 1)
u16       contagem de entradas
contagem × { u16 id, u32 offset }    offset absoluto no arquivo
...       dados das imagens
...       zeros até completar múltiplo de 512 bytes
```

As entradas estão em ordem crescente de id; os offsets também crescem. Não há
campo de tamanho: cada entrada vai até o próximo offset distinto (ou até o fim
do arquivo).

## Marcador de fim

A última entrada do índice não é uma imagem: tem o maior id (0x7D91 em
`ARTIST.PIC`, 0x7F65 em `WRITER.PIC`), aponta para o fim dos dados e o que
segue é só o preenchimento de zeros. Ela existe para delimitar o tamanho da
última imagem real. O leitor a descarta quando ela está no maior offset e só
contém zeros.

| Arquivo | Entradas no índice | Imagens | Tamanho |
|---|---|---|---|
| `ARTIST.PIC` | 2.469 | 2.468 | 3.949.568 bytes (7.714 × 512) |
| `WRITER.PIC` | 2.312 | 2.311 | 2.843.648 bytes (5.554 × 512) |

## Exemplo (`ARTIST.PIC`)

```
47 4c 4f 42 47 4c 4f 42 47 4c 4f 42   "GLOBGLOBGLOB"
01 00 01 00                           1, 1
a5 09                                 2.469 entradas
4e 02 28 3e 00 00                     id 0x024e → offset 0x3e28
4f 02 f9 3f 00 00                     id 0x024f → offset 0x3ff9
...
```

Toda imagem começa com `00 01 00 00 50 4b 42 4d` (`.. .. "PKBM"`).
