# Creative Writer: impressão (ferramenta Imprimir, máquina do McZee, folhas)

Tarefas W18 (ferramenta Imprimir do Escritório), P15 (composição das
folhas) e P16 (máquina de impressão do Writer). Pesquisa:
`cw/report/projects-impressao.md`, `office-ferramentas.md` §9; capturas
`projects_18`…`23`, `86`, `112` e as `cw-print_orig_*` (depressa, Rex,
cópias). Comparações original | port: `cw/shots/cw-print_compare_*.png`.

## Código

| onde | o quê |
|---|---|
| `writer/print.rs` | composição: `SheetKind` (página por folha, cartão, faixa), `PrintDoc` (páginas desenhadas pelo editor + `PageArt`), `paper_of` (papel do `PRNT`), `sheet_for`, `plan` (onde cada página cai em cada folha), `PrintDoc::sheets`/`render_sheet`/`page_image` |
| `writer/borders.rs` | `PageArt::from_refs`: borda e fundo a partir de qualquer busca de chunks (o `.MAX` lido ou o `MaxExtra` do editor) |
| `scenes/gallery_print.rs` | a máquina comum (`PrintPreview`) à Galeria do FA, à Biblioteca e ao Escritório; `MachineSpec` com os passos de escolha; `fit_darkest` |
| `scenes/writer_library.rs` | `WRITER_PREVIEW` (geometria, balões `4c00`/`4c02`/`0805`, bichos) e `WRITER_MACHINE` (passos) |
| `scenes/writer/print.rs` | a faixa do Imprimir (`PrintStrip`), `print_document(ed, a, kind)`, `document_from_file` |
| `examples/print_sheets.rs` | folhas de conferência e a tela da máquina para um `.MAX` |

## A faixa e a máquina

Faixa do Imprimir: "imprimir depressa" (`02ef`, 133,60) e "fazer escolhas"
(`02f3`, McZee, 337,59). Os dois tocam `0233` + `02f6`, fecham o balão do
McZee e cobrem o Escritório com o pontilhado cinza `(100,100,100)` (as
molduras laranja e pretas ficam), com a janela da `WRITER_PREVIEW`.

- **Depressa** (conferido no Wine, `cw-print_orig_depressa_pronto`): direto
  no `4c00[8]` "OK. Já estou pronto para imprimir!" com Pára / Segue.
- **McZee**: `4c00[0]` boas-vindas (Pára / Segue) → `[1]` fundo (só se o
  documento tiver fundo; rádios `[22e:1]` imprimir / `[22e:0]` ignorar) →
  `[6]` páginas (só documento ou jornal de mais de uma página; `[22e:1]`
  todas / `[22e:0]` algumas, caixas De e Até começando em 1 e N; digitar
  nelas marca "algumas") → `[7]` cópias (caixa com 1, toda selecionada;
  `@` = 10) → `[8]` pronto com Volta / Pára / Segue. Conferido no Wine:
  documento de uma página sem fundo vai das boas-vindas às cópias.
- Segue toca `02eb`, Pára `02ec` (fecha a máquina em qualquer passo), Esc
  fecha (ou cancela durante a impressão), Enter é Segue.
- Páginas inválidas (De > Até, fora do documento, não número): Rex
  `4c02[0]`; cópias fora de 1–10: Rex `4c02[1]` (`02fe`). Qualquer clique ou
  tecla fecha o aviso e o passo continua.
- Imprimindo: `4c00[9]` (Esc cancela → `4c02[2]`) e o dinossauro atrás do
  rato no canto, no lugar (`4c0b` encolhido, `4c0f` corcunda, `4c17`
  esticado, os pés na mesma linha, canto em (21,434)). Fim: `4c00[11]`
  (Pára). Falha do `lp`: `0805[21]`; sem programa de impressão (o `lp` não
  existe): `0805[20]` "Não consigo achar a sua impressora". O aviso fica por
  cima do balão "imprimindo" (como no original) e fechá-lo fecha a máquina.
