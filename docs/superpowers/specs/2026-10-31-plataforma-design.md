# Ateliê Livre — Subprojeto 2a: camada de plataforma

Data: 2026-10-31
Status: aguardando revisão
Depende de: subprojeto 1 (`crates/formats`), concluído

## 1. Contexto

O subprojeto 2 (engine) foi dividido em:

- **2a — plataforma (este documento):** janela, ampliação da tela, entrada,
  cursores e som originais, laço principal, primeiras telas e um navegador de
  recursos. Não depende de entender a lógica dos programas.
- **2b — telas e comportamento do Fine Artist:** navegação, botões,
  ferramentas de pintura, animações. Decisões já tomadas: o Fine Artist vem
  antes do Creative Writer, e as telas serão **remontadas à mão** com a arte
  e os sons originais ("usável primeiro, refinado depois"), substituídas aos
  poucos pela interpretação dos recursos originais.

Objetivo do 2a: abrir o Ateliê Livre num Linux atual (CachyOS/KDE e Bluefin
do usuário; monitor de 1080p) e ver as telas do Fine Artist ampliadas e
nítidas, com cursor e sons originais.

## 2. Abordagem

A engine compõe cada quadro em software num buffer RGBA de 640×480,
independente da SDL; a SDL3 apenas amplia e apresenta esse buffer. Compor
640×480 é barato, a engine fica testável por comparação de quadros e a alta
resolução futura é um buffer maior. Alternativas descartadas: texturas de GPU
por imagem (prende a engine à SDL e dificulta testes, ganho de desempenho
desnecessário) e renderizador próprio com wgpu (exagero para 2D em 640×480).

## 3. Estrutura

```
crates/
├── formats/   (existente) leitura dos arquivos originais
├── engine/    (novo) lógica, sem SDL
└── app/       (novo) binário `atelie-livre`; único crate que conhece a SDL3
```

### 3.1 Crate `engine`

Depende de `formats` e de `font8x8` (MIT). Sem SDL, sem E/S de janela ou som.

- **`Assets`**: abre a pasta `MSKIDS` (busca sem distinguir maiúsculas,
  como no `atelie-tools`) e entrega, por fonte e id, imagens já convertidas em
  RGBA (com a paleta correta), sons
  como `Pcm`, e cursores (`Image` + hotspot). Decodifica sob demanda e guarda
  em cache. Fontes de recursos: `ARTIST.PIC`, `WRITER.PIC`, as bibliotecas
  `CHNK` de `SHARED/STUFF`, ícones e cursores de `ARTIST.EXE`/`WRITER.EXE`,
  sons de `ARTIST.SND`/`WRITER.SND`.
- **Regra de paleta compartilhada**: a escolha de paleta hoje implementada no
  `atelie-tools` (paleta embutida → `CTBL` do arquivo → `LOGPALETTE` do
  executável → rampa de cinzas) passa para `formats` (módulo de conversão
  `PKBM` → RGBA), usada pela engine e pelo `atelie-tools`, sem duplicação.
- **`Frame`**: buffer RGBA de tamanho parametrizável (640×480 no 2a), com
  `fill`, `blit(imagem, x, y)` com recorte nas bordas e `text(x, y, texto,
  cor)` com a fonte 8×8.
- **`Input`**: eventos já em coordenadas lógicas: movimento do mouse, clique
  (botão, x, y), tecla (código e modificadores).
- **`Scene`**: interface `update(&mut self, input: &[Input], dt) -> Vec<Effect>`
  e `draw(&self, frame: &mut Frame, assets: &Assets)`.
- **`Effect`**: pedidos da cena ao programa: `PlaySound(SoundRef)`,
  `SetCursor(CursorRef)`, `ChangeScene(SceneId)`, `Log(String)`, `Quit`.
  A engine nunca toca som nem mexe na janela diretamente.

### 3.2 Crate `app` (binário `atelie-livre`)

Depende de `engine` e de `sdl3` (com SDL3 compilada a partir do código-fonte
e ligada estaticamente). Responsável por: janela, apresentação lógica,
tradução de eventos, execução dos efeitos (som, cursor, troca de cena),
configuração e argumentos.

### 3.3 Laço principal

```
eventos SDL → coordenadas lógicas → cena.update → efeitos → app executa
                                     cena.draw(quadro) → textura → SDL amplia → tela
```

60 quadros por segundo, sincronizado com o monitor (vsync); `dt` em segundos
passado ao `update` para temporizações e animações.

## 4. Plataforma

- **Janela**: tela cheia sem bordas por padrão; Alt+Enter alterna com janela.
  Esc **não** fecha o programa (crianças apertam sem querer); sai-se fechando a
  janela ou com Ctrl+Q.
- **Ampliação** (apresentação lógica da SDL3):
  - `integer` (padrão): maior fator inteiro que cabe (2× em 1080p → 1280×960),
    vizinho mais próximo, faixas pretas.
  - `fit`: preenche mantendo 4:3 (2,25× em 1080p), filtro linear.
  - HiDPI tratado pela SDL.
- **Mouse**: a SDL converte as coordenadas para o espaço lógico; cliques
  fora da área 640×480 são ignorados.
- **Cursor**: cursores originais (32×32, máscara, hotspot) convertidos em
  cursores coloridos da SDL, ampliados pelo mesmo fator inteiro da tela
  (vizinho mais próximo), recriados quando a escala muda; cursor do sistema se
  o original faltar.
