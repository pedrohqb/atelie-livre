# Creative Writer: o modelo do documento (Escritório)

Biblioteca do Escritório em `crates/engine/src/writer/`, tarefas W2, W3, W8 e
W9 do plano (`office.md` §10 da pesquisa), usada pela cena `scenes/writer`
(o `Editor`; API em [`creative-writer.md`](creative-writer.md)). A conversão
para o `.MAX` fica em `writer/convert.rs` e os arquivos em `writer/store.rs`.

| módulo | o quê | tarefa |
|---|---|---|
| `writer/doc.rs` | documento: parágrafos, caracteres, figuras do Maçã na linha, folha (`DopProps`), fontes, edição | W2 |
| `writer/object.rs` | objetos flutuantes (adesivo, desenho, palavra com forma, moldura de cartão), Flutua-texto, contorno | W2 |
| `writer/layout.rs` | diagramação em folhas e colunas, contorno de objetos, quebra de página, número de página, consultas | W3 |
| `writer/selection.rs` | seleção, teclado, arrasto, área de transferência interna | W8 |
| `writer/undo.rs` | pilha de 20 ações do Ovo, Ctrl+Z alternando, ovos da faixa | W9 |

Testes: os de cada módulo (`cargo test -p engine --lib writer::`) e
`crates/engine/tests/writer_model.rs`, que confere com o `WRITER.EXE` as 16
cores (`CSCH`), os nomes das fontes (`STR#` 0x206), a fonte padrão e o
"Pág. %d" (`STR#` 0x800).

## Correspondência com o `.MAX`

Os tipos têm os mesmos nomes e semânticas dos de `formats::max`
(`docs/formats/max.md` na pesquisa), para que a conversão documento ↔ arquivo
seja campo a campo. As unidades são as do arquivo: **pontos** (1/72 pol.).

| `.MAX` | modelo | observação |
|---|---|---|
| `TEXT` (Windows-1252, `\r` fecha parágrafo, `\r` final a mais) | `Document::paras: Vec<Paragraph>`, `Paragraph::items` | o `\r` de cada parágrafo não é item; a conversão deve tirar o `\r` extra do fim na leitura e repô-lo na gravação |
| caractere com sprm `0x03` (âncora) | `Item::Rebus(u8)` (o byte do texto) | não é texto: `Document::text` e o corretor pulam |
| `FDPC` + `BTEC` | `Item::Char(char, CharProps)` e `Paragraph::mark` (o `\r`) | os trechos (runs) saem juntando itens iguais (`Paragraph::spans`) |
| `FDPP` + `BTEP` | `Paragraph::props: ParaProps` | um por parágrafo |
| `FONT` | `Document::fonts: FontTable` (`ftc`, nome, família) | `ftc` 0 = Lucida Sans; `ftc` ausente vale a padrão |
| `DOP ` | `Document::dop: DopProps` | ver abaixo |
| `FRAM` | `Document::objects: Vec<FloatObject>` | ver abaixo |
| `PGD ` | calculado: `Layout::page_starts` | a gravação reconstrói das quebras |
| `PICT`/`SPLE`/`SND `/`BDR ` | `ObjRef { tag, id }` | os bytes ficam com a conversão (`formats::max::Document::objects`) |
| `TWPB` | `FloatObject::outline: Outline` | pode ser recalculado da máscara (`Outline::from_mask`) |
| `SNDT` (sons em palavras), `SELN`, `ADD `, `TITL`, `AUTH`, `THMB`, `PRNT` | — | fora deste modelo (sons em palavras: W11; o resto é da gravação, W22) |

### Caractere (`CharProps`)

| campo | sprm | valores |
|---|---|---|
| `font: u16` | `0x0e` | `ftc` da tabela; as 6 da lista estão em `FONTS` (Pablo Graphos, Lucida Sans, Lucida Handwriting, Lucida Blackletter, Impact, Stencil) |
| `size: Size` | `0x0f` | Gigante 48, Enorme 36, Grande 24, Média 18, Pequena 14, **Mínima 12** (padrão, sem sprm), Micro 9; `Size::from_pt` pega o mais perto (os 27 exemplos só usam esses) |
| `color: Option<Rgb>` | `0x0c` + `0x10` | `None` = preto; RGB e não índice, porque os exemplos têm cores fora do `CSCH` (`#d016de`) |
| `highlight: Option<Rgb>` | `0x0d` + `0x11` | Iluminador |
| `bold`, `italic`, `underline` | `0x04`–`0x06` | Seleciona-estilos: "Simples" tira os três, os outros ligam o seu (`TextStyle`) |
| `effect: Option<Effect>` | `0x0b` | 1 Pisca-pisca, 2 Formiguinhas, 3 Vapt, 4 Vupt, 5 Brilho, 6 Treme-treme (o "Maluco" é um Mix, não um efeito); outro valor fica em `Effect::Other` |

