# Creative Writer: assistentes do Max (Oficina de Projetos)

Os três projetos do Creative Writer PT-BR — **Jornal** (`WHAT 3a00`),
**Cartão** (`3c00`) e **Faixa** (`3800`) — são o editor do Escritório em
"modo projeto" com o Max no lugar da porta Saída, guiando a criança balão
a balão. Pesquisa do original: `cw/report/projects.md`,
`projects-jornal.md`, `projects-cartao.md`, `projects-faixa.md`. Este
documento descreve o **motor** (sem tela, pronto) e o **contrato** para a
cena que o hospeda (P6, depois do Escritório integrado).

## Onde está

| arquivo | conteúdo |
|---|---|
| `crates/engine/src/guide.rs` | núcleo comum com a lição da Maggie (Fine Artist): passo atual, histórico do Volta, balões de saída `3803` (`Ask`, `Answer`, `QUIT_ANSWERS`), "trabalhar por conta própria" |
| `crates/engine/src/writer/assistant.rs` | o motor: tipos dos roteiros, `END_FLOW` (fim comum), `Assistant` (eventos → saídas), montagem e desenho dos balões, tradução de cliques |
| `crates/engine/src/writer/assistant_news.rs` | roteiro do Jornal (`NEWS`) |
| `crates/engine/src/writer/assistant_card.rs` | roteiro do Cartão (`CARD`) |
| `crates/engine/src/writer/assistant_banner.rs` | roteiro da Faixa (`BANNER`) |
| `crates/engine/src/writer/fold_figure.rs` | figura e animação da dobra do cartão |

A lição da Maggie (`scenes::wizard::Wizard`) usa o mesmo `crate::guide`; o
que é só dela (painel da Maggie, formas para achar, nome digitado) e o que
é só do Max (âncoras, dicas, Rex, campo de páginas, passos pulados, figura
da dobra) ficam em cada módulo.

## Modelo

Um `Script` é uma lista de `Step`s com id (texto), mais o fim comum
`END_FLOW`:

```rust
step("colunas", &[say(16, Anchor::Tool(Tool::Page))],
     On(&[(Pat::Tool(Tool::Page), go("cria-colunas"))]))
    .skip_if(Cond::StripOpen(Tool::Page), "cria-colunas"),
```

- **`say`**: balões do passo — registro do bloco do roteiro (`Rec::N`), de
  outro bloco (`Rec::In(0x0faa, 14)`, o nome ao salvar) ou do fim comum
  (`Rec::End`, resolvido por `EndRecords`) — e a `Anchor` para onde a
  cauda aponta. O primeiro tem as opções, o campo e os botões; os
  seguintes são os "Clique aqui." que o acompanham (pasta: `[2]`+`[3]`;
  cavalete: `[26]`+`[27]`). Passo sem balão: espera invisível (a máquina
  de impressão por cima).
- **`Wait`**: `Next(go)` (Segue); `Options` (opções `[22c:N]` **pelo índice
  do original**, não pela ordem na tela — `3c00[0]` lista o `22c:1`
  primeiro); `Radios { default, next }` (`[22e:N]`, marcado de início, e
  Segue); `Pages { max, next }` (campo `[0:19]` da faixa, 1…8); `On(&[(Pat,
  Go)])` (ações reais na tela; vale a primeira que casar).
