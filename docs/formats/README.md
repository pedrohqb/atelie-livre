# Formatos de dados

Documentação aberta dos formatos usados pelo Microsoft Fine Artist e pelo
Microsoft Creative Writer (versão Windows 3.1 em português, 1995), levantada
por análise dos próprios arquivos de dados. Serve para reimplementar os
leitores sem consultar o código original.

| Documento | Conteúdo |
|---|---|
| [`recursos.md`](recursos.md) | Recursos dos executáveis NE: paletas, textos (`STR#`, `STR`, `STRING`), padrões (`PAT#`), ícones, cursores, bitmaps |
| [`glob.md`](glob.md) | Índice `GLOB` dos arquivos `.PIC` |
| [`pkbm.md`](pkbm.md) | Imagens `PKBM` (1, 4 e 8 bits, PackBits, paleta embutida) |
| [`sons.md`](sons.md) | Sons WAV com ADPCM da Microsoft e as DLLs `.SND` |
| [`chnk.md`](chnk.md) | Arquivos `CHNK`: documentos salvos e bibliotecas |
| [`atag.md`](atag.md) | Roteiros de animação `ATAG` (recursos dos executáveis e adesivos animados) |
| [`mag.md`](mag.md) | Desenhos salvos do Fine Artist (`.MAG`, `.PTM`): chunks, paleta, miniaturas |
| [`what.md`](what.md) | Balões de ajuda e dos assistentes (`WHAT` dos executáveis): texto formatado, opções, dicas, glossário |
| [`animacao.md`](animacao.md) | Pasta `animacao-…/` gravada pelo Ateliê Livre (desenho animado): PNGs, `animacao.json` e GIF animado |

## Mapa dos arquivos

| Arquivo | Formato | Ver |
|---|---|---|
| `ARTIST.EXE`, `WRITER.EXE` | executável NE (Win16) | `recursos.md` |
| `ARTIST/ARTIST.PIC`, `WRITER/WRITER.PIC` | `GLOB` de imagens `PKBM` | `glob.md`, `pkbm.md` |
| `ARTIST/ARTIST.SND`, `WRITER/WRITER.SND` | DLL NE com sons WAV | `sons.md` |
| `SHARED/MSKIDS.INF` | `CHNK` (usuários) | `chnk.md` |
| `SHARED/USERS/*/*.MAG`, `*.PTM` (exemplos) e `USERS/*/` (usuários) | `CHNK` (desenhos do Fine Artist) | `mag.md`, `chnk.md` |
| `SHARED/USERS/*/*.MAX` (exemplos) e `USERS/*/` (usuários) | `CHNK` (documentos do Creative Writer) | `chnk.md` |
| `SHARED/STUFF/*.ART`, `*.BOR`, `*.ANI`, `*.CHK` | `CHNK` (clip-art, molduras, animações, checks) | `chnk.md` |
| `SHARED/STUFF/MAG100.SND`, `MAX100.SND` | `CHNK` com sons WAV | `chnk.md`, `sons.md` |
| `SHARED/STUFF/PABLO.FON` | fonte bitmap do Windows (NE) | não interpretada |
| `WRITER/SPELL.DCT`, `THES.DCT` | dicionários licenciados (INSO) | fora do escopo |
| `WRITER/PICTIMP.FLT` | filtro de importação de imagens (DLL NE) | fora do escopo |

Os arquivos que o próprio Ateliê Livre grava ao lado dos desenhos
(`*.objetos.json`, `*.fundo.png`, `*.projeto.txt`) estão em
`arquivos-do-port.md`.

## Ainda não interpretado

Os recursos de comportamento dos executáveis (`WHAT`, `SCRD`, `SWND`,
`DSPC`, `BTNW`, `TMEN`, `CSCH`, `SSHD`, `TDRL`, `SMNM`, `PATS`, `ACUR`), o
conteúdo dos chunks de objetos soltos dos documentos (`SOBJ`, `COBJ`,
`SPLE`, `CMBD`, `TEXT`, `FONT`, `FRAM`, ...) e os metarquivos `META` do clip-art. A ferramenta `atelie-tools
extract` os extrai em forma bruta, com hexdump, para a próxima etapa.
