# Creative Writer: Oficina de Idéias (Máquina, Quadro, Caderno, cavalete)

Os close-ups da Oficina de Idéias (4º andar). Pesquisa do original em
`cw/report/ideas.md` (§3–6) e `cw/report/office-complemento.md` (§6–7);
comparação lado a lado com o original em `cw/shots/ideas_compare_*.png`.

| O quê | Código | Tela |
|---|---|---|
| Lógica sem tela | `writer::ideas::{machine, board, notes, session, story, store, rebus}` | — |
| Máquina de Idéias | `scenes::writer_ideas::machine::IdeaMachine` | `SceneId::Cw(CwScene::IdeaMachine)`, `--scene idea-machine` |
| Quadro de Figuras | `scenes::writer_ideas::board::PictureBoard` | `SceneId::Cw(CwScene::PictureBoard)`, `--scene picture-board` |
| Caderno de Idéias | `scenes::writer_ideas::notebook::NotebookView` (sobreposição) | aberto pela Máquina e pelo Quadro |
| Cavalete de lápis de cera | `scenes::writer_easel::CrayonEasel` (sobreposição) | POE e "modificar a figura" do Quadro |
| Botão com estado apertado | `scenes::writer_ideas::buttons::{PressButton, Presser, Art}` | os botões dos close-ups |
| Ponte para o editor | `writer::ideas::story::IdeaStory`, `session::{set_story, take_story}` | `SceneId::Cw(CwScene::IdeaStory(_))` |

Na sala (outro ramo), `Pick::IdeaMachine` e `Pick::PictureBoard` começam
os close-ups (`writer_floors::pick`); Ctrl+clique pula o balão.

## Estado da sessão

As cenas são recriadas a cada troca de tela; o que vale para a sessão do
programa fica em `writer::ideas::session` (um `thread_local` por usuário;
trocar de usuário zera): a Máquina (a sentença sorteada continua a mesma ao
voltar), a última figura vista no Quadro, a última aba do Caderno, a área
de transferência do Caderno (`Clip::Text` / `Clip::Picture`, que o editor
cola) e a história pendente para o editor.

Gravação (formato do port, `docs/formats/arquivos-do-port.md`): as
entradas da Máquina ao sair dela; as figuras próprias do Quadro ao sair
dele; o Caderno no OK.

## Botões (C9)

Cada botão tem a figura normal e a apertada (`office-complemento.md` §7:
FEITO `753b`→`7561`, peixe `753f`→`7567`, POE `7543`→`7569`, TIRA
`7547`→`756d`, OUTRA `7571`→`7577`, seta da lista `02db`→`02dd`). Apertar
mostra a apertada; o som e a ação vêm ao **soltar sobre o mesmo botão**;
soltar fora cancela. As setas de 12×12 não têm máscara: o branco é
transparente (`Art::keyed`).

## Máquina de Idéias

Composição de `ideas.md` §3: parede `6813`, Max sentado `7dff`, corpo
`69bf`/`7531` em (160,110), alavanca `754b` (puxada `754f`) em (423,148),
FEITO (333,176), botões redondos (373, 242+63k), POE (175, 237+63k), TIRA
(176, 265+63k); janelas (204,232), (204,296), (204,358), 162 px de largura.

- Texto da janela centralizado (os espaços do formato `"    %s %s"` não
  aparecem); figura-rebus centrada.
- Alavanca: som `21d`, as três janelas sorteadas a cada 1/15 s por
  1000 ms (`SPIN_MS`), o peixe fica puxado 0,3 s. Botão redondo: `2dc`, só
  aquela janela. Ctrl = só entradas do usuário; Shift = só figuras
  (`Mode::from_keys`).
- POE (`23b`): a janela fica vazia com o cursor; digitar escreve
  centralizado com quebra de linha; Enter (ou clicar em outro lugar)
  termina; texto vazio não entra.
- TIRA (`25f`+`2fe`): entrada do usuário → `0805[7]` (jogar fora ou
  manter); do Max → `0805[9]`.
- FEITO (`239`) → `0805[40]`: história (abre `CwScene::IdeaStory(Machine)`
  com a sentença), Caderno (abre na aba Máquina com a sentença), fechar
  (volta à sala, `2c0`), continuar.
- Chegada: `2f6`+`21d` e `0bb8[46]`; a opção abre as instruções
  `0bb8[47]`, que o port desenha como balão com o título e a figura
  explicativa `0c3f` (o `[860:0]` do `WHAT`; para o Quadro `[85f:0]` =
  `0c3b`). Clicar no Max abre `[46]` de novo (não medido no original).

## Quadro de Figuras

Moldura `7579`/`7725` em (160,80); a figura (do Max: `PICT` do
`WORKSHOP.CHK` por `wmf::meta_dib`; do usuário: PNG) em tamanho natural,
centrada em (193,170,248,202) e recortada, desenhada **só onde a moldura é
branca** (a sanefa e as franjas ficam por cima, como no original). OUTRA
(326,376) sorteia outra (nunca a mesma); seta da lista (206,386) abre a
lista suspensa (x 222, 17 linhas de 21 px, ▲▼ no cabeçalho, ✓ na atual;
ordem: do usuário, das mais novas, e as do Max); campo de título
(222,381,96,19).