- **`Go`**: `Step(id, comandos)` ou `Leave`. Ids especiais: `FREE` (o passo
  de trabalho livre do roteiro, destino de "Trabalhar … por conta
  própria") e `START`.
- `entering(&[Command])` (ex.: abrir a pasta de amostras), `skip_if(Cond,
  id)` (Palavra já ativa, faixa da ferramenta já aberta: o "Clique aqui"
  some, como no original), `folded(FoldStage)` (figura da dobra), `help(N)`
  (registro do "Precisa de ajuda?" deste passo).

O fim comum (`projects.md` §4): Max → `salvar` (`[7]`/`[36]`/`[19]`) →
`nome` (`0faa[14]`, `Command::AskName`, espera `Saved`) → `imprimir?` →
`imprimir` (ferramenta Imprimir; com a faixa do Imprimir já aberta vai a
`imprimir-aberto`, a variante `[21]`/`[109]`/`[28]`) → `imprimir-mczee`
(botão do McZee) → `imprimindo` (espera `Printed`) → `pronto` ("Tudo
pronto!": por conta própria → `FREE`; novo → `START` com
`Command::NewDocument`, sem histórico; voltar → `Command::Leave`). Sem
salvar: `lixo` (imprimir ou jogar fora, os dois com `Command::Discard`).

## Contrato com a cena (P6)

```rust
let mut max = Assistant::new(Kind::Card, Context { pages: 1, ..Default::default() });
for o in max.begin() { /* executar */ }
// a cada clique:
if let Some(ev) = max.click(assets, x, y) { for o in max.on_event(ev) { … } }
// a cada ação no editor que o roteiro possa esperar:
max.ctx.word_active = editor.tool() == ToolId::Word;   // antes do evento
for o in max.on_event(Event::ToolClicked(Tool::Word)) { … }
// a cada quadro:
max.tick(dt);
// desenho, por cima do editor:
max.draw_max(frame, assets);  // Max na porta (7e13 / 7e9f "Clique aqui!")
max.draw(frame, assets);      // figura da dobra e balões com cauda
```

### Eventos que a cena manda

| ação na tela | `Event` |
|---|---|
| clique numa ferramenta da barra | `ToolClicked(Tool)` — `Tool` tem os nomes de `scenes::writer::tools::ToolId` |
| botão de faixa: Forma, Cria-colunas, Alto/Largo, Nº de página, alavanca, rolo BORDA, Visão normal, rolo de adesivos, lápis de cera, imprimir com o McZee | `ButtonClicked(Button)` |
| item de menu: colunas (1–4), Alto/Largo (`TALL`/`WIDE`), etiquetas (`LABEL_NONE`/`LABEL_PAGE`/`LABEL_NUMBER`) | `MenuItem(Menu, u8)` |
| clique na página | `PageClicked` |
| OK do cavalete de palavras com forma (palavra posta) | `ShapedOk` |
| "Já" na pasta de amostras (amostra aberta) | `SampleChosen` |
| adesivo pego na tira (na mão) / posto na página | `StickerPicked` / `StickerPlaced` |
| borda escolhida (armada) no rolo | `BorderPicked` |
| título gravado com Enter (depois de `AskName`) | `Saved` |
| a máquina de impressão fechou | `Printed` |
| o cartão ganhou a 5ª página | `FifthPage` (Rex `3c00[106]`) |
| teclas no campo de páginas | `Typed(c)`, `Backspace` |
| balões, botões, dicas, Max | `Assistant::click` devolve `Option`/`Radio` (índice do original), `Next`/`Back`/`Stop`, `Tip(N)`, `NeedHelp`, `MaxClicked`, `Dismiss` |

`click` devolve `None` quando o clique é do editor. Com o `3803[0]` aberto
só ele responde (o balão do passo fica à vista, inerte, como em
`projects_36_para`); clique fora dele é "ignorar". Clique dentro de um
balão sem acertar nada vira `Dismiss` (fecha dica ou aviso; no balão do
passo não faz nada, mas não chega ao editor: o balão é opaco).

### `Context` (a cena mantém em dia)

| campo | uso |
|---|---|
| `blank` | documento sem palavras: o Max do fim (`Pat::MaxDone`) mostra o Rex `x02[1]` e fica |
| `word_active` | pula `[17]`/`[9]+[10]`/`[68]` ("clique na ferramenta Palavra") |
| `open_strip` | faixa de opções aberta: pula os "clique na ferramenta X" (`[16]`, `[10]`, `[15]`, `[54]`) e escolhe a variante `[21]`/`[109]`/`[28]` do imprimir |
| `pages` | número de páginas: valor inicial do campo da faixa (`[3]` mostra o da amostra) |

### Saídas (`Out`, na ordem)

- `Entered(i)`: entrou no passo `i` (`max.at()` é o id) — redesenhar;
  registrar `Effect::Log("assistente: passo …")` como a lição da Maggie.
- `Sound(id)`: tocar do `WRITER.SND` (`snd::OPTION 02ff`, `NEXT 02eb`,
  `STOP 02ec`, `REX 02fe`, `TIP 0307`, `HELP 02d3`, `MAX 0283`, `FOLDER
  02f1`). Os sons das ações do editor (ferramentas, menus `022d`+`021c`,
  borda `0220`) são da cena.
- `Do(Command)`:

| comando | o que a cena faz |
|---|---|
| `OpenSamples` | abre a pasta por cima, em `MAX › Kind::folder()` (amostras `SHARED/USERS/MAX/SAMP*.MAX` com `WIZA` = `Kind::wiza()`, P19) |
| `SetView(View)` | `Normal`, `TwoPages`, `AllPages`, `SideBySide` (P11) |
| `SetPages(n)` | faixa com `n` páginas largas, palavra com forma esticada (P12) |
| `AskName` | `0faa[14]` sobre a barra inferior, título editável ali; Enter grava `USERS/<APELIDO>/<NOME>.MAX` com `WIZA`/`FLOR 3` e manda `Saved` (P17/P18) |
| `Discard` | não gravar ao sair |
| `NewDocument` | documento em branco do mesmo projeto (`Kind`) |
| `Leave` | voltar ao Andar dos Projetos (`SceneId::Cw(CwScene::Building(Floor::Projects))`, sons `02d7` + `02c0`) |

### Desenho e geometria

- Max na porta: `MAX_DOOR` `7e13` em `MAX_RECT` (581,16,55,87); com a
  placa "Clique aqui!" (`MAX_SIGN` `7e9f`) quando o passo espera um clique
  nele (`max_picture()`).
- Barra inferior: "Oficina de Projetos" | `Kind::doing()` ("Estamos
  fazendo um Cartão") | "Pág. N".
- Balões (`placed`, `place`): os do Max com a borda direita em 620 e o
  topo em 115, cauda em (598,103) (`projects_35`, `45_dobra_1`: mesmo
  pixel); os que apontam a barra logo abaixo do ponto, para a direita na
  metade esquerda (`28_colunas_2`) e para a esquerda na direita
  (`51_visor_1`, `52_visor_2`); a dica ao lado (à esquerda) do balão do
  passo; o `3803[0]` 31 px à esquerda e 22 px abaixo do topo dele
  (`36_para`); o Rex no meio. `Anchor::point()` tem os pontos medidos
  (`projects.md`); âncoras sem ponto (menus, pasta, cavalete, página) vão
  ao meio até a cena dar o lugar — a cena pode pôr os balões onde quiser
  a partir de `shown()`.
- Botões Volta/Pára/Segue só nos balões que apontam para o Max (os "Clique
  aqui." não têm); Volta quando há histórico, Segue em `Next`, `Radios` e
  `Pages`.
- Dobra (`fold_figure`): `0c65`@(183,127), `3c1b`@(193,207),
  `3c33`@(253,205) medidos; os intermediários `3c0b`, `3c13`, `3c23`,
  `3c2b` alinhados pelo canto inferior direito; máscaras 6 figuras adiante
  (`0c65` usa `3c09`); 150 ms por quadro; o Volta desdobra quadro a
  quadro, e um Segue no meio da volta dobra a partir de onde está.

## Decisões e lacunas

- **Max fora de hora** abre `3803[0]` (como a Maggie no Fine Artist); o
  original não foi observado nesse caso.
- `3c00[48]` "Não quero borda" segue como "Deixe como está" (o `[49]`
  "Clique em NADA para apagar uma borda" não foi visto).
- Cartão: o Max em `[26]` (amostra) vai a `[87]` "Estamos quase no fim",
  como no cartão em branco (o original caiu no Wine nesse clique;
  `[87]`/`[88]` vêm só do `WHAT`); `[88]` → `[31]` etiquetas.
- `[60]` "Quando terminar, clique aqui." (lápis de cera) aponta o Max.
- Faixa `[9]` aponta o Max e o `[10]` "Clique aqui." a ferramenta Palavra.
- O campo de páginas começa selecionado (digitar substitui), aceita 2
  dígitos; fora de 1…8 o Rex `3802[0]` com `@` = 8.
- Não usados (sobras ou não vistos): Jornal `[9]`, `[13]`, `[14]`,
  `[28]`/`[29]` (girar/preferidos, sobre os botões do cavalete); Faixa
  `[14]`, `[15]`, `[18]`, `[21]`, `[25]`, `[26]`; Cartão `[5]`, `[7]`,
  `[20]`/`[21]`, `[25]`, `[49]`, `[72]`–`[74]`, `[78]`–`[80]`, `[104]`,
  `[107]` e os marcadores "same as #N" (`[29]`, `[30]`, `[32]`–`[35]`,
  `[43]`, `[89]`–`[103]`), que só marcam passo no original. Os `[N]` de
  nome de cada bloco (`[9]`/`[21]`/`[38]`) dão lugar ao `0faa[14]`, como
  no original.
- Os testes `writer::assistant*` percorrem cada roteiro só com eventos e,
  com `ATELIE_DATA`, conferem cada balão no `WHAT` do `WRITER.EXE` (existe,
  não é marcador vazio, as opções/rádios/campo são os esperados, as dicas
  existem) e que todos cabem na tela; `dump_balloons_for_comparison` grava
  quadros com `ATELIE_DUMP`.

## A cena (P6, P7, P12, P17–P20)

`crates/engine/src/scenes/writer_project.rs` (`ProjectScene`,
`SceneId::Cw(CwScene::Project(kind))`, `--scene jornal|cartao|faixa`)
hospeda o `Writer` do Escritório em modo projeto (`Writer::for_project`:
andar "Oficina de Projetos", título "Estamos fazendo um …" até ter nome,
sem as boas-vindas do McZee) e o `Assistant`. O documento começa com
`writer::project::blank(kind)`.

- **Cliques**: aviso do Rex aberto (qualquer clique fecha) → balões e Max
  (`Assistant::click`; a porta Saída inteira, `DOOR_RECT`, é o Max) →
  pasta de amostras → editor. Os que vão ao editor viram eventos pelo
  lugar (`HITS`: Forma, Cria-colunas, Alto/Largo, Nº de página, rolos
  BORDA, Visão normal, rolos de adesivos, lápis de cera, McZee do
  Imprimir; a alavanca de Página e Visor troca a página da faixa que a
  cena acompanha) ou `PageClicked` (dentro de `PAGE`). Nada vira evento
  com a faixa modal, um "Epa!", o campo de nome ou o McZee por cima.
- **Mudanças do documento** (`writer::project::{Snapshot, events}`): a
  cena guarda o documento como o assistente o viu por último (depois dos
  próprios comandos) e manda o que mudou: colunas → `MenuItem(Columns,
  n)`, orientação → `MenuItem(Orientation, TALL/WIDE)`, etiquetas →
  `MenuItem(PageNumber, código)`, borda armada no rolo (ou, sem ela,
  borda nova) → `BorderPicked`, palavra com forma nova → `ShapedOk`,
  adesivo na mão → `StickerPicked` e adesivo novo → `StickerPlaced`
  (com `StickerPicked` antes, se não estava na mão: cavalete, colar), a
  5ª folha do cartão → `FifthPage`. O que está na mão vem das faixas
  (`PageStrip::border_armed`, `StickersStrip::held`, em
  `Snapshot::with_hand`): como no original, escolher no rolo já mostra
  o `[52]`/`[58]` "… então, clique…" (teste de jogo do Writer). Funciona com
  qualquer interface das ferramentas (menus da Preparação da página,
  cavalete de Forma), que são de outros ramos. Limitação: escolher no menu
  o valor que já estava (2 colunas no jornal) não muda nada e não avança.
- **Restrições** (`writer::project::restrict`, P20): a Preparação da
  página já recusa Alto na faixa (`0805[17]`) e fundo no cartão e na faixa
  (`0805[32]`). A cena é a rede de segurança: a folha da faixa que mudar
  de tamanho volta em silêncio (a faixa de 577×595 conta como "Alto", e
  escolher Largo nela trocaria as medidas) e um fundo posto por outro
  caminho volta com o Rex (som `02fe`).
- **Ordem**: os eventos do documento vão antes dos de clique do mesmo
  quadro (uma borda aplicada sem ter passado pelo rolo é `BorderPicked` e
  depois `PageClicked`, e o roteiro do cartão não fica parado no
  "aplique a borda").
- **Teclado**: com o campo de páginas da faixa no balão, os dígitos,
  Backspace/Delete e Enter (= Segue) vão ao assistente; com a pasta
  aberta, ao fichário; senão ao editor (o nome na barra de baixo é o
  `Rename` do Escritório).
- **Comandos**: `OpenSamples` abre o fichário da pasta por cima do editor
  só com as amostras do Max daquele `WIZA` (P19: miniatura `THMB`,
  Título/Autor, setas do Título para "mais amostras"; os balões
  `Folder(More)`/`Folder(Ok)` apontam para (496,287) e (263,356), medidos
  em `projects_07`/`projects_95`); Já abre a amostra **como cópia** e manda
  `SampleChosen`; o X volta ao passo anterior (`Back`). `SetPages(n)`
  estica as palavras com forma (P12, `stretch_banner`; desfazível). Na
  faixa, uma palavra com forma nova também é esticada na hora através das
  folhas que a faixa tem. `AskName` copia a borda da folha para o
  documento (`copy_border`, P18), garante as marcas do projeto e abre o
  campo de nome (`Writer::ask_name`); quando ele fecha com o documento
  gravado (Enter, ou as duas versões), a cena manda `Saved`. `Discard`,
  `NewDocument` (`Editor::new_from` com o projeto em branco) e `Leave`
  (sons `02d7`+`02c0`, andar dos Projetos).
- **Impressão**: o McZee da faixa do Imprimir abre a máquina de impressão
  do Escritório (`scenes::writer::print`, com o projeto gravado no
  documento); quando ela fecha (`Writer::printing` volta a `false`), a cena
  manda `Printed` e o assistente vai ao "Tudo pronto!".
- **Telas por cima** (cavalete de palavras com forma, máquina de
  impressão, Caderno, Central de Controle): os cliques são delas, nada vira
  evento e o Max fica apagado (o mesmo pontilhado cinza) e não responde.
  Os balões do cavalete apontam o campo (234,295) e o OK (512,393), com o
  "Quando terminar, clique aqui." embaixo dele (`projects_33_manchete_3`).
- **Visões**: `SetView` troca o arranjo do Visor de páginas
  (`Editor::set_view`: normal, duas páginas, todas, lado a lado).

### Faixa: palavra através das folhas (P12)

`Document::min_pages` (o `PGD ` dos cartões e faixas, lido por
`convert::from_max`) fixa as folhas; `Document::object_span` dá as folhas
que um objeto atravessa na faixa (`dop.banner`): a palavra de 1587 pt de
`FAIXA_PA.MAX` vai da folha 0 à 2. A diagramação lista o objeto em cada
folha que ele atravessa e o editor o desenha deslocado e recortado em cada
uma.

### Andar dos Projetos (P7)

`writer_floors::pick` começa `CwScene::Project(kind)` (primeira opção e
Ctrl+clique no troféu); a segunda opção (`writer_floors::folder`) abre a
pasta por cima do andar filtrada pela área (`CwScene::ProjectFolder`,
`WriterPortfolio::only`). O documento escolhido abre no Escritório **sem**
assistente; um documento de `FLOR 3` reaberto tem o andar "Oficina de
Projetos" na barra e a Saída volta ao andar.

### Gravação (P18)

O `.MAX` de um projeto sai com `WIZA` (`news`/`card`/`banr`), `FLOR 3`,
`ADD[10]` = 3, `ADD[11]` = 1/2/3 (`view_code`), a borda `BDR ` copiada e o
nome DOS do título (`store::path_for`: "Jornal Teste" → `JORNAL_T.MAX`).
