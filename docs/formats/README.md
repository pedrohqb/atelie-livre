# Formatos de dados

Documentação aberta dos formatos usados pelo Microsoft Fine Artist e pelo
Microsoft Creative Writer (versão Windows 3.1 em português, 1995), levantada
por análise dos próprios arquivos de dados. Serve para reimplementar os
leitores sem consultar o código original.

| Documento | Conteúdo |
|---|---|
| [`recursos.md`](recursos.md) | Recursos dos executáveis NE: paletas, textos (`STR#`, `STR`, `STRING`), padrões (`PAT#`), ícones, cursores, bitmaps |
| [`glob.md`](glob.md) | Índice `GLOB` dos arquivos `.PIC` |
| [`maca.md`](maca.md) | Tabela palavra → figura da Maçã (Coloca-figuras) escondida no `WRITER.PIC` entre o índice e a 1ª imagem |
| [`pkbm.md`](pkbm.md) | Imagens `PKBM` (1, 4 e 8 bits, PackBits, paleta embutida) |
| [`sons.md`](sons.md) | Sons WAV com ADPCM da Microsoft e as DLLs `.SND` |
| [`chnk.md`](chnk.md) | Arquivos `CHNK`: documentos salvos e bibliotecas |
| [`atag.md`](atag.md) | Roteiros de animação `ATAG` (recursos dos executáveis e adesivos animados) |
| [`mag.md`](mag.md) | Desenhos salvos do Fine Artist (`.MAG`, `.PTM`): chunks, paleta, miniaturas |
| [`max.md`](max.md) | Documentos do Creative Writer (`.MAX`): texto, sprms de caractere/parágrafo/moldura/documento, páginas FKP, PLC, fontes, molduras, objetos, senha do Agente secreto, miniatura |
| [`writer-exe-recursos.md`](writer-exe-recursos.md) | Catálogo dos recursos do `WRITER.EXE`: `STR#`, `STRING`, `MENU`, `ACCELERATOR`, `DIALOG`, `TMEN`, `CSCH`, `SSHD`, `PATS`, `SMNM`, `TDRL`, bitmaps, cursores, mapa dos blocos `WHAT` |
| [`writer-atag.md`](writer-atag.md) | Os 155 `ATAG` do `WRITER.EXE`: travessias, gags, chapéus e poses do McZee, interface |
| [`writer-pic-snd.md`](writer-pic-snd.md) | `WRITER.PIC` por faixa de ids e tela, `WRITER.SND` (122 sons, 99 iguais aos do `ARTIST.SND`) e bibliotecas de `SHARED/STUFF` usadas pelo Writer |
| [`fontes.md`](fontes.md) | Fontes do Creative Writer: Pablo Graphos, as 5 TrueType do produto, os 7 tamanhos e o nome gravado no `FONT` |
| [`pictimp.md`](pictimp.md) | `PICTIMP.FLT` (filtro de figuras do Mac, `QD2GDI`) e os tipos de arquivo que o Writer importa |
| [`bor.md`](bor.md) | Bordas e fundos de página (`SHARED/STUFF/*.BOR`): `WBDR`, `IBGF`, molduras, miniaturas e como o Writer os desenha |
| [`wmf.md`](wmf.md) | Metarquivos do Windows (`META`): só `StretchDIB` nas figuras, vetoriais nas peças de borda; o rasterizador do port |
| [`what.md`](what.md) | Balões de ajuda e dos assistentes (`WHAT` dos executáveis): texto formatado, opções, dicas, glossário |
| [`winres-ui.md`](winres-ui.md) | Menus, atalhos e caixas de diálogo (`MENU`, `ACCELERATOR`, `DIALOG`) dos executáveis |
| [`maxprefs.md`](maxprefs.md) | Preferências do usuário (`~MAXUSER.INF`, `~MAGUSER.INF`): parede da Biblioteca, cache de fontes, entradas da Máquina de Idéias |
| [`pict.md`](pict.md) | Figuras do Mac que o Writer importa: `PICT` (bitmaps das versões 1 e 2) e MacPaint |
| [`fontes-substitutas.md`](fontes-substitutas.md) | As 5 TrueType do produto (ausentes nos dados) e a escolha de substitutos instalados (`engine::paint::ttf::substitute`) |
| [`workshop.md`](workshop.md) | Oficina de Idéias do Creative Writer (`WORKSHOP.CHK`): frases e gramática da Máquina de Idéias, regra de sorteio, lista e figuras do Quadro de Figuras, figuras-rebus |
| [`dct.md`](dct.md) | Dicionários do Creative Writer (`SPELL.DCT`, `THES.DCT`): gramática de pares, lista ortográfica, thesaurus; alternativa livre |
| [`animacao.md`](animacao.md) | Pasta `animacao-…/` gravada pelo Ateliê Livre (desenho animado): PNGs, `animacao.json` e GIF animado |

