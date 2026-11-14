# Creative Writer — fontes do documento

Como o Escritório do Creative Writer mede e desenha o texto do documento, e
como o port reproduz isso (`crates/engine/src/writer/fonts.rs`). Levantamento
anterior: `office-palavra.md` §5–6 e `formats/fontes.md` da pesquisa do
Writer.

## Famílias

O Seleciona-letras oferece as famílias do `STR#` 0x206: **Pablo Graphos**
(bitmap, `SHARED/STUFF/PABLO.FON`), **Lucida Sans** (padrão dos documentos,
`STR#` 0x800 [1]), **Lucida Handwriting**, **Lucida Blackletter**, **Impact**
e **Stencil**. As cinco TrueType **estão nos disquetes de instalação** (no
disco 3 do Creative Writer e no disco 1 do Fine Artist), comprimidas no
formato KWAJ (método 3, LZ+Huffman; o `libmspack` expande):

| Arquivo | Família | Faces |
|---|---|---|
| `LSANS.TT_` | Lucida Sans | só a normal |
| `LHANDW.TT_` | Lucida Handwriting | só a inclinada (é uma cursiva; `post.italicAngle` −11,25°) |
| `LBLACK.TT_` | Lucida Blackletter | só a normal |
| `IMPACT.TT_` | Impact | só a normal |
| `STENCIL.TT_` | Stencil | só a normal; as minúsculas usam o mesmo glifo das maiúsculas |

Não há faces negritas nem inclinadas: o negrito e o inclinado do
Seleciona-estilos são simulados pelo GDI. Os `.FI_` são os `.FOT` (registro
das fontes no Windows 3.1). `LHANDW` e `LBLACK` têm o `loca` com entradas
zeradas além de `numGlyphs + 1` (o `fontTools` recusa; o `ttf-parser`
aceita).

A lista do Seleciona-letras mostra 5 linhas e fica na ordem de uso mais
recente (a fonte atual em cima, com ✓). A Pablo Graphos começa fora da vista,
e rolar a lista derruba o original no Wine.

## Como foi medido

Prefixo Wine próprio com as cinco TrueType expandidas em `C:\WINDOWS\FONTS`
e registradas em `HKLM\…\Fonts`; sete parágrafos "HHxg MMMM iiii", um em cada
tamanho (Dimensiona-palavras + pincel), e o documento inteiro passado para
cada família pelo Aplicador. A altura da linha é a do ponto de inserção
(achado pela diferença entre quadros do cursor piscando); a base, o fundo do
'H'. O Writer no Wine com as fontes instaladas desenha em Lucida Sans; sem
elas, cai numa substituta do Wine (Liberation Sans/serifada).

## Tamanhos

Visão normal: **1 ponto = 1 pixel** (pixels por em = pontos). A altura do 'H'
da Lucida Sans (0,7227 em) foi 35, 26, 18, 13, 10, 9 e 7 px nos sete
tamanhos; a largura de "HHxg MMMM iiii" bate com os avanços do `hmtx`
arredondados **por glifo** (sem `hdmx` e sem kerning): 370, 276, 190, 141,
108, 90 e 74 px de tinta.

| Tamanho | Gigante | Enorme | Grande | Média | Pequena | **Mínima** | Micro |
|---|---|---|---|---|---|---|---|
| Pontos (sprm `0x0f`) | 48 | 36 | 24 | 18 | 14 | **12** | 9 |

## Métricas de linha

A linha mede **`tmAscent + tmDescent + tmExternalLeading`** do GDI; a base
fica `tmAscent` abaixo do topo e a entrelinha externa vai embaixo. Linhas de
tamanhos diferentes se empilham sem sobreposição (o 1º parágrafo começa no
topo da área de texto, y = 179 na tela com a página em cima). O GDI tira
`tmAscent`/`tmDescent` da tabela `VDMX` quando a fonte tem o tamanho nela,
senão de `usWinAscent`/`usWinDescent` arredondados (26.6, meio para cima); a
entrelinha externa é `max(0, hhea.lineGap − ((winAscent + winDescent) −
(hhea.ascender − hhea.descender)))`, arredondada. As alturas abaixo foram
**todas medidas** e coincidem com essa regra.

