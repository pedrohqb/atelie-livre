# Arquitetura do engine

Mapa dos módulos do crate `engine` e dos componentes compartilhados, para quem
for escrever telas novas — em especial o Creative Writer, que reaproveita a
Entrada, as salas, os balões, os adesivos, a pasta, a impressão e os sons do
Fine Artist. Regra geral: **antes de escrever outra cópia de um laço de
interface, procure aqui**; se o que você precisa quase existe, generalize o
componente (com testes) em vez de copiá-lo.

## Crates

| Crate | O quê |
|---|---|
| `formats` | Leitura dos formatos originais (`.MAG`, `WHAT`, `ATAG`, `PKBM`, `CHNK`, `.FNT`, paletas, WAV, GIF…). Sem dependência de tela. Ver `docs/formats/`. |
| `engine` | Recursos, quadro 640×480 em software, cenas. Não conhece janela nem dispositivo de som: as cenas devolvem `Effect`s. |
| `app` | Plataforma (SDL3): janela, escala, áudio, microfone, configuração, troca de cenas (`run.rs`). |
| `tools` | Ferramentas de desenvolvimento (extração de recursos etc.). |

## Fluxo de uma cena

`scene.rs` define a interface:

- `trait Scene { update(&mut self, &[Input], dt, &Assets) -> Vec<Effect>; draw(&self, &mut Frame, &Assets); … }`
- `Input`: cliques, arrasto, teclas, texto.
- `Effect`: `PlaySound`, `PlayLoop`/`StopLoop`, `SetCursor`, `ChangeScene`,
  `Push`/`Back` (cena por cima, como a pasta aberta pela maleta), `SetVolume`,
  `RecordStart`/`RecordStop`, `SetUser`, `Log`, `Quit`.
- `KidCommon`: cursor padrão, som de clique e teclas comuns às telas infantis.
- `SceneId` (em `scene.rs`) e a construção das cenas (`app/src/run.rs`).
  As telas do Creative Writer que reaproveitam cenas do Fine Artist são
  `SceneId::Cw(CwScene::…)`; ver "Creative Writer: specs por programa".

As cenas leem tudo de `assets::Assets` (`image`, `sound`, `font`,
`palettes`, `dialogs` (registros `WHAT`), `animations`, `library_bytes`…),
sempre com um `Program` (`Artist` ou `Writer`). Sem os dados originais
(`ATELIE_DATA` ausente nos testes) os recursos faltam e cada componente usa a
sua reserva: textos em português embutidos, fonte 8×8, figura vazia.

## Módulos do `engine`

### Base

| Módulo | O quê |
|---|---|
| `assets` | Recursos originais (imagens `PIC`, sons, cursores, fontes, `WHAT`, bibliotecas `SHARED/STUFF`). |
| `frame` | `Frame` RGBA 640×480: `put`, `rect`, `frame_rect`, `blit`, `blit_masked`, `text_font`, `text` (8×8). |
| `anim` | Tocador dos roteiros `ATAG` (animações dos personagens e dos adesivos). |
| `rng` | xorshift32 determinístico (gags, fantasias do McZee). |
| `paths` | Busca sem maiúsculas (`find_ci`), varredura, nomes de arquivo seguros. |
| `users` | Cadastro da Entrada (nome, apelido, bicho). |
| `clipart` | Rolos de clip-art (`*.ART`). |

### Componentes compartilhados de interface — `ui`