`CSCH` tem as 16 cores do Seleciona-cores/Iluminador na ordem da lista.
`CharChange` é o "pincel" de cada lista (`Font`, `Size`, `Color`,
`Highlight`, `Style`, `Effect`, `All`); `Document::apply_char` o aplica a um
intervalo (o Aplicador usa `Document::all()`), e `Document::expand_to_words`
leva o arrasto do pincel a palavras inteiras, como no original.

### Parágrafo (`ParaProps`)

| campo | sprm | lista |
|---|---|---|
| `align: Align` | `0x19` | Posicionador: Esquerda 0, Direita 1, Centro 2, Esquerda e direita 3 |
| `indent: Indent` | `0x1a` | Recuos: nenhum, só a 1ª linha (1), todas menos a 1ª (2), todas (3) |
| `line_spacing` | `0x1c` | Espaçamento: espaço extra entre as linhas (entrelinha dupla) |
| `space_after` | `0x1b` | Espaçamento: espaço extra depois do parágrafo (meia linha) |
| `new_page` | `0x1f` | Espaçamento: começa uma nova página |
| `drop_cap: Option<u8>` | `0x1d` | Estilos de parágrafo: Capitular (o original grava 3 = linhas) |
| `raised_cap` | `0x1e` | Estilos de parágrafo: Destacada |
| `bullets` | `0x20` | Estilos de parágrafo: Marcadores |

As listas "Estilos de parágrafo" e "Espaçamento" têm um ✓ só: os setters
`set_para_style`/`set_spacing` (e `ParaChange::Style`/`Spacing`) ligam um e
desligam os outros do grupo, mas grupos diferentes convivem (o Mix
"Relatório" = espaço depois + recuo da 1ª linha). Enter (`Document::split`)
copia as propriedades do parágrafo para o novo, menos "começa uma nova
página".

### Documento (`DopProps`)

| campo | sprm | padrão / valores |
|---|---|---|
| `page_width`, `page_height` | `0x3e`, `0x3f` | **595 × 792** (o que o original grava num documento novo); "Largo" troca os dois (`set_orientation`) |
| `margin_top/left/bottom/right` | `0x40`–`0x43` | 72; as 5 da lista (`Margin`): 36, 54, 72, 90, 108 = 1,3 / 2 / 2,5 / 3,2 / 3,8 cm, as quatro iguais (`set_margin`) |
| `column_gap` | `0x45` | 12 (42 nos cartões) |
| `field_49` | `0x49` | 36 (desconhecido, regravado) |
| `columns` | `0x4a` + 1 | 1 a 4 (`set_columns`) |
| `newspaper`, `banner` | `0x4c`, `0x5b` | jornais / faixas |
| `page_number: PageNumber` | `0x4d` | nenhum, "Pág. N" (1), "N" (2) |
| `page_number_rect` | `0x51`–`0x54` | (topo, esq., base, dir.); `None` = centrado no pé, 64 × 15 (`page_number_box`; confere com `S4.MAX` "Largo": (580, 364, 595, 428)) |
| `border`, `border_alt` | `0x58`, `0x5a` | `BDR n`; o original grava os dois iguais (`set_border`) |
| `border_inner`, `border_outer` | `0x47`, `0x48` | larguras da borda |
| `background` | **?** | rolo FUNDO; o sprm não aparece em nenhum exemplo — a conversão o ignora até ser medido (W26) |

A folha "carta 612 × 792" do plano: o original grava 595 de largura (e 792 ×
595 em "Largo"); o modelo segue o arquivo. Na Visão normal 1 pt ≈ 1 pixel: a
página ocupa x 23..615 da tela e o texto começa em (93, 180) = (23 + 72,
108 + 72).

### Objetos (`FloatObject`)