- Clique na figura: cavalete com a figura. Figura do Max modificada vira
  uma figura nova do usuário com o mesmo título (as coisas do Max não
  mudam; comportamento do original não medido).
- POE: cavalete vazio; depois do OK a figura entra no começo da lista e o
  título é digitado no campo (Enter termina; vazio = "Figura
  Desconhecida", `STR#0809[2]`).
- TIRA `0805[10]`/`[11]`; FEITO `0805[41]` (história com a figura como
  adesivo, Caderno na aba Figuras, fechar, continuar).

## Caderno de Idéias

`7929`/`792d` em (163,73), aba ativa por cima (Lembretes `791f`, Figuras
`791b`, Máquina `7727`), tesoura `7923` (179,359), câmera `7927`
(216,359), campo de título, NÃO `0271` (361,362), OK `026d` (393,362),
setas ▶ `0871` (431,359) e ◀ `0819` (431,377). Abre na última aba usada,
com o balão `0bb8[59]`.

- Trabalha numa **cópia**: OK (`239`) grava as três abas; NÃO (`2ed`)
  fecha sem perguntar e descarta tudo, inclusive o item recém-posto.
- Clique fora: com um balão de ajuda aberto, só o fecha; senão `0805[15]`
  (`2fe`) e a moldura laranja em volta de NÃO/OK.
- Lembretes: clicar põe o cursor, arrastar seleciona (vídeo inverso),
  digitar escreve. Tesoura (`217`) e câmera (`288`) agem na hora sobre a
  seleção (balões `[72]`/`[73]`).
- Máquina e Figuras: tesoura/câmera armam o modo (balões `[61]`/`[60]`); o
  próximo clique numa sentença (ou na figura da página) a recorta/copia
  inteira; copiar mostra "OK! Foi copiado." (`[64]`).
- Arrastar pela faixa de cima ou de baixo move o caderno.
- Diferenças conhecidas: o texto das sentenças sai na fonte dos balões (o
  original usa a fonte do documento); o título não é editável.

## Cavalete de lápis de cera

Janela (115,100)–(524,379): moldura preta, faixa laranja com sombra
marrom; área branca (120,105) de 400×200 mostrando um pedaço do desenho de
600×350 (ou do tamanho da figura que veio para ser modificada); barra
cinza (y 310–349) com lápis `095b`/`0bb9` (163,316), aspirador
`0bbb`/`0bc1` (235,317), ovo `0295`/`0297` (313,314), setas ▲ `02db`
(405,313), ▼ `02df` (405,335), ◀ `0819` (393,324), ▶ `0871` (417,324),
NÃO `0271` (446,316), OK `026d` (484,317); paleta das 16 cores
(`office-palavra.md` §7) em casas de 25 px a partir de x 120, a escolhida
com borda de 4 px. Balão `0fa1[6]` ao abrir.

Lápis: traço de 4 px na cor escolhida; aspirador limpa tudo; ovo desfaz
(um nível, `paint::History`); setas andam 40 px. Ids achados casando o
contorno de 1 bit com as capturas `ideas_34_desenho.png`/`ideas_44`.
Falta: mudar o tamanho do cavalete arrastando as bordas (`0fa1[7]`).

## Ponte para o editor (I10) — o que falta ligar

`IdeaStory::from_sentence` monta o documento (modelo rico
`writer::doc`) com a sentença no 1º parágrafo (fonte padrão, rebus como
`Item::Rebus`, cursor no fim); `IdeaStory::from_picture`, a figura como
adesivo flutuante `PICT 0` em tamanho natural, centrado na largura da
folha e na margem de cima. O close-up guarda a história com
`session::set_story` e troca para `SceneId::Cw(CwScene::IdeaStory(fonte))`.

O Escritório integrado (ramo `cw-office-core`) deve, ao montar essa cena:
pegar o conteúdo com `session::take_story(usuário)`; abrir o documento com
o cursor em `IdeaStory::cursor` e as figuras de `IdeaStory::pictures`; pôr
"Oficina de Idéias" (`story::FLOOR_LABEL`) na barra de status e o balão
`IdeaSource::help` (`0faa[13]`/`[16]`); ao salvar, gravar `FLOR` 4 e
`WIZA` (`IdeaSource::wiza`: `plot`/`wpic`); a porta SAÍDA volta à Oficina
(`IdeaSource::back`). Até lá `run.rs` abre o editor atual sem o conteúdo.
A área de transferência do Caderno (`session::clip`) é a que o "Colar" do
editor deve usar.

## Pasta filtrada (I12) — pendente

"Trabalhar com uma história que você já começou" deve abrir a pasta
filtrada em "Conteúdo: Máquina de Idéias / Quadro de Figuras"
(`IdeaSource::kind_label`, `from_wiza`). Depende da pasta de documentos do
Writer (T13) guardar o tipo do documento; a sala hoje abre a pasta comum.

## Piadas da Varinha no mastro (I13)

O grupo do 4º andar é `0961–0966` (`writer_gags::IDEAS_SURPRISES`), não
`0a8f–0a99` como sugeria `ideas.md` §9: a tabela do código
(`building.md` §8.7) dá `0a8f–0a99` ao Escritório, onde o dálmata `0a93`
(só desse grupo) foi visto; na Oficina, conferido no original nesta
rodada, a Varinha no mastro mostrou o homem de chapéu descendo (`668c..`,
presente em `0964`/`0965`).