| Item | Uso |
|---|---|
| `ui::Rect`, `ui::inside`, `ui::center` | Retângulo `(x, y, w, h)` e teste de clique (borda direita/de baixo de fora). Não declare outro `type Rect`/`fn inside`. |
| `ui::clip_text(s, width, max)` | Corta com "…" para caber em `max` px. |
| `ui::Pen` | Fonte de 8 pontos do programa (reserva 8×8): `width`, `text`, `centered`, `clip`. |
| `ui::DoubleClick<K>` | Clique duplo: `click(alvo, agora, pos)`; `with_slack(px)` também exige proximidade; `take`/`arm`/`reset` para quem separa o teste do registro (salas). Limiar único `DOUBLE_CLICK_S` (0,4 s). |
| `ui::SoundQueue` | Sons em sequência (o próximo quando o anterior acaba, pela duração do PCM); `play` substitui a sequência pendente, `pump` a cada `update`. Usada pela Entrada e pela Galeria. As salas tocam as vinhetas juntas e não a usam. |
| `ui::pal_rgb`, `ui::blit_canvas`, `ui::blit_canvas_fit` | Índice de paleta → RGB (preto se faltar); `Canvas` indexado na tela, inteiro ou esticado num retângulo. |
| `ui::checker` | Xadrez de pontos (tela "apagada" atrás de diálogos, botões desligados). |

### Balões

| Módulo | O quê |
|---|---|
| `balloon` | `Balloon`: caixa amarela pontilhada com texto, opções (bolinhas) e ligações; `from_block`/`from_record` lêem o `WHAT`. Também `draw_box`, `draw_box_white` (avisos), `draw_tail`, `wrap`, `balloon_font`. |
| `text_balloon` | `TextBalloon`: balões com `Part`s (texto, campos editáveis, grade de figuras, opções) e botões Volta / Para / Segue; `as_alert` (fundo branco), `handle` (teclado e clique). Usado na Entrada, nos avisos "EPA!", nas máquinas de impressão e nos assistentes. |
| `help`, `help_icons` | Balão de ajuda do McZee na pintura e os ícones embutidos no texto. |

(A renderização dos balões está sendo acabada em outro ramo; as telas só os
montam e desenham.)

### Pintura (núcleo, sem tela) — `paint`

| Módulo | O quê |
|---|---|
| `paint::canvas` | `Canvas` (índices da paleta de 256 cores), tinta, traço, `History`. |
| `paint::brush`, `geom`, `shape` | As 72 modalidades do pincel, formas geométricas, formas dos botões. |
| `paint::bucket_fx` | Efeitos da lata de tinta e papéis de parede. |
| `paint::objects`, `stamp` | Camada de objetos (adesivos e palavras soltos) e imagens coladas. |
| `paint::text`, `shaped`, `splot_res`, `ttf` | Palavras: fonte "Pablo", palavras com forma (Splot) com as formas `SCRD`, menus `TMEN`, cores `CSCH` e padrões `PATS` do executável — iguais no Writer —, fontes instaladas. |
| `paint::store` | Gravar/abrir PNG e `.MAG`, nomes com carimbo, último desenho, objetos. |
| `paint::drawings` | **Desenhos guardados como as telas os listam**: `list_drawings` (pasta da criança, sem os arquivos de trabalho), `list_samples` (`SHARED/USERS/*/*.MAG`), `mag_meta` (título e autor), miniaturas `shrink` (1/n) e `shrink_darkest` (1/n preservando linhas finas). |
| `paint::print` | Montagem das folhas impressas (esticar, pôster, fundo, branco e preto), `spawn_lp`, e as constantes da tela do McZee em `print::machine`. |

### Cenas — `scenes`

Telas do prédio e da Entrada:

| Módulo | O quê |
|---|---|
| `splash`, `chooser` | Abertura e escolha Fine Artist / Creative Writer. |
| `entry`, `entry_spec` | Entrada: McZee nas nuvens, quadro-negro com os usuários, cadastro em balões sucessivos (`WHAT 4400`), "EPA!" de sair (`Entry::quit_balloon_in`). O que muda por programa é um `EntrySpec` (`ARTIST_ENTRY`; o do Writer em `writer_entry`). |
| `street`, `imaginopolis`, `story` | Rua do museu, Imaginópolis, álbum da história do McZee. |
| `room`, `room_spec`, `rooms_data`, `room_objects`, `room_talk` | Salas do prédio: a mecânica (`room`), o contrato dos dados (`room_spec`: `RoomSpec` por sala, `RoomKit` por programa, `Action`), as salas do Fine Artist (`rooms_data`: `LOBBY_ROOM`…, `ARTIST_KIT`), objetos clicáveis (bolsa mágica, varinha, lâmpada, maleta), balões (`WHAT 100d`/`0bb8`), `Helper` (McZee/Maggie/Max, "Você quer me ver menos?"), `Pick`/`PickSpec` (balões de escolha), `quit_balloon_in(pos)`. |
| `elevator`, `attic`, `gags`, `studio_wall` | Elevador, sótão, animações ambientes, plaqueta do quadro da parede. |
| `writer_*` | Dados do Creative Writer para as cenas compartilhadas, um arquivo por tarefa (`writer_entry`, `writer_rooms`, `writer_lobby`, `writer_office_room`, `writer_floors`, `writer_attic`, `writer_gags`, `writer_street`, `writer_story`, `writer_elevator`, `writer_library`, `writer_portfolio`). Telas só do Writer: `writer_ideas` (Máquina de Idéias, Quadro de Figuras, Caderno) e `writer_easel` (Cavalete de lápis de cera). |
| `gallery_room` | Galeria do saguão: três molduras, maleta, lixeira, impressora, McZee. |
| `gallery` | Galeria antiga (grade de miniaturas); reexporta `list_drawings`/`list_samples`. |
| `portfolio` | Fichário verde (a pasta de trabalhos): árvore criança › Ateliê › quadros, busca pelo nome, miniatura, título/artista. |
| `browser` | Navegador de recursos (desenvolvimento). |

Tela de pintura e ferramentas:

| Módulo | O quê |
|---|---|
| `paint` (cena) | A tela do Ateliê; os submódulos `paint/*` são métodos de `Paint` (porta "Epa!" em `exit_dialog`, menu do McZee, lente, nome, lata, aspirador, máquina de impressão, Cachorrinho dos adesivos, cavalete de adesivos, Palavra e cavaletes). |
| `stickers`, `sticker_scripts` | Seleciona-adesivos: rolos, adesivos do usuário (`adesivo-*.png`), adesivos animados. |
| `sounds` (+ `finder`, `recorder`) | Seleciona-sons: categorias, Cachorrinho dos sons, Gravador. |
| `supertools`, `transformers` | Super-ferramentas e transformadores. |
| `wizard`, `tricks_basic`, `tricks_3d` | Assistente da Maggie e os roteiros dos Truques. |
| `projects` (`button`, `comics`, `flipbook`, `dialogs`) | Oficina de Projetos. |
| `paint_sfx` | Sons da pintura. |
| `writer` | Escritório do Creative Writer (início). Núcleo em `crate::writer`: modelo rico `doc`, `object`, `layout`, `selection`, `undo` (`docs/telas/creative-writer-modelo.md`; ainda não ligado à cena) e o provisório `legacy` + `store`, que a cena usa hoje. |

### Componentes compartilhados entre cenas