## Mapa dos arquivos

| Arquivo | Formato | Ver |
|---|---|---|
| `ARTIST.EXE`, `WRITER.EXE` | executável NE (Win16) | `recursos.md`; `writer-exe-recursos.md`, `writer-atag.md` |
| `ARTIST/ARTIST.PIC`, `WRITER/WRITER.PIC` | `GLOB` de imagens `PKBM` (+ tabela da Maçã no `WRITER.PIC`) | `glob.md`, `pkbm.md`, `writer-pic-snd.md`, `maca.md` |
| `ARTIST/ARTIST.SND`, `WRITER/WRITER.SND` | DLL NE com sons WAV | `sons.md`, `writer-pic-snd.md` |
| `SHARED/MSKIDS.INF` | `CHNK` (usuários) | `chnk.md` |
| `SHARED/USERS/*/*.MAG`, `*.PTM` (exemplos) e `USERS/*/` (usuários) | `CHNK` (desenhos do Fine Artist) | `mag.md`, `chnk.md` |
| `SHARED/USERS/*/*.MAX` (exemplos) e `USERS/*/` (usuários) | `CHNK` (documentos do Creative Writer) | `max.md`, `chnk.md` |
| `USERS/*/~MAXUSER.INF`, `~MAGUSER.INF` | `CHNK` (preferências do usuário) | `maxprefs.md` |
| `SHARED/STUFF/*.ART`, `*.BOR`, `*.ANI`, `*.CHK` | `CHNK` (clip-art, bordas e fundos, animações, checks) | `chnk.md`, `bor.md` |
| `SHARED/STUFF/WORKSHOP.CHK` | `CHNK` (Oficina de Idéias do Writer) | `workshop.md` |
| `SHARED/STUFF/MAG100.SND`, `MAX100.SND` | `CHNK` com sons WAV | `chnk.md`, `sons.md` |
| `SHARED/STUFF/PABLO.FON` | fonte bitmap do Windows (NE) | `fontes.md` |
| `WRITER/SPELL.DCT`, `THES.DCT` | dicionários licenciados (INSO; Houghton Mifflin), comprimidos com gramática de pares | `dct.md` |
| `WRITER/PICTIMP.FLT` | filtro de importação de figuras do Mac (DLL NE) | `pictimp.md` (só o necessário; o filtro não é usado pelo port) |

Os arquivos que o próprio Ateliê Livre grava ao lado dos desenhos
(`*.objetos.json`, `*.fundo.png`, `*.projeto.txt`) estão em
`arquivos-do-port.md`.

## Ainda não interpretado

Os recursos de comportamento `SWND`, `DSPC`, `BTNW` e `ACUR` do
`ARTIST.EXE` e o `TDRL`; o conteúdo dos chunks de objetos soltos dos
desenhos do Fine Artist (`SOBJ`, `COBJ`, `CMBD`); nos documentos do Creative
Writer, o cabeçalho do Splot (`SPLE`, parcial em `max.md`), alguns campos do
`DOP`/`FRAM`/`ADD ` marcados "a mapear" em `max.md` e a ligação entre as
âncoras de figuras na linha e os `PICT`; a lista de palavras e os sinônimos
dos dicionários (`SPELL.DCT`, `THES.DCT`). A ferramenta `atelie-tools
extract` extrai os recursos em forma bruta, com hexdump.
