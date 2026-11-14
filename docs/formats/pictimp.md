# `PICTIMP.FLT` — filtro de importação de figuras do Mac

> Levantamento da pesquisa do Creative Writer (2026). As capturas (`shots/…`),
> ferramentas (`tools/…`) e amostras (`amostras/…`) citadas ficam na pasta da
> pesquisa, fora do repositório.

`WRITER/PICTIMP.FLT` (57 600 bytes) é uma DLL NE de 16 bits: **"QuickDraw
Graphic Import Filter" versão 2.2**, produto "Microsoft Graphic Filters" 2.0,
© 1992–1993 Microsoft — o filtro de **imagens PICT do Macintosh** dos
programas do Office/Works da época.

## Interface

| Nome exportado | Ordinal | Uso |
|---|---|---|
| `GetFilterInfo` | 1 | API padrão de filtros gráficos do Office |
| `ImportGr` | 2 | importa um arquivo PICT inteiro |
| `GetFilterPref` | 3 | preferências |
| `ImportEmbeddedGr` | 4 | importa um PICT embutido |
| `QD2GDI` | 5 | **converte QuickDraw (PICT) em metarquivo GDI (WMF)** |
| `StatusProc` | 6 | progresso |
| `EnumFontFunc` | 7 | mapeamento de fontes |
| `WEP`, `___EXPORTEDSTUB` | 8, 9 | rotinas de DLL |

Importa `KERNEL`, `GDI` e `USER`. Recursos: um diálogo sem título (0x3:
"Importing ", "Copyright © 1992-1993 Microsoft Corp.", "QuickDraw Graphic
Import - Version 2.2"), a string 20 "Importing " e `VERSION`. Traz uma
tabela de nomes de fontes do Mac → Windows (Chicago, New York, Geneva,
Monaco, Venice, London, Athens, San Francisco, Toronto, Los Angeles, Zapf
Dingbats, Bookman, Helvetica Narrow, Palatino, Zapf Chancery, Helvetica,
Courier, Symbol, Mobile, Avant Garde, New Century Schlbk, MT Extra, Fences,
Monotype Sorts…).

## Como o Writer usa

O `WRITER.EXE` carrega a DLL pelo nome `"pictimp"` e só pega o ponto de
entrada **`QD2GDI`** (strings no segmento de dados em 0xB95F7:
`07 "pictimp" 00 "QD2GDI" 00 "QD2GDI"`): recebe os dados de um PICT e devolve
um WMF, que vira a parte `META` de um `PICT` no documento.

Quando é usado: na busca de figuras do **Cachorrinho** ("Achar outras
figuras", `WHAT 0fa1[9]`, `[11]`) e ao abrir documentos/figuras gravados no
Mac. A tabela de tipos de arquivo do Writer (código em 0x74ADF) é:

| Extensão | Tipo | O que é |
|---|---|---|
| `MAX` | 0x01 | documento do Creative Writer |
| `DOC`, `WPS` | 0x02 | documentos do Word/Works (texto) |
| `MAG` | 0x04 | desenho do Fine Artist (vira figura) |
| `WMF`, `BMP`, `DIB` | 0x08 | figura do Windows |
| `WAV` | 0x10 | som |
| `....` (qualquer) | 0x20 | arquivo do Mac sem extensão: PICT (via `QD2GDI`) ou MacPaint (decodificado pelo próprio Writer: `idsOpenMacPaintNot`, `idsEmptyMacPaint` em `WHAT 0801`) |
| `TXT` | 0x40 | texto |

## No port

Não há o que reaproveitar do filtro (código nativo 16 bits). Para
compatibilidade basta: BMP/DIB (já há decodificador de DIB em
`formats::winres`), WMF de um `StretchDIB` (`formats::wmf::meta_dib`), MAG
(`formats::mag`), WAV (`formats::wave`), TXT; PICT e MacPaint são
opcionais (um leitor de PICT v2 com `PackBitsRect`/`DirectBitsRect` e de
MacPaint — 576×720, 1 bit, PackBits após 512 bytes de cabeçalho — cobrem os
arquivos de exemplo da época). Prioridade baixa.