| Componente | Onde | Quem usa |
|---|---|---|
| Diálogo verde "achar arquivos" (Cachorrinho) | `scenes::finder_dialog`: geometria, `FileList` (rolagem, escolha, `hit` → `Hit`), `Entry`/`Kind` (`File`, `Outside`, `Back`), `draw_left`/`draw_right`. Cada ferramenta decide o que os botões fazem, os sons e a metade direita. | Cachorrinho dos sons (`sounds::finder`), Cachorrinho dos adesivos (`paint::find_pictures`). |
| Fichário verde (pasta) | `scenes::portfolio::Binder` | Galeria e maleta das salas (`GalleryRoom::portfolio`). |
| Impressão | `scenes::printing`: `PrintJob` (folhas numa thread, `lp`, tempo mínimo, cancelar), `step_balloon` (Pronto / Imprimindo / Terminei), `mczee_balloon`, `alert_balloon` + `CANT_PRINT`/`CANCELED`. | Prévia da Galeria (`gallery_print`), máquina do McZee (`paint::print_machine`). |
| "EPA! O que você quer fazer?" (Sair / Continuar) | `Entry::quit_balloon`, posicionado por `room_talk::quit_balloon(assets, pos)` | Entrada, salas, elevador, Galeria (esta com `alert = false`). |
| "Você quer me ver menos?" / "Que bom que você me chamou de novo" | `room_talk::Helper::balloon` | Salas, elevador, Galeria. |
| Sons em sequência | `ui::SoundQueue` | Entrada, Galeria. |
| Clique duplo | `ui::DoubleClick` | Salas (cavalete/parede, com folga de 6 px), fichário, Galeria, desenho animado. |
| Listas de desenhos e miniaturas | `paint::drawings` | Galeria, fichário, Cachorrinho dos adesivos, desenho animado. |

O "Epa!" da porta da pintura (`paint::exit_dialog`: Salvar / Jogar fora /
Ficar, `WHAT 0203`) é outro diálogo, com desenho próprio medido no original.

## Para o Creative Writer

O que já serve, e o que ainda está preso ao Fine Artist:

- **Entrada, salas**: já parametrizadas pelo programa (`EntrySpec`,
  `RoomSpec`/`RoomKit`); ver a seção seguinte. Os balões (`TextBalloon`,
  `Balloon`) ainda desenham as figuras embutidas do `ARTIST.PIC` (a arte
  "EPA!", os botões Volta/Para/Segue, as bolinhas): os ids e desenhos são
  os mesmos no `WRITER.PIC`, então a Entrada do Writer fica certa; quando
  os balões souberem o programa, use `EntrySpec::alert_art`.
- **Adesivos**: `stickers` + `paint::stamp`/`paint::objects` são a camada de
  objetos; a página do Writer pode usar os mesmos `Objects` sobre o texto.
- **Pasta**: `portfolio::Binder` monta a árvore a partir de
  `gallery_room::Item`; para documentos do Writer, generalize o item (nome,
  miniatura opcional) e a linha "Ateliê de Pintura".
- **Achar arquivos**: `finder_dialog` já é independente da ferramenta; para
  achar documentos basta outra lista de `Entry` e outro ícone de linha.
- **Impressão**: `printing::PrintJob` e os balões do McZee independem do que
  se imprime; a folha é montada por quem chama (`paint::print::Composer`
  para desenhos).
- **Sons**: `ui::SoundQueue` para vinhetas encadeadas; `scenes::sounds` é a
  ferramenta Seleciona-sons inteira (categorias, Cachorrinho, Gravador).
- **Texto de interface**: `ui::Pen` e `ui::clip_text`; para o texto do
  documento, `crate::writer::layout`.
- **Oficina de Idéias**: `crate::writer::ideas` (sem SDL) tem a Máquina de
  Idéias (`Grammar` do `WORKSHOP.CHK`, `Machine` com o sorteio do
  original), as figuras-rebus (`Assets::rebus`, `ideas::rebus::draw`), a
  gravação das entradas, figuras próprias e Caderno (`ideas::store`), a
  lista do Quadro (`board`), o texto editável (`notes`), o estado da
  sessão (`session`) e a ponte para o editor (`story`). As cenas são
  `scenes::writer_ideas` (Máquina, Quadro, Caderno, botões com estado
  apertado) e `scenes::writer_easel` (Cavalete de lápis de cera, que os
  adesivos do Escritório podem reaproveitar); ver
  `docs/telas/creative-writer-ideias.md`.

## Creative Writer: specs por programa

