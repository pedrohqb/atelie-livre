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
  original; o ovo desfaz e refaz, e o aspirador aspira tinta (recorta), copia, cola,
  apaga áreas e cores e joga tudo fora.
- Subprojetos 2c e 2d (adesivos): os 29 rolos do original, 7 por página —
  o rolo de bichos animados (cobra, peixe-espada, tartaruga, pássaro...,
  com os movimentos gravados do original) e os 28 rolos de clip-art
  (passatempos, bichos, dinossauros, fadas, comida, zodíaco chinês...).
  Escolha o rolo, o adesivo na tira de filme e clique no quadro. Sons e as
  demais ferramentas ainda não funcionam.
- Palavras (primeira versão): com a ferramenta Palavra, clique no quadro e
  digite; Enter (ou um clique em outro lugar) grava o texto na cor atual, na
  fonte "Pablo Graphos" de 16 pontos, simples ou dentro de um balão de fala;
  o ovo desfaz o texto inteiro.
- Subprojeto 2e (prédio): a porta da rua leva ao saguão; o quadro de andares
  e o poste levam ao Ateliê de Pintura (clique no cavalete para pintar), à
  Oficina de Projetos e aos Truques de Desenho, ainda sem as atividades
  dessas salas, sem o McZee e a Maggie. O quadro na estante do Ateliê (e a
  Galeria do saguão) abre a galeria dos desenhos guardados; clique num deles
  para continuar pintando.

- Ao abrir, o Ateliê Livre pergunta qual programa usar (ou `--program
  artist|writer`, ou `program = "artist"|"writer"|"ask"` no config).
- Subprojeto 3a (Creative Writer, primeira versão): `atelie-livre --program
  writer` (ou `program = "writer"` no config) abre a tela de título do
  Creative Writer e vai direto ao Escritório: a página branca com a barra de
  ferramentas e a faixa de opções do original. Digite; Enter faz um novo
  parágrafo; as setas, Home, End, PageUp e PageDown (e as setas de "Pág."
  na barra de baixo) movem o cursor e as páginas; a faixa do Texto abre as
  listas de tamanho (de Gigante a Minúsculo), cor (as 38 da pintura),
  estilo (negrito, inclinado, sublinhado) e alinhamento. Os adesivos de
  clip-art entram no texto na posição do cursor, como um caractere grande.
  O ovo desfaz e refaz uma ação; a explosão da tesoura apaga tudo. Só há um
  tipo de letra (a "Pablo Graphos"); sons, inseto, mão, livro, página,
  imprimir e o prédio do Creative Writer ainda não existem.

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
`dev = true`. Opções: `--window`, `--scale integer|fit`, `--browser`,
`--program artist|writer`.

No navegador: ↑/↓ trocam a fonte, ←/→ (PageUp/PageDown, Home/End) percorrem
os itens, Enter toca um som, Esc volta. Sobre uma imagem, a faixa mostra a
posição do mouse na imagem, e um clique imprime no terminal uma linha como
`ARTIST.PIC 6d01 x=312 y=405`.

Os desenhos são guardados ao sair da pintura (porta "Saída", Ctrl+Q ou
fechando a janela) em `~/.local/share/atelie-livre/desenhos/` (ou
`$XDG_DATA_HOME/atelie-livre/desenhos/`): `ultimo.png`, que é reaberto na
próxima vez, e uma cópia com data e hora de cada desenho modificado.

Os textos do Creative Writer são guardados ao sair (porta "Saída", Ctrl+Q
ou fechando a janela) em `~/.local/share/atelie-livre/textos/` (ou
`$XDG_DATA_HOME/atelie-livre/textos/`): `ultimo.txt`, reaberto na próxima
vez, e uma cópia `texto-AAAAMMDD-HHMMSS.txt` de cada texto modificado. É
texto puro: parágrafos separados por uma linha vazia e adesivos como
`[adesivo:ROLO:N]`; os estilos ainda não são guardados.

Configuração em `~/.config/atelie-livre/config.toml` (os argumentos têm
prioridade):

```toml
data_dir = "/home/voce/MSKIDS"
scale = "integer"      # ou "fit"
fullscreen = true
volume = 80            # 0 a 100
dev = false            # true habilita o F12
program = "artist"     # ou "writer" (Creative Writer)
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

# converte um desenho .MAG do Fine Artist em PNG, e um PNG (594×347) em .MAG
target/release/atelie-tools mag2png caminho/MSKIDS/SHARED/USERS/MAGGIE/SAMP6.MAG praia.png
target/release/atelie-tools png2mag praia.png praia.mag --title "Praia" --data caminho/MSKIDS
```

O `png2mag` aproxima as cores à paleta do `ARTIST.EXE` da pasta indicada por
`--data` (ou pela variável `ATELIE_DATA`); sem a pasta, o PNG precisa ter até
256 cores. Os `.MAG` gravados seguem a estrutura dos desenhos de exemplo da
instalação, para abrir no Fine Artist original (ver `docs/formats/mag.md`).

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