Ascendente + descendente + entrelinha (= altura da linha), em pixels:

| Família | Gigante 48 | Enorme 36 | Grande 24 | Média 18 | Pequena 14 | Mínima 12 | Micro 9 |
|---|---|---|---|---|---|---|---|
| Lucida Sans | 45+10+2 = 57 | 33+7+1 = 41 | 22+5+1 = 28 | 17+4+1 = 22 | 13+3+1 = 17 | 12+3+0 = 15 | 10+3+0 = 13 |
| Lucida Handwriting | 47+16+5 = 68 | 36+12+3 = 51 | 23+8+2 = 33 | 17+6+2 = 25 | 14+5+1 = 20 | 12+4+1 = 17 | 10+4+1 = 15 |
| Lucida Blackletter | 47+10+5 = 62 | 36+8+3 = 47 | 24+5+2 = 31 | 17+4+2 = 23 ¹ | 14+3+1 = 18 | 13+3+1 = 17 | 11+3+1 = 15 |
| Impact | 50+10+0 = 60 | 36+8+0 = 44 | 24+5+0 = 29 | 18+4+0 = 22 | 15+3+0 = 18 | 13+3+0 = 16 | 11+2+0 = 13 |
| Stencil ² | 44+13+0 = 57 | 33+10+0 = 43 | 22+7+0 = 29 | 16+5+0 = 21 | 13+4+0 = 17 | 11+3+0 = 14 | 8+2+0 = 10 |
| Pablo Graphos ³ | 51+12+6 = 69 | 36+12+4 = 52 | 22+4+2 = 28 | 18+6+2 = 26 | 11+2+1 = 14 | 11+2+1 = 14 | 9+3+1 = 13 |

¹ 18 pontos não está no `VDMX` da Lucida Blackletter: vale a fórmula.
² A Stencil não tem `VDMX`.
³ Não medida (ver Pablo Graphos abaixo).

Para tamanhos fora da lista (documentos do Mac), o port usa a fórmula com os
valores das fontes (em unidades de 2048 por em):

| Família | winAscent | winDescent | entrelinha | sublinhado (posição, espessura) |
|---|---|---|---|---|
| Lucida Sans | 1900 | 432 | 80 | −200, 100 |
| Lucida Handwriting | 1964 | 666 | 195 | −200, 100 |
| Lucida Blackletter | 1962 | 445 | 197 | −200, 100 |
| Impact | 2066 | 432 | 0 | −205, 102 |
| Stencil | 1860 | 567 | 0 | −275, 126 |

## Estilos (simulados)

Medidos na Lucida Sans, nos sete tamanhos:

- **Negrito**: +1 px no avanço de cada caractere em qualquer tamanho
  ("HHxg MMMM iiii" 14 px mais largo); a altura da linha não muda.
- **Inclinado**: os avanços e a linha não mudam; a letra inclina ≈ 0,24 px
  por linha (8 px em 33 linhas no 'H' de 48 pontos). A Lucida Handwriting já
  é inclinada: o GDI não inclina de novo.
- **Sublinhado**: espessura `max(1, arred(espessura × pontos / em))`, topo em
  `arred(−posição × pontos / em) − espessura / 2` pixels abaixo da base:
  Gigante 4 (2 linhas), Enorme 3 (2), Grande 2, Média 2, Pequena 1, Mínima 1,
  Micro 1. O pincel só sublinha as palavras tocadas: os espaços entre elas
  ficam sem traço.

## Pablo Graphos

`PABLO.FON` tem três fontes de bitmap (6, 8 e 16 pontos): altura de célula
12, 13 e 21, ascendente 9, 11 e 17, entrelinha interna 3, 2 e 5 (altura de
caractere 9, 11 e 16), entrelinha externa 1, 1 e 2. Para outros tamanhos o
GDI amplia uma delas por um fator inteiro (até 8×). Isto **não pôde ser
medido**: o Wine não amplia fontes de bitmap (abrindo um documento cuja
tabela `FONT` diz "Pablo Graphos", o texto sai na Liberation Sans). O port
segue os pesos do mapeador de fontes do Windows 3.1 (150 por pixel de
diferença de altura, +600 se passar do pedido, +50 por ampliar e +20 por
fator), o que dá: Gigante = 16 pt × 3, Enorme = 6 pt × 4, Grande = 8 pt × 2,
Média = 6 pt × 2, Pequena e Mínima = 8 pt, Micro = 6 pt. Avanço = largura do
glifo × fator (+1 no negrito); altura, ascendente e entrelinha externa
multiplicadas pelo fator.