Princípio: **uma cena, dados por programa**. A Entrada, a história, a
Imaginópolis, a rua, as salas do prédio, o elevador, o sótão, a Galeria
(Biblioteca) e a pasta do Creative Writer são as mesmas cenas do Fine
Artist; o que muda (ids de figuras, sons e balões, posições, roteiros) é um
*spec*: uma `struct` só de dados, `Copy`, escrita como `const`. O do Fine
Artist reproduz o comportamento atual (os testes e as capturas do FA não
mudam); o do Writer vem de `cw/report/building*.md`.

### Navegação: `SceneId::Cw` e `in_program`

- As telas do Writer equivalentes às do FA são `SceneId::Cw(CwScene::…)`:
  `Entry`, `Story`, `BookStory`, `Imaginopolis`, `Street`,
  `Building(Floor)`, `Library`, `Portfolio`, `NewDocument`; só do Writer:
  `IdeaMachine`, `PictureBoard` (close-ups da Oficina de Idéias) e
  `IdeaStory(fonte)` (o editor com a história começada lá). O editor é
  `SceneId::Writer`.
- `SceneId::in_program(p)` leva um id ao programa `p` (`Street` →
  `Cw(Street)`, `Studio` → `Cw(Building(Floor::Studio))`, `Gallery` →
  `Cw(Library)`, `Paint` → `Writer`, `PaintNew` → `Cw(NewDocument)`, e de
  volta); `SceneId::program()` diz de que programa é a tela.
- Uma cena compartilhada guarda o `program` e **emite o id do Fine Artist
  passado por `in_program`**: `Effect::ChangeScene(SceneId::Street.in_program(self.program))`.
  Nunca escreva `SceneId::Cw(…)` à mão numa cena compartilhada.
- `app/src/run.rs` monta as duas variantes com um braço só:
  `SceneId::Street | SceneId::Cw(CwScene::Street) => Street::for_program(id.program(), …)`.
  Com `--program writer`, `--scene street` abre a rua do Writer (também
  `office`, `ideas`, `library`, `writer`, `writer-new`).
- `Floor` é o mesmo nos dois: `Lobby`, `Studio` (andar 2: Ateliê /
  Escritório), `Projects` (3), `Tricks` (4: Truques / Oficina de Ideias),
  `Attic`.

### O padrão de um spec

Para cada cena `X` (arquivo `x.rs`):

1. `pub struct XSpec { pub program: Program, … }` com tudo o que muda; os
   ids referem-se ao `.PIC`/`.SND`/`WHAT` de `program`.
2. `pub const ARTIST_X: XSpec` com os valores de hoje (as constantes que
   já existem no módulo; não duplique números).
3. `pub const WRITER_X: XSpec` em `writer_x.rs`.
4. `X::new(…)` continua sendo o Fine Artist (`X::for_program(Program::Artist, …)`);
   `X::for_program(p, …)` escolhe o spec (`match p { Artist => &ARTIST_X, Writer => &WRITER_X }`).
5. Testes: os do FA ficam como estão; os do Writer com
   `testutil::empty_assets()` (fluxo) e `testutil::assets()` (os ids
   existem no `WRITER.PIC`/`WHAT` com os tamanhos e opções esperados).

Os ganchos `for_program` já existem e já são chamados por `run.rs` (ou
pela sala) com o programa certo; hoje devolvem a cena do FA com o programa
guardado para a navegação. Por isso a próxima onda **não precisa mexer em
`scene.rs`, `run.rs`, `scenes/mod.rs` nem `room.rs`** para pôr dados:

| tarefa | arquivo do Writer (só seu) | cena que ganha o spec | gancho pronto |
|---|---|---|---|
| T1 Entrada (feita) | `writer_entry` | `entry` (`EntrySpec`, `entry_spec`) | `Entry::for_program` |
| T2 História | `writer_story` | `story` (`StorySpec`) | `Story::for_program` |
| T3 Imaginópolis e rua | `writer_street` | `imaginopolis`, `street` | `Imaginopolis::for_program`, `Street::for_program` |
| T5 Saguão | `writer_lobby` | — (só dados) | `writer_rooms::room(Floor::Lobby)` |
| T6 Escritório (sala) | `writer_office_room` | — (miniatura do documento: gancho em `room.rs`, ver abaixo) | `writer_rooms::room(Floor::Studio)` |
| T7 Projetos e Ideias | `writer_floors` (`PROJECTS`, `IDEAS`, `pick`) | — | `writer_rooms::room(…)`, `Pick::Newspaper`… |
| T8 Elevador | `writer_elevator` | `elevator` (`ElevatorSpec`) | `Elevator::for_program` |
| T9 Biblioteca | `writer_library` | `gallery_room` (`GallerySpec`) | `GalleryRoom::for_program` |
| T10 Sótão | `writer_attic` (`ATTIC`) | `attic` | `attic::draw_in`, `attic::dark_sound_in` |
| T11/T12 Gags, surpresas, fantasias | `writer_gags` (listas já transcritas do relatório, a conferir) | — | usadas pelos `RoomSpec` |
| T13 Bolsa e pasta | `writer_portfolio` | `portfolio`/`gallery_room` | `GalleryRoom::portfolio_for_program` |

O `writer_rooms` (o `WRITER_KIT` e o despacho por andar) é comum: só mude
um campo do kit que for da sua tarefa (por ex. T13: `wand`, `wand_cursor`).

### `EntrySpec` (exemplo: `writer_entry.rs`)

```rust
pub const WRITER_QUIT: QuitSpec = QuitSpec {
    program: Program::Writer,
    block: 0x0805,
    record: 12,
    w: 223,
    options: ["Sair do Creative Writer", "Continuar trabalhando"],
};

pub const WRITER_ENTRY: EntrySpec = EntrySpec {
    program: Program::Writer,
    background: 0x3c41,
    mczee: (0x4c43, 435, 112),
    what: 0x4e00,
    errors: 0x4e02,
    quit: &WRITER_QUIT,
    alert_art: 0x2cf,
    snd: EntrySounds { select: 0x22d, mczee: 0x281, browse: 0x281, choice: 0x2ff, /* … */ },
    direct: SceneId::Cw(CwScene::NewDocument), // opção 4
    direct_sounds: &[0x304],
    direct_label: "Ir direto para o Escritório",
    default_user: "escritor",
};
```

`Entry::finish` passa os destinos (`Imaginopolis`, `Story`) por
`in_program`; a opção 4 vai a `spec.direct`. O cadastro e a pasta de
usuários são os mesmos nos dois programas (um usuário criado num aparece no
outro, como no original).

### `RoomSpec` e `RoomKit` (salas do prédio)

`room_spec.rs` define o contrato; `room.rs` só tem a mecânica.

- `RoomKit` — o que é comum às salas de um programa: `program`, bloco
  `what` (`0x100d` / `0x0bb8`), `snd: RoomSounds` (bolinha, objeto, mastro,
  bolsa, quadro do saguão…), `tips` comuns, `glossary_links`,
  `hotkey_pic` (figura da "Tecla de atalho" que vira ligação), `help_off`/
  `help_on` (29/30 nos dois), `wand` (balão da varinha), `quit`
  (`&QuitSpec`), `bag` (`&BagPics`), `wand_cursor`, `elevator_helper`.
  Kits: `rooms_data::ARTIST_KIT`, `writer_rooms::WRITER_KIT`.
- `RoomSpec` — uma sala: `kit`, `background` (0 = preto), `elements`
  (figuras fixas), `hotspots` (área → `Action`, a primeira vale),
  `arrival` (registro, texto reserva, uma `Action` por opção, glossário),
  `character` (fantasias/poses, posição, `Helper`, som, posição do menu),
  `quit_pos`, `bag` (`BagLayout`) e `bag_open`, `surprises`, `gags`,
  `tips` (dicas da sala, procuradas antes das do kit), `painting`,
  `studio` (quadro do último trabalho e plaquetas), `elevator_sound`,
  `arrival_sound`, `elevator_arrival_sound`.
