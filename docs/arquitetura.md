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
- `SceneId` e a construção das cenas ficam em `app/src/run.rs`.

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
| `paint::text`, `shaped`, `ttf` | Palavras: fonte "Pablo", palavras com forma, fontes instaladas. |
| `paint::store` | Gravar/abrir PNG e `.MAG`, nomes com carimbo, último desenho, objetos. |
| `paint::drawings` | **Desenhos guardados como as telas os listam**: `list_drawings` (pasta da criança, sem os arquivos de trabalho), `list_samples` (`SHARED/USERS/*/*.MAG`), `mag_meta` (título e autor), miniaturas `shrink` (1/n) e `shrink_darkest` (1/n preservando linhas finas). |
| `paint::print` | Montagem das folhas impressas (esticar, pôster, fundo, branco e preto), `spawn_lp`, e as constantes da tela do McZee em `print::machine`. |

### Cenas — `scenes`

Telas do prédio e da Entrada:

| Módulo | O quê |
|---|---|
| `splash`, `chooser` | Abertura e escolha Fine Artist / Creative Writer. |
| `entry` | Entrada: McZee nas nuvens, quadro-negro com os usuários, cadastro em balões sucessivos (`WHAT 4400`), "EPA!" de sair (`Entry::quit_balloon`). |
| `street`, `imaginopolis`, `story` | Rua do museu, Imaginópolis, álbum da história do McZee. |
| `room`, `rooms_data`, `room_objects`, `room_talk` | Salas do prédio (saguão, Ateliê, Projetos, Truques): dados medidos, objetos clicáveis (bolsa mágica, varinha, lâmpada, maleta), balões (`WHAT 100d`), `Helper` (McZee/Maggie, "Você quer me ver menos?"), `quit_balloon(pos)`. |
| `elevator`, `attic`, `gags`, `studio_wall` | Elevador, sótão, animações ambientes, plaqueta do quadro da parede. |
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
| `writer` | Escritório do Creative Writer (início); núcleo em `crate::writer` (`doc`, `layout`, `store`). |

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

- **Entrada, salas, balões**: `Entry`, `room_talk` e os balões leem os
  registros com `Program::Artist` e têm textos de reserva com "Fine Artist"
  (por ex. `Entry::quit_balloon`). Para o Writer, parametrize pelo `Program`
  (o `WRITER.EXE` tem blocos `WHAT` próprios) em vez de copiar a cena.
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
  original), as figuras-rebus (`Assets::rebus`, `ideas::rebus::draw`) e a
  gravação das entradas, figuras próprias e Caderno (`ideas::store`). As
  cenas (sala, Máquina, Quadro, Caderno) ainda não existem.

## Convenções

- Geometria medida no original: constantes com o nome da captura no
  comentário (`report/*.md` e `shots/` da varredura).
- Sons por id do `ARTIST.SND`, com nome em módulos `sfx`/`snd`.
- Componente novo compartilhado: teste unitário no próprio módulo; tela
  inteira: teste com `testutil::empty_assets()` (reserva) e, quando a
  fidelidade importa, com `testutil::assets()` (dados reais via `ATELIE_DATA`).