| campo | sprm de `FRAM` | observação |
|---|---|---|
| `page` | `0x27` | 0 = primeira |
| `kind: ObjectKind` | `0x28` | Splot 2 (`Shaped`), figura 3 (`Sticker` e `Drawing`), moldura de página 4 (`PageFrame`); adesivo × desenho do cavalete se distingue pela figura (o desenho não tem `PKBM`) |
| `wrap: Wrap` | `0x29` | ver abaixo |
| `x`, `y` | `0x2c`, `0x2d` | canto superior esquerdo na folha |
| `width`, `height` | `0x2e`, `0x2f` | tamanho atual |
| `orig_width`, `orig_height` | `0x30`, `0x31` | tamanho ao colar (`reset_size`) |
| `object: ObjRef` | `0x35` | `PICT n`, `SPLE n` |
| `sound: Option<ObjRef>` | `0x37` | `SND n` (Varinha Mágica; Remove-sons: `Document::remove_object_sound`) |
| `border: Option<ObjRef>` | **?** | borda em volta do adesivo (Preparação da página, pág. 2): sprm desconhecido (W26) |
| `outline: Option<Outline>` | `TWPB` do `PICT` | contorno para o modo normal |

O sprm `0x2a` (sempre `ff` nas figuras) não tem campo: a conversão o grava
ligado.

Operações: `FloatObject::centered` (adesivo colado com um clique),
`move_by` (Mover-modificar; um canto de 8 pt sempre fica na folha),
`resize` com `Resize::All/Width/Height` (os três Dimensionar; "tudo" mantém a
proporção), `contains`, `on_page`; no documento, `add_object` (por cima),
`object_at` (o de cima no ponto), `remove_object`.

#### Flutua-texto (`Wrap`)

| lista (bitmap) | `Wrap` | `0x29` (provisório) | diagramação |
|---|---|---|---|
| "em volta das bordas — é o normal" (`0c2b`) | `Contour` | ausente | o texto segue o contorno da figura, dos dois lados |
| "coloca o adesivo em todas as páginas" (`0c2d`) | `AllPages` | 1 | o objeto se repete na mesma posição em todas as folhas, texto em volta das bordas |
| "deixando um quadrado de espaço em branco" (`0c2f`) | `Box` | 2 | o texto contorna o retângulo |
| "acima e abaixo" (`0c31`) | `TopBottom` | 3 | nenhuma letra ao lado do objeto |

O código do sprm foi deduzido (índice na lista, com o normal sem sprm; nos
exemplos aparecem só 3, numa figura de jornal, e 2, nas molduras dos
cartões) e precisa ser conferido no original.

## Diagramação (`layout.rs`)

`Layout::build(&doc, &measure)` devolve as folhas (`pages`, pelo menos uma,
com as linhas, os objetos de cada uma e a caixa do número) e as linhas
(`lines`, em ordem de leitura). Cada linha sabe o parágrafo, os itens
`start..end`, folha, coluna, topo, altura, base e a caixa (`ItemBox`) de
cada item em coordenadas da folha, com o tamanho de desenho (a capitular e
a destacada ampliam).

Regras:

- área do texto = folha menos margens, dividida em `columns` colunas com
  `column_gap` entre elas; o texto corre coluna a coluna e folha a folha;
- quebra de linha por palavras (os espaços do fim ficam pendurados); uma
  palavra maior que a coluna é partida;
- altura da linha = maior ascendente + maior descendente da linha (linha
  vazia: as do `\r`); uma linha nunca fica entre duas colunas;
- "começa uma nova página" abre folha nova, a não ser na folha ainda vazia;
- cada linha é cortada em **segmentos** pelos objetos da folha (com folga de
  `WRAP_GAP`); o texto enche os segmentos da esquerda para a direita, como no
  original ("escola e resolveu ler [Ana] tudo de uma vez", `office_34`);
  sem segmento útil (mais estreito que `MIN_SEGMENT`), a linha desce até
  passar do objeto;
- alinhamento por segmento; "esquerda e direita" distribui a sobra nos
  espaços, menos na última linha do parágrafo;
- recuo `INDENT` (36 pt) conforme `Indent`; marcadores: bolinha `BULLET` no
  começo de um recuo de `BULLET_INDENT` (18 pt) em todas as linhas;
  capitular: a 1ª letra com `n` vezes o tamanho, base na `n`-ésima linha, e
  as linhas ao lado recuadas pela largura dela + `DROP_GAP`; destacada: a 1ª
  letra com `RAISED_SCALE` (3) vezes o tamanho, na 1ª linha;
- espaço extra entre as linhas (`0x1c`) = entrelinha dupla (+ a altura da
  linha); depois do parágrafo (`0x1b`) = meia altura da última linha (+7 na
  fonte padrão de 15), medidos em `office-complemento.md` §3.1;
