# Recursos dos executáveis

Os executáveis (`ARTIST.EXE`, `WRITER.EXE`) e as DLLs de som (`ARTIST.SND`,
`WRITER.SND`) estão no formato NE do Windows 3.x. Além dos recursos padrão do
Windows (ícones, cursores, diálogos, menus, strings, versão), eles trazem
recursos com tipos por nome em estilo Mac clássico. Todos os campos descritos
aqui são **little-endian**.

Observação: o NE arredonda o tamanho de cada recurso para o alinhamento da
tabela (o "shift", 16 bytes nestes arquivos). Os bytes de um recurso podem,
portanto, terminar com preenchimento; cada formato abaixo usa seus próprios
contadores e ignora o resto.

Ids: nos exemplos, o id aparece sem o bit 0x8000 (que no NE só indica "id
numérico"). Ex.: o `STR#` gravado como 0x8206 é o id 0x206.

## Inventário (versão em português, 1995)

| Tipo | ARTIST.EXE | WRITER.EXE |
|---|---|---|
| ICON / GROUP_ICON | 48 / 16 | 6 / 2 |
| CURSOR / GROUP_CURSOR | 90 / 90 | 35 / 35 |
| BITMAP | — | 6 |
| LOGPALETTE | 2 | 2 |
| STR# / STR | 17 / 3 | 14 / — |
| PAT# / PATS | 1 / 1 | 1 / 1 |
| WHAT | 34 | 32 |
| ATAG | 162 | 155 |
| SCRD | 14 | 14 |
| SWND / DSPC / BTNW | 36 / 27 / 37 | — |
| TMEN | 3 | 2 |
| CSCH, SSHD, TDRL, SMNM, ACUR | sim | sim (exceto ACUR) |

`ARTIST.SND`: 131 recursos `SND` e um `SNDRANGE`. `WRITER.SND`: 122 `SND` e um
`SNDRANGE`. Ver `sons.md`.

Os tipos `WHAT`, `ATAG`, `SCRD`, `SWND`, `DSPC`, `BTNW`, `TMEN`, `CSCH`,
`SSHD`, `TDRL`, `SMNM`, `PATS` e `ACUR` ainda não foram interpretados; a
ferramenta os extrai em forma bruta.

## LOGPALETTE

Estrutura `LOGPALETTE` do Windows, guardada como recurso de tipo nomeado.

```
u16  versão      sempre 0x0300
u16  contagem
contagem × { u8 r, u8 g, u8 b, u8 flags }
```

Os dois executáveis têm uma paleta de 16 cores (id 4) e uma de 256 (id 8).
Início da paleta 4 de `ARTIST.EXE`:

```
00 03 10 00  ff ff ff 00  00 00 00 00  ff ee 4c 00  ff 8c 32 00 ...
versão 0x300, 16 cores: branco, preto, amarelo, laranja...
```

## STR# (lista de textos)

```
u16  contagem
contagem × { u8 tamanho, tamanho bytes de texto, u8 0x00 }
```

Diferente do `STR#` do Mac, a contagem é little-endian e cada string Pascal é
seguida de um 0x00. O texto está em Windows-1252. Exemplo (`ARTIST.EXE`,
`STR#` 0x206, nomes das fontes):

```
07 00  0d "Pablo Graphos" 00  0b "Lucida Sans" 00  12 "Lucida Handwriting" 00 ...
```

## STR (texto único)

Texto Windows-1252 terminado em 0x00. Exemplo (`ARTIST.EXE`, `STR` 0x1005):
`45 75 20 6e e3 6f 20 70 6f 73 73 6f ...` = "Eu não posso salvar o seu arquivo...".

## PAT# (padrões de preenchimento 8×8)

```
u16  contagem
contagem × 8 × u16   uma palavra por linha; os 8 pixels ficam no byte baixo
```

Cada linha ocupa 16 bits, como as linhas de um bitmap monocromático do Windows
(alinhadas em palavra); o byte alto é preenchimento (zero nos dados
observados). `ARTIST.EXE` tem 43 padrões e `WRITER.EXE`, 18. Exemplo
(`ARTIST.EXE`):

```
2b 00                                               43 padrões
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00     padrão 0: vazio
00 00 44 00 00 00 11 00 00 00 44 00 00 00 11 00     padrão 1: 00 44 00 11 00 44 00 11
22 00 88 00 22 00 88 00 22 00 88 00 22 00 88 00     padrão 2: 22 88 22 88 ...
55 00 aa 00 55 00 aa 00 55 00 aa 00 55 00 aa 00     padrão 3: xadrez 55 aa ...
```