- Passos do original não feitos: papel contínuo `[2]`/`[3]` (só com
  impressora de formulário contínuo, nunca apareceu) e cor `[4]`/`[5]` (só
  com impressora colorida; o `lp` não diz).

Prévia (à esquerda): documento e jornal mostram a 1ª folha que sai (em pé
na moldura cinza (66,62) 210×270; folha deitada deita a moldura); o
cartão mostra **só a capa** no tamanho de uma página (`projects_112`); a
faixa mostra as páginas emendadas numa tira de 54 px de largura no meio
de uma coluna cinza de 70 px, com contorno e as emendas (`projects_86`).
A redução fica com o pixel mais escuro de cada bloco (o original desenha o
texto em corpo pequeno e às vezes fica cinza).

## As folhas

Cada página vem do editor (`Editor::render_page`, 1 px por ponto, sem
borda nem fundo; o branco é transparente). A borda e o fundo
(`writer::borders`) são redesenhados por cima do branco na resolução da
folha, 144 dpi (`paint::print::SCALE`), e o texto e os objetos vão por
cima (ampliados).

Papel: o `PRNT` do documento (`"prr1"`, dpi x/y, largura/altura em pixels
da impressora): 612×792 (carta, 300 dpi) nos exemplos do Max, 595×792 nos
documentos gravados sem impressora, 792×595 na faixa do Wine. No original a
página é a área imprimível dessa folha (por isso as faixas dos exemplos
têm 542×767 e a do Wine 577×595). A folha é o papel como está, virado se a
página só couber assim, ou do tamanho da página; a página vai centrada.

- **Página por folha** (documento, jornal): uma folha por página escolhida.
- **Cartão**: uma folha; quartos com a metade da folha cada; em cima à
  esquerda a página 1 (Capa) e à direita a 4 (Contracapa), as duas giradas
  180°; embaixo a 2 e a 3 em pé (figura `0c65`; comparação em
  `cw-print_compare_folha_cartao.png`). A página é reduzida para caber no
  quarto (metade exata no papel padrão). Da 5ª página em diante não sai.
  Linhas tracejadas de dobra no meio (guias).
- **Faixa**: uma folha por página, para emendar lado a lado, com linhas de
  corte tracejadas nas beiradas da página quando sobra papel dos lados.

Páginas: as da diagramação, ou as do `PGD` gravado se forem mais (a faixa
de N folhas e o cartão de 4 que não têm texto em todas); o cartão tem pelo
menos 4.

As folhas são gravadas como `impressao-<carimbo>[-k].png` na pasta do
usuário e vão para o `lp -n <cópias> -o fit-to-page` (`ATELIE_LP` troca o
programa). A montagem e a gravação correm numa thread.

## Para os assistentes da Oficina de Projetos

- `scenes::writer::print::print_document(&ed, a, Some(kind))` → `PrintDoc`
  (o projeto escolhe a composição; `None` usa o `WIZA` gravado ou a marca
  de faixa do `DOP`);
- `Writer::print_document(Some(kind), quick, a)` escolhe a ferramenta
  Imprimir e abre a máquina (com escolhas ou depressa);
- `Writer::printing()`: a máquina está aberta. O `Printed` do roteiro é a
  passagem de `true` para `false`.
- O próprio botão do McZee da faixa também abre a máquina (o roteiro
  espera o clique nele): o projeto vem do documento.

## O que falta

- O texto entra na folha ampliado de 72 para 144 dpi (blocos); desenhar as
  fontes do Writer na resolução da folha.
- A palavra com forma (Splot) ainda não é desenhada pelo editor no main:
  sai o retângulo cinza que marca o lugar dela, e a faixa sai sem a
  palavra (o desenho através das folhas é do ramo dos projetos).
- As etiquetas do cartão ("Capa", "Página Interna 2"…) são o número de
  página do editor.
- Branco e preto, papel contínuo; a folha da prévia "subindo" enquanto
  imprime (`projects-impressao.md`) não aparece nas capturas e não foi feita.