- folhas = as do texto, ou mais se um objeto está numa folha adiante;
- número de página em cada folha, na caixa do `DopProps`;
- um objeto que tampa a coluna inteira em todas as folhas não trava a
  diagramação: depois de algumas colunas sem lugar, a linha ignora os
  objetos.

As medidas marcadas acima (recuo, marcador, folga, capitular,
destacada) são **provisórias** e devem ser medidas no original (W26).

Medição: o trait `Measure` (`advance`, `metrics`, `rebus_size`) recebe uma
`FontKey` (nome da fonte da tabela, tamanho, negrito, inclinado) e responde
em pontos. A cena liga as fontes de verdade (W4); `SimpleMeasure` (avanço
3/5 do tamanho, ascendente 4/5, descendente 1/4) serve para os testes e
quando falta fonte.

Consultas: `caret(pos)` (folha, x, topo, altura), `pos_at(page, x, y)`
(clique; nas colunas, vale a coluna clicada), `vertical(pos, up, x)` (↑/↓),
`line_home`/`line_end`, `line_of`, `lines_on(page)`,
`selection_rects(range)` (vídeo inverso, um retângulo por pedaço contínuo de
linha, mais o `\r` selecionado) e `page_starts(doc)` (para o `PGD `).

## Seleção e área de transferência (`selection.rs`)

`Selection { anchor, caret, goal_x }`; vazia = ponto de inserção.
`apply(doc, layout, Motion, extend)` faz ←→, Ctrl+←→ (palavras), ↑↓ (com a
coluna desejada guardada em `goal_x`), Home/End (da linha diagramada),
Ctrl+Home/End; com Shift estende; sem Shift, ←/→ sobre uma seleção a desfazem
na ponta correspondente. `begin_drag`/`drag_to` (arrasto), `select_word`,
`select_all`. Edição sobre a seleção: `type_text`, `newline`, `backspace`,
`delete_forward`; `typing_props` dá o estilo do ponto de inserção.

`Clipboard` (interna; não é a do sistema) guarda `Clip::Text(Fragment)` ou
`Clip::Object(FloatObject)`. Texto: `copy`, `cut`, `paste_text` (no lugar da
seleção; recortar e colar no mesmo lugar devolve o documento igual, inclusive
as propriedades de parágrafo); as fontes vão pelo nome e são casadas com a
tabela do documento de destino. Objeto: `copy_object`, `cut_object`,
`paste_object(page, x, y)` (centrado no clique, quantas vezes quiser, como o
Colar dos Re-movedores).

## Ovo (`undo.rs`)

`UndoStack<T>` genérica; a cena guarda um `Snapshot { doc, sel }` antes de
cada ação (`record`). Até `UNDO_LIMIT` = 20 ações (as mais antigas se
perdem). `undo`/`redo` (DESFAZER/REFAZER da faixa), `toggle` (Ctrl+Z:
depois de um desfazer refaz, senão desfaz), `undo_len`/`redo_len`, `eggs()`
(chocados e depois inteiros, da esquerda para a direita) e `egg_pos(k)`
(faixa a partir de (144, 68), passo 13; `EGG_HATCHED` `08e3`, `EGG_WHOLE`
`08df`).

Digitação (`record_typing`): no original, um texto digitado de uma vez volta
em dois passos — o 1º desfazer apaga tudo menos a 1ª letra, o 2º apaga a 1ª
letra. A pilha faz igual: a 1ª tecla é uma ação e as seguintes formam outra,
até uma pausa de `TYPING_PAUSE` (1 s, provisório), `close()` ou outra ação.

## Integração (feita)

1. A cena usa `doc::Document` + `selection::Selection` +
   `undo::UndoStack<Snapshot>` + `layout::Layout`, medida com `WriterFonts`
   (e o tamanho de verdade das figuras do Maçã: 4 + largura + 4).
2. Os adesivos viram `FloatObject`s com a cópia do `PICT` da biblioteca
   (`Editor::add_sticker`), com o contorno `TWPB` calculado.
3. Conversão `doc::Document` ↔ `formats::max::Document` em `convert.rs`
   (trechos `FDPC` juntando itens iguais, um `FDPP` por parágrafo, o `\r`
   final a mais, `PGD ` de `Layout::page_starts`, `0x2a` ligado; o que o
   modelo não representa vai e volta em `MaxExtra`). Os 21 exemplos do Max
   voltam iguais (`tests/writer_samples.rs`).
4. `writer/legacy/` e as reexportações antigas saíram.