- `Action` — o que um clique faz: `Floor(f)` (quadro do saguão, inclusive
  o sótão), `Gallery(som)` (Galeria/Biblioteca), `Quit`, `PoleUp`/
  `PoleDown`, `MouseHole`, `Elevator`, `Outside`, `Easel`/`Wall`/
  `WallLabel`, `Pick(p)`, `Painting`, `Tip(k)`, `Sound(id)` (só um som:
  o mostrador do saguão do Writer), `AskButton`, `Close`.
- `Pick` — objetos com balão de escolha. Os do Writer já existem
  (`Newspaper`, `Card`, `Banner`, `IdeaMachine`, `PictureBoard`) e os dados
  deles (`PickSpec`: registro, posição, texto reserva, dica do "Ignorar",
  o que a primeira opção começa) estão em `writer_floors::pick`. Enquanto
  os projetos do Writer não existem, "começar" abre `Cw(NewDocument)`.
- `Helper::Max` — o Max (som `283`) nos andares 3 e 4 do Writer.
- A sala guarda o programa do prédio à parte do kit: um andar do Writer
  ainda sem dados (`writer_lobby::LOBBY = &rooms_data::LOBBY_ROOM`) mostra a
  sala do FA, mas as saídas vão às telas do Writer.

Exemplo de sala do Writer (esboço do saguão, `writer_lobby.rs`):

```rust
use super::room_spec::{Action, Arrival, CharacterSpec, Fallback, RoomSpec, Tip, hot};
use super::room_talk::Helper;
use super::writer_gags as g;
use super::writer_rooms::{WRITER_KIT, snd};
use super::room::Floor;

const LOBBY_ROOM: RoomSpec = RoomSpec {
    kit: &WRITER_KIT,
    background: 0x6805,
    elements: &[/* quadro 68b7, mastro 68df, abertura 68fb, luminária 6917, … */],
    hotspots: &[
        hot((25, 24, 159, 33), Action::Floor(Floor::Tricks)),   // Oficina de Idéias
        hot((30, 56, 153, 21), Action::Floor(Floor::Projects)),
        hot((34, 76, 149, 22), Action::Floor(Floor::Studio)),    // Escritório
        hot((10, 99, 36, 20), Action::Gallery(0x304)),           // setinha Biblioteca
        hot((41, 128, 53, 12), Action::Floor(Floor::Attic)),     // faixa de baixo
        hot((288, 21, 64, 30), Action::Sound(0x233)),            // mostrador (tamanho a medir)
        hot((94, 306, 120, 50), Action::Gallery(0x2fb)),         // seta Biblioteca (tamanho a medir)
        hot((409, 391, 81, 66), Action::Outside),                // skate LÁ FORA
        /* portas e botão do elevador, mastro, abertura, luminária, buraco */
    ],
    arrival: Some(Arrival {
        record: 1,
        fallback: Fallback { x: 129, y: 221, w: 278, text: "Oi, @! O que você quer fazer primeiro?", options: &[/* 4 */] },
        actions: &[Action::Tip(2), Action::Tip(3), Action::Tip(4), Action::Tip(5)],
        glossary: None,
    }),
    character: Some(CharacterSpec {
        choices: g::MCZEE_COSTUMES,
        pos: g::MCZEE_POS,
        helper: Helper::McZee,
        sound: snd::MCZEE,
        menu: (129, 221),
    }),
    tips: &[Tip { record: 2, pos: (34, 86), w: 167, text: "Clique no andar do Escritório…", links: &[] }, /* [3]–[5] */],
    gags: g::LOBBY_GAGS,
    surprises: g::LOBBY_SURPRISES,
    /* bag, quit_pos, elevator_sound (0x2d4), … */
};
pub const LOBBY: &RoomSpec = &LOBBY_ROOM;
```