- **Som**: um fluxo de áudio da SDL3 por som em reprodução; a SDL mistura e
  reamostra (11.025 / 11.127 / 11.128 / 44.100 Hz → taxa do dispositivo). Até
  8 sons simultâneos (o mais antigo é interrompido); volume global
  configurável. Sem dispositivo de áudio, o programa segue mudo.
- **Configuração**: `~/.config/atelie-livre/config.toml` (respeitando
  `XDG_CONFIG_HOME`), com `data_dir`, `scale` (`integer`|`fit`), `fullscreen`,
  `volume` (0–100), `click_sound` (fonte e id, opcional) e `dev` (habilita F12).
  Argumentos têm prioridade: `atelie-livre [--data DIR] [--window]
  [--scale integer|fit] [--browser]`.
- **Pasta de dados ausente ou inválida**: caixa de mensagem da SDL explicando
  como indicar a pasta (o programa pode ter sido aberto por um ícone), além da
  mensagem no terminal; sai com código 1.
- **Compilação**: requer `cmake` e os cabeçalhos de desenvolvimento de
  Wayland, X11 e áudio (PipeWire/PulseAudio/ALSA) na máquina que compila. O
  binário resultante não exige a SDL3 instalada.

## 5. Cenas do 2a

Provisórias, para validar a plataforma; serão substituídas no 2b.

- **Abertura**: tela de título colorida do Fine Artist (`ARTIST.PIC` 0x6ca7).
  Clique ou 5 s → rua.
- **Rua**: cenário da rua de Imaginópolis (`ARTIST.PIC` 0x6d01), sem áreas
  clicáveis.
- **Clique**: em qualquer cena para crianças, um clique toca `click_sound` se
  configurado.
- **Navegador de recursos** (desenvolvimento): abre com `--browser`, ou F12 se
  `dev = true`.
  - ↑/↓ trocam a fonte (`ARTIST.PIC`, `WRITER.PIC`, cada biblioteca `CHNK`,
    ícones, cursores, sons de cada `.SND`); ←/→ percorrem os itens (com
    PageUp/PageDown saltando 10); Home/End vão ao primeiro/último.
  - Imagem centrada sobre fundo quadriculado; linha de informação no topo:
    fonte, posição na lista, id, largura×altura, profundidade.
  - Sobre a imagem, as coordenadas do mouse **na imagem** aparecem na linha
    de informação; um clique emite `Log("<fonte> <id> x=<x> y=<y>")`, que o app
    imprime no terminal — ferramenta para medir posições na remontagem do 2b.
  - Itens de som: Enter toca; a linha mostra taxa e duração.
  - Esc volta à cena anterior (exceção à regra do Esc, por ser ferramenta de
    desenvolvimento).

## 6. Erros

Nenhum recurso ausente ou corrompido derruba o programa: imagem ausente vira
um quadriculado magenta do tamanho pedido (ou 32×32), som ausente é ignorado,
ambos com aviso no log (stderr). Erros da SDL na inicialização do vídeo são
fatais (mensagem + código 1); falha ao abrir o áudio não é.

## 7. Testes e critérios de sucesso

Testes automatizados:

- `engine::Frame`: `blit` com recorte em todas as bordas e fora do quadro;
  `text` desenha os pixels esperados de um caractere.
- `engine::Assets` (com `ATELIE_DATA`): carrega 0x6ca7 e 0x6d01 com 640×480;
  cursor com hotspot dentro de 32×32; som 0x209 de `ARTIST.SND` decodifica;
  arquivos com nomes em minúsculas são encontrados; recurso inexistente
  devolve o placeholder, não erro fatal.
- Cenas, alimentadas com eventos sintéticos: clique na abertura →
  `ChangeScene(rua)`; 5 s de `dt` acumulado → `ChangeScene(rua)`; navegador:
  setas trocam fonte e item, clique sobre a imagem emite `Log` com as
  coordenadas certas (considerando a centralização), Enter num som emite
  `PlaySound`.
- `app`: conversão da configuração (TOML + argumentos, com prioridade dos
  argumentos); teste de fumaça com `SDL_VIDEODRIVER=dummy` e
  `SDL_AUDIODRIVER=dummy` executando 120 quadros com `--frames 120` (opção
  oculta) e saindo com código 0.
- `cargo test`, `cargo clippy -- -D warnings` e `cargo fmt --check` passam.

Critérios de sucesso (verificados pelo usuário no CachyOS):

1. `atelie-livre --data …/MSKIDS` abre em tela cheia mostrando a abertura,
   ampliada 2× e nítida, com o cursor original ampliado.
2. Clique leva à rua; Alt+Enter alterna janela/tela cheia sem perder a
   proporção; `--scale fit` preenche a altura.
3. No navegador, todas as imagens e sons são exibidos/tocados; os sons de
   11.127 Hz soam na altura certa; as coordenadas registradas batem com a
   posição do mouse.
4. Esc não fecha o programa nas cenas para crianças; Ctrl+Q fecha.

## 8. Fora do escopo

Áreas clicáveis, navegação real entre telas, ferramentas de pintura,
animações, McZee, salvar desenhos, fonte original (`PABLO.FON`), seletor
gráfico da pasta de dados, empacotamento (Flatpak/AppImage), alta resolução.
