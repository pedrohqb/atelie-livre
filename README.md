# Ateliê Livre

Reimplementação livre, em Rust, de dois programas infantis de desenho e escrita
dos anos 1990 (Microsoft Fine Artist e Microsoft Creative Writer), para que
crianças possam usá-los em sistemas atuais, com tela ampliada e som funcionando.

O Ateliê Livre não contém nenhum arquivo original: ele lê os dados de uma cópia
que você já possui.

## Estado

- Subprojeto 1 (leitura dos formatos de dados) concluído: todos os arquivos
  da versão Windows 3.1 em português são lidos, e as imagens e os sons são
  extraídos.
- Subprojeto 2a (plataforma) concluído: o programa `atelie-livre` abre a
  abertura e a rua do Fine Artist em tela cheia, ampliadas e nítidas, com o
  cursor e os sons originais, e traz um navegador de recursos. A
  documentação dos formatos está em [`docs/formats/`](docs/formats/).
- Subprojeto 2b (tela de pintura) concluído: na rua, clique na porta do
  prédio para pintar com o pincel (formas redondas, quadradas, carimbos e
  traços de caligrafia), a lata de tinta, as 38 cores e os padrões do
  original; o ovo desfaz e refaz, e o aspirador limpa o quadro.
- Subprojetos 2c e 2d (adesivos): os 29 rolos do original, 7 por página —
  o rolo de bichos animados (cobra, peixe-espada, tartaruga, pássaro...,
  com os movimentos gravados do original) e os 28 rolos de clip-art
  (passatempos, bichos, dinossauros, fadas, comida, zodíaco chinês...).
  Escolha o rolo, o adesivo na tira de filme e clique no quadro. Palavras,
  sons e as demais ferramentas ainda não funcionam.

Os próximos passos (engine, Fine Artist, Creative Writer) estão descritos em
[`docs/superpowers/specs/`](docs/superpowers/specs/).

## Rodar o Ateliê Livre

A SDL3 é compilada junto com o programa e embutida no binário; para isso, a
máquina que compila precisa do `cmake` e dos cabeçalhos de vídeo e áudio.

- CachyOS / Arch:
  `sudo pacman -S --needed base-devel cmake libx11 libxext libxrandr libxcursor libxi libxss libxtst libxkbcommon wayland wayland-protocols libdecor alsa-lib libpulse pipewire`
- Fedora (no Bluefin, dentro de um distrobox/toolbox Fedora):
  `sudo dnf install gcc cmake libX11-devel libXext-devel libXrandr-devel libXcursor-devel libXi-devel libXScrnSaver-devel libXtst-devel libxkbcommon-devel wayland-devel wayland-protocols-devel libdecor-devel alsa-lib-devel pulseaudio-libs-devel pipewire-devel`
- Debian / Ubuntu:
  `sudo apt install build-essential cmake libx11-dev libxext-dev libxrandr-dev libxcursor-dev libxi-dev libxss-dev libxtst-dev libxkbcommon-dev libwayland-dev wayland-protocols libdecor-0-dev libasound2-dev libpulse-dev libpipewire-0.3-dev`

```sh
cargo run --release -p app -- --data caminho/MSKIDS
```

A primeira compilação leva alguns minutos (a SDL3 é compilada uma vez). O
binário fica em `target/release/atelie-livre` e roda sem a SDL3 instalada.

Teclas: **Alt+Enter** alterna tela cheia/janela; **Ctrl+Q** sai (Esc não
fecha o programa, de propósito); **F12** abre o navegador de recursos se
`dev = true`. Opções: `--window`, `--scale integer|fit`, `--browser`.

No navegador: ↑/↓ trocam a fonte, ←/→ (PageUp/PageDown, Home/End) percorrem
os itens, Enter toca um som, Esc volta. Sobre uma imagem, a faixa mostra a
posição do mouse na imagem, e um clique imprime no terminal uma linha como
`ARTIST.PIC 6d01 x=312 y=405`.

Os desenhos são guardados ao sair da pintura (porta "Saída", Ctrl+Q ou
fechando a janela) em `~/.local/share/atelie-livre/desenhos/` (ou
`$XDG_DATA_HOME/atelie-livre/desenhos/`): `ultimo.png`, que é reaberto na
próxima vez, e uma cópia com data e hora de cada desenho modificado.

Configuração em `~/.config/atelie-livre/config.toml` (os argumentos têm
prioridade):

```toml
data_dir = "/home/voce/MSKIDS"
scale = "integer"      # ou "fit"
fullscreen = true
volume = 80            # 0 a 100
dev = false            # true habilita o F12
click_sound = { program = "artist", id = 0x209 }   # opcional
```

## Ferramentas de dados

É preciso ter o Rust (1.85 ou mais novo) e a pasta `MSKIDS` de uma instalação
do Fine Artist e/ou do Creative Writer, copiada para o seu computador.

```sh
cargo build --release

# inventário de um arquivo (executável, .PIC, documento, biblioteca, WAV)
target/release/atelie-tools info caminho/MSKIDS/ARTIST.EXE

# extrai imagens (PNG), sons (WAV), recursos e documentos para uma pasta
target/release/atelie-tools extract caminho/MSKIDS saida
```

O `extract` gera `saida/index.html`, uma folha de contato com todas as imagens
e os sons, e `saida/report.json`, com o que foi processado, o que falhou e o
que não foi encontrado. Os nomes dos arquivos podem estar em qualquer
combinação de maiúsculas e minúsculas, e os documentos são reconhecidos pelo
conteúdo, não pela extensão, em qualquer subpasta (inclusive `USERS/`). Códigos de saída: 0 sucesso, 1 houve
falhas, 2 a pasta não parece uma instalação compatível.

## Desenvolvimento

```sh
export ATELIE_DATA=caminho/MSKIDS   # habilita os testes com os dados reais
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Sem `ATELIE_DATA`, os testes com dados reais são pulados e só os testes com
dados sintéticos rodam. Os arquivos originais nunca entram no repositório.

## Licença

GPL-3.0-or-later (ver `LICENSE`).

Fine Artist, Creative Writer e demais nomes citados são marcas da Microsoft
Corporation; este projeto não tem vínculo com a Microsoft.