Comportamento novo numa sala (por ex. a miniatura do documento no quadro
do Escritório, o Max sentado, o lampejo do sótão) não cabe em dados: ponha
um campo opcional no `RoomSpec` (ou uma variante de `Action`) e o gancho
mínimo em `room.rs`, com teste, e avise no relatório — `room.rs` é
compartilhado por T6, T7, T10 e T13.

### Elevador, rua, Imaginópolis, história, Galeria e sótão

Mesmo padrão (`XSpec` + `ARTIST_X` no módulo da cena + `WRITER_X` em
`writer_x.rs` + `X::for_program`). Esboço para a rua:

```rust
// street.rs
pub struct StreetSpec {
    pub program: Program,
    pub background: u16,
    pub door: (u16, Rect),        // roteiro da porta giratória e área
    pub ball: (u16, &'static [Rect]),
    pub hydrant: (u16, (i32, i32), Rect),
    pub trash: (u16, (i32, i32), Rect),
    pub lobby: &'static [Rect],   // fachada de baixo: direto ao saguão
    pub gags: &'static [u16],
    pub snd_facade: u16,
    pub snd_enter: u16,
}
pub const ARTIST_STREET: StreetSpec = StreetSpec { /* constantes de hoje */ };
impl Street {
    pub fn for_program(p: Program, click: Option<SoundRef>) -> Street {
        let spec = match p { Program::Artist => &ARTIST_STREET, Program::Writer => &super::writer_street::WRITER_STREET };
        /* … */
    }
}
```

- **Elevador** (`ElevatorSpec`): fundo, portas e escotilha, mostrador e
  números acesos, painel/seta e botões (áreas), luminária, posição do
  McZee e do balão, roteiros de subir/descer, sons, registro do "Subindo?"
  (`100d[64]` / `0bb8[63]`). `Elevator::for_program` já recebe o McZee da
  sala (`Character`, com o programa da figura).
- **Imaginópolis** (`ImaginopolisSpec`): fundo, roteiro do carrinho, sons
  de chegada e de entrada.
- **História** (`StorySpec`): bloco `WHAT`, álbum e máscara, falas e
  páginas, fotos e legendas por fala.
- **Galeria/Biblioteca** (`GallerySpec`): número de molduras e posições,
  impressora, lixeira, interruptor e gaveta de unidades, balões.
- **Sótão**: aro, som de chegada, sons do escuro e (Writer) regiões e
  objetos do lampejo; o aro e o som de chegada já são dados do
  `RoomSpec` do sótão (`writer_attic::ATTIC`).

### Conferir que o Fine Artist não mudou

- `ATELIE_SEED=<n>` fixa o sorteio (`XorShift::from_clock`): mesma fantasia
  do McZee, mesmas animações — capturas reproduzíveis.
- `room::tests::dump_fa_rooms_for_comparison` grava (com `ATELIE_DATA` e
  `ATELIE_DUMP=<pasta>`) 36 quadros das salas do FA em estados fixos;
  gere antes e depois da mudança e compare com `cmp`. Na criação dos
  specs das salas os 36 saíram idênticos.

### Pendências conhecidas

- As salas, a rua, a Imaginópolis, a história, o elevador, o sótão, a
  Biblioteca e a pasta do Writer ainda mostram os dados do FA (com a
  navegação do Writer) até as tarefas acima.
- `CwScene::NewDocument` abre o editor com o último documento até o
  Escritório ganhar um construtor de documento em branco (`run.rs`).
- O cursor padrão (`DEFAULT_CURSOR`) é a mão do `ARTIST.EXE`.

## Convenções

- Geometria medida no original: constantes com o nome da captura no
  comentário (`report/*.md` e `shots/` da varredura).
- Sons por id do `ARTIST.SND`, com nome em módulos `sfx`/`snd`.
- Componente novo compartilhado: teste unitário no próprio módulo; tela
  inteira: teste com `testutil::empty_assets()` (reserva) e, quando a
  fidelidade importa, com `testutil::assets()` (dados reais via `ATELIE_DATA`).