## Substitutas no port

As TrueType do produto não vão com o port. Cada família é resolvida, em
ordem, para:

1. a **original instalada** no sistema (o usuário pode expandir os `.TT_`
   dos seus disquetes para `~/.local/share/fonts`): usada sem ajuste — os
   avanços saem idênticos aos do original;
2. a **substituta embutida** (`assets/fonts/`, recurso `embedded-fonts` do
   crate `engine`, ligado por padrão; licenças em `assets/fonts/README.md`);
3. uma substituta **instalada no sistema** (lista abaixo);
4. a fonte sem serifa padrão do sistema (`paint::ttf::default_face`).

As fontes do sistema são procuradas pelo `paint::ttf` (pastas de fontes do
sistema, do Flatpak e do usuário), em qualquer estilo
(`ttf::find_any_style`, porque a Lucida Handwriting só tem a face
inclinada).

| Original | Embutida | Outras do sistema | Ajuste (largura × altura) |
|---|---|---|---|
| Lucida Sans | DejaVu Sans (recorte Windows-1252) | Verdana, Open Sans, Noto Sans, Liberation Sans, FreeSans | ≈ 0,96 × 0,99 |
| Lucida Handwriting | Merienda (+ inclinação de 11,25°) | Courgette, Kalam, Comic Neue | ≈ 1,13 × 1,01 |
| Lucida Blackletter | UnifrakturMaguntia | UnifrakturCook, New Rocker | ≈ 1,05 × 1,03 |
| Impact | Anton | Oswald, League Gothic | ≈ 1,01 × 0,92 |
| Stencil | Stardos Stencil Bold (minúsculas → maiúsculas) | Allerta Stencil, Saira Stencil One | ≈ 0,94 × 0,96 |

Escolha: largura média e altura das maiúsculas comparadas com as originais
(texto em português com acentos), e aparência lado a lado. A Anton tem a
mesma largura média da Impact (razão 0,996); a DejaVu Sans é a de desenho
mais parecido com a Lucida Sans (humanista, olho grande).

O ajuste é calculado ao carregar qualquer substituta: a escala vertical
iguala a altura do 'H' à da original e a horizontal iguala a largura média
sobre o texto `fonts::CALIBRATION` (limitadas a ±30%). As **métricas de
linha são sempre as da tabela acima**, seja qual for a fonte — a quebra de
página sai igual à do original; a quebra de linha fica perto (a largura
média é a mesma, não cada letra).

## API (`engine::writer::fonts`)

- `Family` (as 6, com `name`, `from_name` e `resolve` para os nomes da
  tabela `FONT`) e `RunFont` (família, pontos, negrito, inclinado,
  sublinhado; `RunFont::of(documento, CharProps)`, `RunFont::from_key`).
  Os tamanhos são os de `writer::doc::Size`.
- `WriterFonts::new(pablo)` resolve as famílias (com o sistema);
  `WriterFonts::embedded(pablo)` usa só as embutidas (igual em qualquer
  máquina). `source(família)` diz de onde veio cada uma.
- Medição: `line_metrics`, `advance`, `text_width`, `extent`;
  `fonts::line_metrics` dá as métricas de linha sem carregar fontes.
  `WriterFonts` implementa o `layout::Measure` da diagramação (o
  descendente inclui a entrelinha externa, que fica embaixo da linha).
- `render(fonte, texto)` → `TextMask` (cobertura 0–255 com origem no topo da
  linha, na posição da caneta); `draw(quadro, x, topo, fonte, texto, cor,
  suavizar)` desenha e devolve o avanço. Sem suavizar, só os pixels com mais
  de meia cobertura, na cor cheia (o aspecto das telas de 1995).
